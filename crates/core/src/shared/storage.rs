//! Persistenza dei dati utente su SQLite, tramite sqlx.
//!
//! # Perche' l'API a runtime e non le macro
//!
//! Le query si scrivono con `sqlx::query` e `query_as` e i parametri si passano con
//! `bind`, non con le macro verificate in compilazione. Cosi' il repository appena
//! clonato compila senza avere un database a disposizione. Passare alle macro resta
//! un miglioramento possibile piu' avanti, non un debito che blocca.
//!
//! # Predisposizione al sync
//!
//! Non c'e' nessun sync e per ora non e' previsto, ma lo schema e' fatto perche'
//! aggiungerlo non richieda di rifarlo. Le chiavi sono valide su piu' dispositivi,
//! le righe che cambiano portano `updated_at` e `rev`, e c'e' posto per le lapidi
//! delle cancellazioni. I dettagli, tabella per tabella, stanno nei commenti della
//! migrazione.

use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::shared::error::{CoreError, Result};
use crate::shared::srs::{Grade, MemoryState, Scheduled};

/// Lo stato di studio di un elemento per un tipo di esercizio.
///
/// Lo stesso kana studiato in riconoscimento e in input sono due carte distinte.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct Card {
    pub item_id: String,
    pub exercise_type: String,
    /// Quando va ripresentata. `None` significa mai studiata, quindi dovuta subito.
    pub due_at: Option<DateTime<Utc>>,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub reps: i64,
    pub lapses: i64,
    /// Per quanti giorni il ricordo regge, secondo FSRS. `None` se mai studiata.
    pub stability: Option<f32>,
    /// Quanto l'elemento e' faticoso per questa persona. `None` se mai studiata.
    pub difficulty: Option<f32>,
    pub updated_at: DateTime<Utc>,
    /// Numero di revisione della riga, cresce a ogni modifica.
    pub rev: i64,
}

impl Card {
    /// Lo stato di memoria da ridare allo scheduler al prossimo ripasso.
    ///
    /// I due numeri hanno senso solo insieme: una carta mai studiata non ne ha
    /// nessuno.
    pub fn memory(&self) -> Option<MemoryState> {
        match (self.stability, self.difficulty) {
            (Some(stability), Some(difficulty)) => Some(MemoryState {
                stability,
                difficulty,
            }),
            _ => None,
        }
    }
}

/// Una risposta gia' data, come e' stata registrata.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AnswerLog {
    pub id: String,
    pub item_id: String,
    pub exercise_type: String,
    pub answered_at: DateTime<Utc>,
    pub correct: bool,
    pub answer: String,
    /// La valutazione nella scala di FSRS, da 1 a 4.
    ///
    /// Resta un numero grezzo invece di un [`Grade`] perche' e' un dato che arriva
    /// dal database e potrebbe essere stato scritto da una versione futura con piu'
    /// gradini: [`AnswerLog::grade`] lo interpreta quando serve.
    pub rating: Option<i64>,
}

impl AnswerLog {
    pub fn grade(&self) -> Option<Grade> {
        self.rating.and_then(Grade::from_i64)
    }
}

/// Cosa registrare quando l'utente risponde.
#[derive(Debug, Clone, Copy)]
pub struct NewAnswer<'a> {
    pub item_id: &'a str,
    pub exercise_type: &'a str,
    pub correct: bool,
    /// Il testo prodotto dall'utente, tenuto per poter rivedere l'errore invece di
    /// sapere soltanto che c'e' stato.
    pub answer: &'a str,
    pub answered_at: DateTime<Utc>,
    /// La valutazione data alla risposta.
    pub grade: Grade,
    /// Il nuovo stato della carta. Lo decide lo scheduler, non l'archivio: qui arriva
    /// gia' calcolato.
    pub next: Scheduled,
}

