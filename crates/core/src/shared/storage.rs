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
use sqlx::{FromRow, QueryBuilder, SqlitePool};
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
    /// Quanto ci e' voluto a rispondere, in millisecondi. `None` se non misurato.
    ///
    /// Si raccoglie sempre e non lo legge ancora nessuno: e' un dato che si puo' solo
    /// misurare mentre accade. **Non e' un voto**, e la sezione 3 di CLAUDE.md dice
    /// perche' non deve diventarlo.
    pub response_time_ms: Option<i64>,
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
    /// Da quando la domanda e' comparsa a quando l'utente ha risposto, in
    /// millisecondi. `None` quando chi chiama non l'ha misurato.
    ///
    /// Va riempito ovunque si possa: e' l'unico dato di questa tabella che non si puo'
    /// ricostruire dopo. Serve al dataset, non alla pianificazione, che non deve
    /// guardarlo.
    pub response_time_ms: Option<i64>,
    /// Come aggiornare la carta, per le materie che usano la ripetizione spaziata.
    ///
    /// `None` per quelle che non la usano: la risposta entra comunque nello storico,
    /// `rating` resta NULL perche' nessuno ha dato un giudizio a quattro gradini, e la
    /// carta non viene toccata. Vale oggi per i kana, vedi la nota su FSRS in
    /// CLAUDE.md.
    pub scheduling: Option<Scheduling>,
}

/// Come lo scheduler vuole aggiornare la carta dopo una risposta.
///
/// Lo decide lo scheduler, non l'archivio: qui arriva gia' calcolato.
#[derive(Debug, Clone, Copy)]
pub struct Scheduling {
    /// La valutazione data alla risposta.
    pub grade: Grade,
    pub next: Scheduled,
}