/// Il database locale dell'utente.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Apre il database su file, creandolo se non esiste, e applica le migrazioni
    /// mancanti.
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir).map_err(|e| CoreError::Storage {
                message: format!("cartella dati non creata: {e}"),
            })?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            // Con WAL le letture non aspettano la scrittura in corso: su un'app che
            // registra una risposta mentre gia' prepara la domanda dopo, si sente.
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        Self::connect(SqlitePoolOptions::new(), options).await
    }

    /// Database temporaneo in memoria, per i test.
    ///
    /// La pool e' limitata a una sola connessione: in SQLite ogni connessione in
    /// memoria vedrebbe un database tutto suo, e i test lavorerebbero su copie
    /// diverse senza accorgersene.
    pub async fn in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new().in_memory(true);
        Self::connect(SqlitePoolOptions::new().max_connections(1), options).await
    }

    async fn connect(pool: SqlitePoolOptions, options: SqliteConnectOptions) -> Result<Self> {
        let pool = pool.connect_with(options).await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    /// Porta lo schema all'ultima versione. E' sicuro chiamarla a ogni avvio: sqlx
    /// tiene traccia di cosa ha gia' applicato.
    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Lo stato di studio di una carta, se esiste.
    pub async fn card(&self, item_id: &str, exercise_type: &str) -> Result<Option<Card>> {
        let card = sqlx::query_as::<_, Card>(
            "SELECT item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                    stability, difficulty, updated_at, rev
             FROM cards
             WHERE item_id = ? AND exercise_type = ? AND deleted_at IS NULL",
        )
        .bind(item_id)
        .bind(exercise_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(card)
    }

    /// Mette una carta tra quelle da studiare, senza ancora studiarla.
    ///
    /// Nasce con `due_at` a NULL, cioe' dovuta subito e mai vista. Se la carta esiste
    /// gia' non viene toccata: reintrodurre un elemento non deve azzerarne i
    /// progressi.
    pub async fn ensure_card(
        &self,
        item_id: &str,
        exercise_type: &str,
        now: DateTime<Utc>,
    ) -> Result<Card> {
        sqlx::query(
            "INSERT INTO cards (
                 item_id, exercise_type, due_at, reps, lapses, created_at, updated_at, rev
             )
             VALUES (?, ?, NULL, 0, 0, ?, ?, 1)
             ON CONFLICT (item_id, exercise_type) DO NOTHING",
        )
        .bind(item_id)
        .bind(exercise_type)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // `DO NOTHING` non restituisce righe quando la carta c'era gia', quindi la si
        // rilegge invece di usare `RETURNING`.
        self.card(item_id, exercise_type)
            .await?
            .ok_or_else(|| CoreError::Storage {
                message: format!(
                    "carta {item_id} / {exercise_type} non trovata dopo l'inserimento"
                ),
            })
    }

    /// Le carte da studiare adesso, le piu' arretrate per prime.
    ///
    /// Le carte mai studiate hanno `due_at` a NULL e SQLite le ordina prima di tutte
    /// le altre. Se e quanto mescolare le nuove tra le arretrate e' una decisione
    /// della sessione, non dell'archivio.
    pub async fn due_cards(&self, now: DateTime<Utc>, limit: i64) -> Result<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            "SELECT item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                    stability, difficulty, updated_at, rev
             FROM cards
             WHERE deleted_at IS NULL AND (due_at IS NULL OR due_at <= ?)
             ORDER BY due_at ASC
             LIMIT ?",
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    /// Registra una risposta e aggiorna la carta corrispondente.
    ///
    /// Le due scritture stanno nella stessa transazione: una risposta senza il suo
    /// effetto sulla carta, o il contrario, lascerebbe lo storico e la pianificazione
    /// a raccontare due storie diverse.
    ///
    /// La carta viene creata se e' la prima volta che quell'elemento viene studiato.
    pub async fn record_answer(&self, answer: NewAnswer<'_>) -> Result<Card> {
        let mut tx = self.pool.begin().await?;

        // UUID versione 7: contiene l'istante di creazione, quindi e' unico tra
        // dispositivi e ordinabile nel tempo senza colonne di appoggio.
        sqlx::query(
            "INSERT INTO answers (
                 id, item_id, exercise_type, answered_at, correct, answer, rating
             )
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(answer.item_id)
        .bind(answer.exercise_type)
        .bind(answer.answered_at)
        .bind(answer.correct)
        .bind(answer.answer)
        .bind(answer.grade.as_i64())
        .execute(&mut *tx)
        .await?;

        // Alla prima risposta la riga nasce con reps a 1; dalla seconda in poi
        // `excluded` porta i valori che avremmo inserito e si sommano a quelli gia'
        // presenti.
        let lapse = i64::from(!answer.correct);
        let card = sqlx::query_as::<_, Card>(
            "INSERT INTO cards (
                 item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                 stability, difficulty, created_at, updated_at, rev
             )
             VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, ?, 1)
             ON CONFLICT (item_id, exercise_type) DO UPDATE SET
                 due_at           = excluded.due_at,
                 last_reviewed_at = excluded.last_reviewed_at,
                 reps             = cards.reps + 1,
                 lapses           = cards.lapses + excluded.lapses,
                 stability        = excluded.stability,
                 difficulty       = excluded.difficulty,
                 updated_at       = excluded.updated_at,
                 rev              = cards.rev + 1,
                 deleted_at       = NULL
             RETURNING item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                       stability, difficulty, updated_at, rev",
        )
        .bind(answer.item_id)
        .bind(answer.exercise_type)
        .bind(answer.next.due_at)
        .bind(answer.answered_at)
        .bind(lapse)
        .bind(answer.next.memory.stability)
        .bind(answer.next.memory.difficulty)
        .bind(answer.answered_at)
        .bind(answer.answered_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(card)
    }

    /// Lo storico delle risposte su una carta, dalla piu' vecchia.
    pub async fn answers(&self, item_id: &str, exercise_type: &str) -> Result<Vec<AnswerLog>> {
        let answers = sqlx::query_as::<_, AnswerLog>(
            "SELECT id, item_id, exercise_type, answered_at, correct, answer, rating
             FROM answers
             WHERE item_id = ? AND exercise_type = ?
             ORDER BY answered_at ASC, id ASC",
        )
        .bind(item_id)
        .bind(exercise_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(answers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::srs::Scheduler;
    use chrono::TimeDelta;

    const ITEM: &str = "kana:hiragana:か";
    const EXERCISE: &str = "kana.input";

    fn pianificata(due_at: DateTime<Utc>) -> Scheduled {
        Scheduled {
            memory: MemoryState {
                stability: 3.0,
                difficulty: 5.0,
            },
            due_at,
            interval_days: 1.0,
        }
    }

    fn risposta(correct: bool, answered_at: DateTime<Utc>) -> NewAnswer<'static> {
        NewAnswer {
            item_id: ITEM,
            exercise_type: EXERCISE,
            correct,
            answer: if correct { "か" } else { "き" },
            answered_at,
            grade: Grade::from_correct(correct),
            next: pianificata(answered_at + TimeDelta::days(1)),
        }
    }

    #[tokio::test]
    async fn un_database_nuovo_e_vuoto_ma_gia_migrato() {
        let db = Database::in_memory().await.unwrap();
        assert_eq!(db.card(ITEM, EXERCISE).await.unwrap(), None);
        assert!(db.due_cards(Utc::now(), 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn la_prima_risposta_crea_la_carta() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        let card = db.record_answer(risposta(true, now)).await.unwrap();

        assert_eq!(card.item_id, ITEM);
        assert_eq!(card.reps, 1);
        assert_eq!(card.lapses, 0);
        assert_eq!(card.rev, 1);
        assert_eq!(card.last_reviewed_at, Some(now));
        assert_eq!(card.due_at, Some(now + TimeDelta::days(1)));
        assert_eq!(
            card.memory(),
            Some(MemoryState {
                stability: 3.0,
                difficulty: 5.0
            })
        );
        assert_eq!(db.card(ITEM, EXERCISE).await.unwrap(), Some(card));
    }

    #[tokio::test]
    async fn le_risposte_successive_accumulano_sulla_stessa_carta() {
        let db = Database::in_memory().await.unwrap();
        let t0 = Utc::now();

        db.record_answer(risposta(true, t0)).await.unwrap();
        db.record_answer(risposta(false, t0 + TimeDelta::minutes(1)))
            .await
            .unwrap();
        let card = db
            .record_answer(risposta(true, t0 + TimeDelta::minutes(2)))
            .await
            .unwrap();

        assert_eq!(card.reps, 3);
        // Un solo errore su tre risposte.
        assert_eq!(card.lapses, 1);
        // La revisione cresce a ogni scrittura: e' cio' su cui si appoggera' il sync.
        assert_eq!(card.rev, 3);
        assert_eq!(card.updated_at, t0 + TimeDelta::minutes(2));
    }

    #[tokio::test]
    async fn lo_storico_tiene_tutte_le_risposte_in_ordine() {
        let db = Database::in_memory().await.unwrap();
        let t0 = Utc::now();

        db.record_answer(risposta(true, t0)).await.unwrap();
        db.record_answer(risposta(false, t0 + TimeDelta::minutes(1)))
            .await
            .unwrap();

        let log = db.answers(ITEM, EXERCISE).await.unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].correct);
        assert_eq!(log[0].grade(), Some(Grade::Good));
        assert!(!log[1].correct);
        assert_eq!(log[1].grade(), Some(Grade::Again));
        assert_eq!(log[1].answer, "き");
        // Gli identificatori sono unici e crescono nel tempo, essendo UUID v7.
        assert_ne!(log[0].id, log[1].id);
        assert!(log[0].id < log[1].id);
    }

    #[tokio::test]
    async fn una_carta_introdotta_e_dovuta_e_non_ha_ancora_memoria() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        let card = db.ensure_card(ITEM, EXERCISE, now).await.unwrap();
        assert_eq!(card.due_at, None);
        assert_eq!(card.reps, 0);
        assert_eq!(card.memory(), None);

        assert_eq!(db.due_cards(now, 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn reintrodurre_una_carta_non_ne_azzera_i_progressi() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        db.record_answer(risposta(true, now)).await.unwrap();
        let card = db.ensure_card(ITEM, EXERCISE, now).await.unwrap();

        assert_eq!(card.reps, 1);
        assert_eq!(card.rev, 1);
    }

    #[tokio::test]
    async fn sono_dovute_le_carte_mai_studiate_e_quelle_scadute() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        // Scaduta ieri.
        db.record_answer(NewAnswer {
            item_id: "kana:hiragana:あ",
            next: pianificata(now - TimeDelta::days(1)),
            ..risposta(true, now - TimeDelta::days(2))
        })
        .await
        .unwrap();

        // Da rivedere domani.
        db.record_answer(NewAnswer {
            item_id: "kana:hiragana:い",
            next: pianificata(now + TimeDelta::days(1)),
            ..risposta(true, now)
        })
        .await
        .unwrap();

        // Introdotta e mai studiata.
        db.ensure_card("kana:hiragana:う", EXERCISE, now)
            .await
            .unwrap();

        let dovute = db.due_cards(now, 10).await.unwrap();
        let ids: Vec<&str> = dovute.iter().map(|c| c.item_id.as_str()).collect();

        // La mai studiata viene prima, poi la scaduta. Quella futura resta fuori.
        assert_eq!(ids, ["kana:hiragana:う", "kana:hiragana:あ"]);
    }

    #[tokio::test]
    async fn il_limite_sulle_carte_dovute_viene_rispettato() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        for c in ["あ", "い", "う"] {
            db.ensure_card(c, EXERCISE, now).await.unwrap();
        }

        assert_eq!(db.due_cards(now, 2).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn i_dati_sopravvivono_alla_chiusura_e_le_migrazioni_si_possono_rieseguire() {
        let dir = tempfile::tempdir().unwrap();
        // La sottocartella non esiste: `open` deve crearla.
        let path = dir.path().join("dati").join("tanren.db");
        let now = Utc::now();

        {
            let db = Database::open(&path).await.unwrap();
            db.record_answer(risposta(true, now)).await.unwrap();
        }

        // Riaprire applica di nuovo le migrazioni, che devono accorgersi di essere
        // gia' state eseguite invece di fallire.
        let db = Database::open(&path).await.unwrap();
        let card = db.card(ITEM, EXERCISE).await.unwrap().unwrap();
        assert_eq!(card.reps, 1);
    }

    #[tokio::test]
    async fn il_giro_completo_pianifica_salva_e_ripianifica() {
        // E' il giro che fara' l'app a ogni risposta: leggi lo stato, pianifica,
        // salva, e alla volta dopo riparti da quello che hai salvato.
        let db = Database::in_memory().await.unwrap();
        let scheduler = Scheduler::default();
        let t0 = Utc::now();

        let card = db.ensure_card(ITEM, EXERCISE, t0).await.unwrap();
        assert_eq!(card.memory(), None);

        let primo = scheduler
            .schedule(card.memory(), card.last_reviewed_at, Grade::Good, t0)
            .unwrap();
        let card = db
            .record_answer(NewAnswer {
                item_id: ITEM,
                exercise_type: EXERCISE,
                correct: true,
                answer: "か",
                answered_at: t0,
                grade: Grade::Good,
                next: primo,
            })
            .await
            .unwrap();

        // Lo stato di memoria e' stato salvato e riletto.
        let memoria = card.memory().expect("la carta ha ora uno stato");
        assert_eq!(memoria, primo.memory);
        assert!(memoria.stability > 0.0);

        // Alla ripetizione successiva si riparte da li', e l'intervallo si allunga.
        let t1 = card.due_at.unwrap();
        let secondo = scheduler
            .schedule(card.memory(), card.last_reviewed_at, Grade::Good, t1)
            .unwrap();

        assert!(
            secondo.interval_days > primo.interval_days,
            "una seconda risposta giusta deve allontanare il ripasso: {} contro {}",
            secondo.interval_days,
            primo.interval_days
        );
        assert!(secondo.memory.stability > primo.memory.stability);
    }
}