/// Come restringere la ricerca delle carte dovute.
///
/// Un campo a `None` non filtra.
#[derive(Debug, Clone, Copy, Default)]
pub struct CardFilter<'a> {
    /// Gli elementi che fanno parte dell'ambito scelto.
    pub items: Option<&'a [String]>,
    pub exercise_type: Option<&'a str>,
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

    /// Introduce in blocco un elenco di carte, in una sola transazione.
    ///
    /// E' quello che serve all'inizio di una sessione: l'ambito scelto puo' contenere
    /// centinaia di elementi, e farne una scrittura per uno sarebbe altrettanto
    /// corretto ma inutilmente lento.
    pub async fn ensure_cards(
        &self,
        items: &[String],
        exercise_type: &str,
        now: DateTime<Utc>,
    ) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let mut nuove = 0;

        for item_id in items {
            let esito = sqlx::query(
                "INSERT INTO cards (
                     item_id, exercise_type, due_at, reps, lapses, created_at, updated_at, rev
                 )
                 VALUES (?, ?, NULL, 0, 0, ?, ?, 1)
                 ON CONFLICT (item_id, exercise_type) DO NOTHING",
            )
            .bind(item_id.as_str())
            .bind(exercise_type)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            nuove += esito.rows_affected();
        }

        tx.commit().await?;
        Ok(nuove)
    }

    /// Tutte le carte che rientrano nel filtro, studiate o no.
    ///
    /// Serve a misurare a che punto e' un insieme di item: quante sono gia' nate,
    /// quante hanno raggiunto una certa stabilita', quanto reggono adesso. E' una
    /// domanda diversa da [`Self::due_cards`], che chiede solo cosa tocca fare.
    pub async fn cards(&self, filter: CardFilter<'_>) -> Result<Vec<Card>> {
        let mut query = QueryBuilder::new(
            "SELECT item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                    stability, difficulty, updated_at, rev
             FROM cards
             WHERE deleted_at IS NULL",
        );

        if let Some(exercise) = filter.exercise_type {
            query.push(" AND exercise_type = ").push_bind(exercise);
        }

        if let Some(items) = filter.items {
            if items.is_empty() {
                return Ok(Vec::new());
            }
            query.push(" AND item_id IN (");
            let mut elenco = query.separated(", ");
            for item in items {
                elenco.push_bind(item.as_str());
            }
            query.push(")");
        }

        Ok(query.build_query_as::<Card>().fetch_all(&self.pool).await?)
    }

    /// Quante carte di un tipo sono nate dopo un certo istante.
    ///
    /// Una carta nasce quando l'elemento viene introdotto, quindi contarle e' il modo
    /// di sapere quanto si e' introdotto oggi. Il tipo di esercizio serve a contare
    /// **una volta sola per elemento** quando un elemento ne genera piu' d'una.
    pub async fn cards_created_since(
        &self,
        exercise_type: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM cards
             WHERE deleted_at IS NULL AND exercise_type = ? AND created_at >= ?",
        )
        .bind(exercise_type)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Quando e' nata l'ultima carta di un tipo. `None` se non ce n'e' nessuna.
    pub async fn last_card_created(&self, exercise_type: &str) -> Result<Option<DateTime<Utc>>> {
        let (last,): (Option<DateTime<Utc>>,) = sqlx::query_as(
            "SELECT max(created_at) FROM cards
             WHERE deleted_at IS NULL AND exercise_type = ?",
        )
        .bind(exercise_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(last)
    }

    /// Una preferenza, se e' stata scritta.
    ///
    /// Torna `None` sia quando la chiave non c'e' sia quando non e' mai stata toccata:
    /// per chi legge sono la stessa cosa, cioe' «vale il default».
    pub async fn setting(&self, key: &str) -> Result<Option<String>> {
        let riga: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(riga.map(|(v,)| v))
    }

    /// Scrive una preferenza, sovrascrivendo quella di prima.
    pub async fn set_setting(&self, key: &str, value: &str, now: DateTime<Utc>) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Le carte da studiare adesso, le piu' arretrate per prime.
    ///
    /// Le carte mai studiate hanno `due_at` a NULL e SQLite le ordina prima di tutte
    /// le altre. Se e quanto mescolare le nuove tra le arretrate e' una decisione
    /// della sessione, non dell'archivio.
    ///
    /// Il filtro serve a restare dentro l'ambito scelto: senza, il limite taglierebbe
    /// prima di escludere le carte fuori ambito e la sessione si ritroverebbe con meno
    /// carte di quelle che ha chiesto.
    pub async fn due_cards(
        &self,
        filter: CardFilter<'_>,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<Card>> {
        // La query si costruisce pezzo per pezzo perche' il numero di elementi da
        // filtrare non e' noto in anticipo. `QueryBuilder` e' lo strumento che sqlx
        // offre per farlo: i valori entrano sempre come parametri, mai concatenati
        // nella stringa.
        let mut query = QueryBuilder::new(
            "SELECT item_id, exercise_type, due_at, last_reviewed_at, reps, lapses,
                    stability, difficulty, updated_at, rev
             FROM cards
             WHERE deleted_at IS NULL AND (due_at IS NULL OR due_at <= ",
        );
        query.push_bind(now).push(")");

        if let Some(exercise) = filter.exercise_type {
            query.push(" AND exercise_type = ").push_bind(exercise);
        }

        if let Some(items) = filter.items {
            // Un ambito vuoto non e' un ambito che comprende tutto: e' un ambito senza
            // niente dentro.
            if items.is_empty() {
                return Ok(Vec::new());
            }
            query.push(" AND item_id IN (");
            let mut elenco = query.separated(", ");
            for item in items {
                elenco.push_bind(item.as_str());
            }
            query.push(")");
        }

        query.push(" ORDER BY due_at ASC LIMIT ").push_bind(limit);

        Ok(query.build_query_as::<Card>().fetch_all(&self.pool).await?)
    }

    /// Quante carte sono dovute nell'ambito indicato.
    ///
    /// Serve a dire all'utente quanto gli resta senza caricare le carte stesse.
    pub async fn count_due(&self, filter: CardFilter<'_>, now: DateTime<Utc>) -> Result<i64> {
        let mut query = QueryBuilder::new(
            "SELECT COUNT(*) FROM cards
             WHERE deleted_at IS NULL AND (due_at IS NULL OR due_at <= ",
        );
        query.push_bind(now).push(")");

        if let Some(exercise) = filter.exercise_type {
            query.push(" AND exercise_type = ").push_bind(exercise);
        }
        if let Some(items) = filter.items {
            if items.is_empty() {
                return Ok(0);
            }
            query.push(" AND item_id IN (");
            let mut elenco = query.separated(", ");
            for item in items {
                elenco.push_bind(item.as_str());
            }
            query.push(")");
        }

        let (count,): (i64,) = query.build_query_as().fetch_one(&self.pool).await?;
        Ok(count)
    }

    /// Registra una risposta e aggiorna la carta corrispondente.
    ///
    /// Le due scritture stanno nella stessa transazione: una risposta senza il suo
    /// effetto sulla carta, o il contrario, lascerebbe lo storico e la pianificazione
    /// a raccontare due storie diverse.
    ///
    /// La carta viene creata se e' la prima volta che quell'elemento viene studiato.
    pub async fn record_answer(&self, answer: NewAnswer<'_>) -> Result<Option<Card>> {
        let mut tx = self.pool.begin().await?;

        // UUID versione 7: contiene l'istante di creazione, quindi e' unico tra
        // dispositivi e ordinabile nel tempo senza colonne di appoggio.
        sqlx::query(
            "INSERT INTO answers (
                 id, item_id, exercise_type, answered_at, correct, answer, rating,
                 response_time_ms
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(answer.item_id)
        .bind(answer.exercise_type)
        .bind(answer.answered_at)
        .bind(answer.correct)
        .bind(answer.answer)
        .bind(answer.scheduling.map(|s| s.grade.as_i64()))
        .bind(answer.response_time_ms)
        .execute(&mut *tx)
        .await?;

        // Senza pianificazione non c'e' niente da aggiornare: la carta esiste per
        // tenere lo stato della ripetizione spaziata, e chi non la usa non ne ha una.
        let card = match answer.scheduling {
            None => None,
            Some(scheduling) => {
                // Alla prima risposta la riga nasce con reps a 1; dalla seconda in poi
                // `excluded` porta i valori che avremmo inserito e si sommano a quelli
                // gia' presenti.
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
                     RETURNING item_id, exercise_type, due_at, last_reviewed_at, reps,
                               lapses, stability, difficulty, updated_at, rev",
                )
                .bind(answer.item_id)
                .bind(answer.exercise_type)
                .bind(scheduling.next.due_at)
                .bind(answer.answered_at)
                .bind(lapse)
                .bind(scheduling.next.memory.stability)
                .bind(scheduling.next.memory.difficulty)
                .bind(answer.answered_at)
                .bind(answer.answered_at)
                .fetch_one(&mut *tx)
                .await?;
                Some(card)
            }
        };

        tx.commit().await?;
        Ok(card)
    }

    /// Lo storico delle risposte su una carta, dalla piu' vecchia.
    pub async fn answers(&self, item_id: &str, exercise_type: &str) -> Result<Vec<AnswerLog>> {
        let answers = sqlx::query_as::<_, AnswerLog>(
            "SELECT id, item_id, exercise_type, answered_at, correct, answer, rating,
                    response_time_ms
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
            response_time_ms: Some(1_200),
            scheduling: Some(Scheduling {
                grade: Grade::from_correct(correct),
                next: pianificata(answered_at + TimeDelta::days(1)),
            }),
        }
    }

    /// Le materie con ripetizione spaziata aggiornano sempre la carta.
    async fn registra(db: &Database, answer: NewAnswer<'_>) -> Card {
        db.record_answer(answer)
            .await
            .unwrap()
            .expect("con la pianificazione la carta si aggiorna")
    }

    #[tokio::test]
    async fn un_database_nuovo_e_vuoto_ma_gia_migrato() {
        let db = Database::in_memory().await.unwrap();
        assert_eq!(db.card(ITEM, EXERCISE).await.unwrap(), None);
        assert!(
            db.due_cards(CardFilter::default(), Utc::now(), 10)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn una_risposta_senza_pianificazione_resta_solo_nello_storico() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        let card = db
            .record_answer(NewAnswer {
                scheduling: None,
                ..risposta(false, now)
            })
            .await
            .unwrap();

        assert_eq!(card, None, "non c'e' nessuna carta da aggiornare");
        assert_eq!(db.card(ITEM, EXERCISE).await.unwrap(), None);

        // Lo storico invece la registra, con `rating` vuoto: nessuno ha dato un
        // giudizio a quattro gradini, e inventarne uno racconterebbe a un futuro
        // riaddestramento una storia mai avvenuta.
        let storico = db.answers(ITEM, EXERCISE).await.unwrap();
        assert_eq!(storico.len(), 1);
        assert!(!storico[0].correct);
        assert_eq!(storico[0].rating, None);

        // Il tempo di risposta si registra anche qui: non ha niente a che vedere con
        // la pianificazione, e una materia che non usa la ripetizione spaziata
        // continua a costruire il dataset.
        assert_eq!(storico[0].response_time_ms, Some(1_200));
    }

    #[tokio::test]
    async fn il_tempo_di_risposta_puo_mancare_e_allora_resta_vuoto() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        db.record_answer(NewAnswer {
            response_time_ms: None,
            ..risposta(true, now)
        })
        .await
        .unwrap();

        // NULL vuol dire "non misurato", non "istantaneo": e' la stessa ragione per
        // cui `rating` e' nullable, cioe' non raccontare risposte mai avvenute.
        let storico = db.answers(ITEM, EXERCISE).await.unwrap();
        assert_eq!(storico[0].response_time_ms, None);
    }

    #[tokio::test]
    async fn la_prima_risposta_crea_la_carta() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        let card = registra(&db, risposta(true, now)).await;

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

        registra(&db, risposta(true, t0)).await;
        registra(&db, risposta(false, t0 + TimeDelta::minutes(1))).await;
        let card = registra(&db, risposta(true, t0 + TimeDelta::minutes(2))).await;

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

        registra(&db, risposta(true, t0)).await;
        registra(&db, risposta(false, t0 + TimeDelta::minutes(1))).await;

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

        assert_eq!(
            db.due_cards(CardFilter::default(), now, 10)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn reintrodurre_una_carta_non_ne_azzera_i_progressi() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        registra(&db, risposta(true, now)).await;
        let card = db.ensure_card(ITEM, EXERCISE, now).await.unwrap();

        assert_eq!(card.reps, 1);
        assert_eq!(card.rev, 1);
    }

    #[tokio::test]
    async fn sono_dovute_le_carte_mai_studiate_e_quelle_scadute() {
        let db = Database::in_memory().await.unwrap();
        let now = Utc::now();

        // Scaduta ieri.
        registra(
            &db,
            NewAnswer {
                item_id: "kana:hiragana:あ",
                scheduling: Some(Scheduling {
                    grade: Grade::Good,
                    next: pianificata(now - TimeDelta::days(1)),
                }),
                ..risposta(true, now - TimeDelta::days(2))
            },
        )
        .await;

        // Da rivedere domani.
        registra(
            &db,
            NewAnswer {
                item_id: "kana:hiragana:い",
                scheduling: Some(Scheduling {
                    grade: Grade::Good,
                    next: pianificata(now + TimeDelta::days(1)),
                }),
                ..risposta(true, now)
            },
        )
        .await;

        // Introdotta e mai studiata.
        db.ensure_card("kana:hiragana:う", EXERCISE, now)
            .await
            .unwrap();

        let dovute = db.due_cards(CardFilter::default(), now, 10).await.unwrap();
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

        assert_eq!(
            db.due_cards(CardFilter::default(), now, 2)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn i_dati_sopravvivono_alla_chiusura_e_le_migrazioni_si_possono_rieseguire() {
        let dir = tempfile::tempdir().unwrap();
        // La sottocartella non esiste: `open` deve crearla.
        let path = dir.path().join("dati").join("tanren.db");
        let now = Utc::now();

        {
            let db = Database::open(&path).await.unwrap();
            registra(&db, risposta(true, now)).await;
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
        let card = registra(
            &db,
            NewAnswer {
                item_id: ITEM,
                exercise_type: EXERCISE,
                correct: true,
                answer: "か",
                answered_at: t0,
                response_time_ms: Some(1_200),
                scheduling: Some(Scheduling {
                    grade: Grade::Good,
                    next: primo,
                }),
            },
        )
        .await;

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
