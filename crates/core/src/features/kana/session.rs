//! Il giro di una sessione di studio sui kana.
//!
//! Tiene insieme i tre pezzi che finora vivevano separati: l'esercizio sa costruire e
//! correggere le domande, lo scheduler sa quando ripresentarle, l'archivio sa cosa e'
//! gia' successo. Qui si decide in che ordine parlano.
//!
//! # Perche' sta nella feature e non nel livello condiviso
//!
//! Questo modulo sa cosa sia un kana, quali esercizi esistono e come si costruiscono i
//! loro identificatori. E' esattamente il genere di conoscenza che non deve salire in
//! `shared`. Quando arriveranno i kanji si vedra' cosa e' davvero comune e lo si
//! sposta: generalizzare adesso significherebbe indovinare.
//!
//! # Niente stato di sessione
//!
//! Non c'e' nessun oggetto sessione da tenere vivo. Ogni chiamata riparte dal
//! database, e l'ambito arriva insieme alla richiesta. Cosi' chiudere l'app a meta'
//! ripasso non perde niente e non c'e' niente da far scadere.

use chrono::{DateTime, TimeDelta, Utc};
use rand::Rng;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::features::kana::data::{KanaGroup, Syllabary, table};
use crate::features::kana::exercise::{KanaInput, KanaRecognition, item_id};
use crate::shared::error::Result;
use crate::shared::exercise::{
    Answer, ExerciseType, ExerciseTypeId, ItemId, Question, QuestionRequest, Verdict,
};
use crate::shared::srs::{Grade, Scheduler};
use crate::shared::storage::{CardFilter, Database, NewAnswer};

/// Quante opzioni mostrare nella scelta multipla, risposta giusta compresa.
const CHOICES: usize = 4;

/// Entro quanti minuti due scadenze contano come ugualmente urgenti.
///
/// Sotto quella soglia dire che una carta viene prima dell'altra e' una precisione
/// finta: sono scadute praticamente insieme, e l'ordine puo' deciderlo la sorte.
const SAME_URGENCY_MINUTES: i64 = 60;

static RECOGNITION: KanaRecognition = KanaRecognition;
static INPUT: KanaInput = KanaInput;

/// In che modo si sta allenando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Si vede il segno e si sceglie la trascrizione.
    Recognition,
    /// Si vede la trascrizione e si scrive il segno con l'IME.
    Input,
}

impl Mode {
    fn exercise(self) -> &'static dyn ExerciseType {
        match self {
            Self::Recognition => &RECOGNITION,
            Self::Input => &INPUT,
        }
    }

    pub fn exercise_id(self) -> ExerciseTypeId {
        self.exercise().id()
    }
}

/// Cosa si sta allenando in questa sessione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub syllabary: Syllabary,
    /// Le famiglie scelte. Vuoto significa tutte.
    #[serde(default)]
    pub groups: Vec<KanaGroup>,
    pub mode: Mode,
}

impl Scope {
    /// Gli elementi compresi nell'ambito, nell'ordine tradizionale.
    pub fn items(&self) -> Vec<ItemId> {
        table(self.syllabary)
            .all()
            .iter()
            .filter(|k| self.groups.is_empty() || self.groups.contains(&k.group))
            .map(|k| item_id(self.syllabary, &k.character))
            .collect()
    }

    fn filter<'a>(&self, items: &'a [String], exercise: &'a str) -> CardFilter<'a> {
        CardFilter {
            items: Some(items),
            exercise_type: Some(exercise),
        }
    }
}

/// A che punto e' la sessione.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Quanti segni comprende l'ambito.
    pub total: i64,
    /// Quanti sono da studiare adesso.
    pub due: i64,
}

fn ids(scope: &Scope) -> Vec<String> {
    scope
        .items()
        .into_iter()
        .map(|i| i.as_str().to_owned())
        .collect()
}

/// Prepara l'ambito: crea le carte che ancora non esistono e riporta a che punto si e'.
///
/// Va chiamata all'inizio di una sessione. Non tocca le carte gia' presenti, quindi
/// ripeterla e' innocuo.
pub async fn prepare(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<Progress> {
    let items = ids(scope);
    let exercise = scope.mode.exercise_id();
    let exercise = exercise.as_str();

    db.ensure_cards(&items, exercise, now).await?;

    progress(db, scope, now).await
}

/// A che punto e' l'ambito, senza modificare nulla.
pub async fn progress(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<Progress> {
    let items = ids(scope);
    let exercise = scope.mode.exercise_id();

    Ok(Progress {
        total: items.len() as i64,
        due: db
            .count_due(scope.filter(&items, exercise.as_str()), now)
            .await?,
    })
}

/// Un segno che si potrebbe chiedere adesso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub item: ItemId,
    /// Quando era dovuto. `None` se non e' mai stato studiato.
    pub due_at: Option<DateTime<Utc>>,
}

/// I segni che si potrebbero chiedere adesso, dal piu' arretrato in giu'.
///
/// Restituisce tutto l'ambito e non solo la prima carta perche' la scelta vera la fa
/// [`pick`], e per sorteggiare bisogna conoscere tutti quelli che se lo meritano. Sono
/// al massimo un centinaio di righe.
pub async fn due_items(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<Vec<Candidate>> {
    let items = ids(scope);
    let exercise = scope.mode.exercise_id();
    let limit = items.len() as i64;

    Ok(db
        .due_cards(scope.filter(&items, exercise.as_str()), now, limit)
        .await?
        .into_iter()
        .map(|card| Candidate {
            item: ItemId::new(card.item_id),
            due_at: card.due_at,
        })
        .collect())
}

/// Sceglie a sorte tra i candidati piu' urgenti.
///
/// # Perche' non si prende semplicemente il primo
///
/// L'urgenza non e' un ordine totale. Le carte mai viste hanno tutte la stessa
/// scadenza, cioe' nessuna, e l'archivio le restituisce nell'ordine in cui sono state
/// scritte, che e' quello tradizionale del gojuon. Prendendo sempre la prima, una
/// sessione nuova chiederebbe あ, い, う, え, お, か, き... tutte le volte: dopo due
/// giri non si riconosce piu' il segno, si indovina la posizione nella sequenza. E'
/// esattamente il contrario di quello che questa app deve allenare.
///
/// Quindi tra chi e' ugualmente urgente si tira a sorte, mentre chi e' davvero piu'
/// arretrato continua a passare avanti: la pianificazione di FSRS resta intatta,
/// perche' quello che si rompe qui e' solo un ordine che FSRS non aveva mai stabilito.
pub fn pick(candidates: &[Candidate], rng: &mut dyn Rng) -> Option<ItemId> {
    let head = candidates.first()?;
    let tolerance = TimeDelta::minutes(SAME_URGENCY_MINUTES);

    // I candidati arrivano ordinati, quindi quelli urgenti quanto il primo stanno in
    // testa: basta contare fin dove arrivano.
    let tied = candidates
        .iter()
        .take_while(|c| match (head.due_at, c.due_at) {
            // Mai studiate: nessuna e' piu' urgente di un'altra.
            (None, None) => true,
            (Some(primo), Some(altro)) => altro - primo <= tolerance,
            // Una carta mai vista e una gia' pianificata non si equivalgono, e
            // l'ordine tra le due lo ha gia' deciso l'archivio.
            _ => false,
        })
        .count();

    candidates[..tied].choose(rng).map(|c| c.item.clone())
}

/// Costruisce la domanda su un segno.
///
/// E' separata da [`due_items`] perche' non tocca il database: scegliere cosa
/// chiedere richiede una lettura, formulare la domanda no. Tenerle distinte lascia
/// questa parte pura e verificabile con un seme fisso, e evita che il generatore di
/// numeri casuali debba attraversare un'attesa asincrona.
pub fn question_for(scope: &Scope, item: &ItemId, rng: &mut dyn Rng) -> Result<Question> {
    scope.mode.exercise().question(
        QuestionRequest {
            item,
            pool: &scope.items(),
            distractors: CHOICES - 1,
        },
        rng,
    )
}

/// L'esito di una risposta, con quando si rivedra' il segno.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub verdict: Verdict,
    pub due_at: DateTime<Utc>,
    /// Fra quanti giorni tornera'. Puo' essere minore di uno.
    pub interval_days: f32,
}

/// Corregge una risposta, ripianifica il segno e registra tutto.
pub async fn submit(
    db: &Database,
    scheduler: &Scheduler,
    scope: &Scope,
    item: &ItemId,
    answer: &Answer,
    now: DateTime<Utc>,
) -> Result<Outcome> {
    let exercise = scope.mode.exercise();
    let exercise_id = exercise.id();
    let verdict = exercise.grade(item, answer)?;

    // Lo stato da cui riparte lo scheduler e' quello salvato: se la carta non c'e'
    // ancora, e' la prima volta che si vede questo segno.
    let card = db.card(item.as_str(), exercise_id.as_str()).await?;
    let scheduled = scheduler.schedule(
        card.as_ref().and_then(|c| c.memory()),
        card.as_ref().and_then(|c| c.last_reviewed_at),
        Grade::from_correct(verdict.is_correct()),
        now,
    )?;

    db.record_answer(NewAnswer {
        item_id: item.as_str(),
        exercise_type: exercise_id.as_str(),
        correct: verdict.is_correct(),
        answer: answer.as_str(),
        answered_at: now,
        grade: Grade::from_correct(verdict.is_correct()),
        next: scheduled,
    })
    .await?;

    Ok(Outcome {
        verdict,
        due_at: scheduled.due_at,
        interval_days: scheduled.interval_days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::exercise::{AnswerFormat, Prompt};
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashSet;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn base(mode: Mode) -> Scope {
        Scope {
            syllabary: Syllabary::Hiragana,
            groups: vec![KanaGroup::Base],
            mode,
        }
    }

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    /// Scorciatoia per i test: i tre passi che il comando fa uno dopo l'altro.
    async fn next_question(
        db: &Database,
        scope: &Scope,
        now: DateTime<Utc>,
        rng: &mut dyn Rng,
    ) -> Result<Option<Question>> {
        let candidates = due_items(db, scope, now).await?;
        match pick(&candidates, rng) {
            Some(item) => Ok(Some(question_for(scope, &item, rng)?)),
            None => Ok(None),
        }
    }

    fn candidate(id: &str, due_at: Option<DateTime<Utc>>) -> Candidate {
        Candidate {
            item: ItemId::new(id),
            due_at,
        }
    }

    #[tokio::test]
    async fn preparare_crea_le_carte_dell_ambito_e_solo_quelle() {
        let db = db().await;
        let now = Utc::now();

        let p = prepare(&db, &base(Mode::Recognition), now).await.unwrap();

        assert_eq!(p.total, 46, "la famiglia di base ha 46 segni");
        assert_eq!(p.due, 46, "appena introdotte sono tutte da studiare");

        // L'altro esercizio sullo stesso ambito non e' stato toccato: sono carte
        // distinte.
        let altro = progress(&db, &base(Mode::Input), now).await.unwrap();
        assert_eq!(altro.due, 0);
    }

    #[tokio::test]
    async fn preparare_due_volte_non_raddoppia_niente() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Recognition);

        prepare(&db, &scope, now).await.unwrap();
        let p = prepare(&db, &scope, now).await.unwrap();

        assert_eq!(p.due, 46);
    }

    #[tokio::test]
    async fn la_domanda_resta_dentro_l_ambito() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Recognition);
        prepare(&db, &scope, now).await.unwrap();

        let q = next_question(&db, &scope, now, &mut rng())
            .await
            .unwrap()
            .expect("c'e' da studiare");

        let Prompt::Japanese(segno) = &q.prompt else {
            panic!("il riconoscimento mostra il segno");
        };
        let dentro = table(Syllabary::Hiragana)
            .group(KanaGroup::Base)
            .any(|k| &k.character == segno);
        assert!(dentro, "segno fuori ambito: {segno}");

        let AnswerFormat::Choice { options } = &q.format else {
            panic!("il riconoscimento e' a scelta multipla");
        };
        assert_eq!(options.len(), CHOICES);
    }

    #[tokio::test]
    async fn senza_carte_dovute_non_c_e_nessuna_domanda() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Recognition);

        // Nessun `prepare`: non esiste ancora nessuna carta.
        assert!(
            next_question(&db, &scope, now, &mut rng())
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn rispondere_bene_toglie_il_segno_da_quelli_dovuti() {
        let db = db().await;
        let scheduler = Scheduler::default();
        let now = Utc::now();
        let scope = base(Mode::Input);
        prepare(&db, &scope, now).await.unwrap();

        let q = next_question(&db, &scope, now, &mut rng())
            .await
            .unwrap()
            .unwrap();
        let Prompt::Latin(romaji) = &q.prompt else {
            panic!("l'input mostra la trascrizione");
        };
        assert_eq!(q.format, AnswerFormat::Input);

        // Si risale al segno atteso dall'identificatore della domanda.
        let segno = q.item.as_str().rsplit(':').next().unwrap().to_owned();
        assert!(!romaji.is_empty());

        let esito = submit(&db, &scheduler, &scope, &q.item, &Answer::new(&segno), now)
            .await
            .unwrap();

        assert_eq!(esito.verdict, Verdict::Correct);
        assert!(esito.due_at > now);

        let p = progress(&db, &scope, now).await.unwrap();
        assert_eq!(p.due, 45, "il segno appena studiato non e' piu' dovuto");
    }

    #[tokio::test]
    async fn rispondere_male_lo_rimanda_indietro_di_poco() {
        let db = db().await;
        let scheduler = Scheduler::default();
        let now = Utc::now();
        let scope = base(Mode::Input);
        prepare(&db, &scope, now).await.unwrap();

        let q = next_question(&db, &scope, now, &mut rng())
            .await
            .unwrap()
            .unwrap();

        let esito = submit(&db, &scheduler, &scope, &q.item, &Answer::new("ん"), now)
            .await
            .unwrap();

        assert!(!esito.verdict.is_correct());
        assert!(
            esito.interval_days < 1.0,
            "una risposta sbagliata deve tornare in giornata: {} giorni",
            esito.interval_days
        );
    }

    #[tokio::test]
    async fn la_sessione_si_svuota_rispondendo_a_tutto() {
        let db = db().await;
        let scheduler = Scheduler::default();
        let now = Utc::now();
        let scope = Scope {
            syllabary: Syllabary::Katakana,
            groups: vec![KanaGroup::Handakuten],
            mode: Mode::Recognition,
        };

        let p = prepare(&db, &scope, now).await.unwrap();
        assert_eq!(p.total, 5);

        for _ in 0..5 {
            let q = next_question(&db, &scope, now, &mut rng())
                .await
                .unwrap()
                .expect("ne resta almeno una");
            // La risposta giusta e' la trascrizione canonica del segno mostrato.
            let Prompt::Japanese(segno) = &q.prompt else {
                unreachable!()
            };
            let atteso = table(Syllabary::Katakana)
                .all()
                .iter()
                .find(|k| &k.character == segno)
                .unwrap()
                .romaji[0]
                .clone();

            let esito = submit(&db, &scheduler, &scope, &q.item, &Answer::new(atteso), now)
                .await
                .unwrap();
            assert_eq!(esito.verdict, Verdict::Correct);
        }

        assert!(
            next_question(&db, &scope, now, &mut rng())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(progress(&db, &scope, now).await.unwrap().due, 0);
    }

    #[test]
    fn senza_candidati_non_si_sceglie_niente() {
        assert_eq!(pick(&[], &mut rng()), None);
    }

    #[test]
    fn tra_le_carte_mai_viste_si_tira_a_sorte() {
        let candidati: Vec<_> = ["a", "b", "c", "d", "e"]
            .into_iter()
            .map(|id| candidate(id, None))
            .collect();

        let usciti: HashSet<_> = (0..12)
            .filter_map(|seme| pick(&candidati, &mut StdRng::seed_from_u64(seme)))
            .collect();

        assert!(
            usciti.len() > 1,
            "l'ordine di scrittura non deve diventare l'ordine delle domande: {usciti:?}"
        );
    }

    #[test]
    fn una_carta_molto_arretrata_passa_avanti() {
        let now = Utc::now();
        let candidati = vec![
            candidate("arretrata", Some(now - TimeDelta::days(2))),
            candidate("appena scaduta", Some(now - TimeDelta::minutes(1))),
            candidate("scaduta ora", Some(now)),
        ];

        // Qui la sorte non c'entra: chi aspetta da due giorni non aspetta un giro in
        // piu' perche' e' uscito un numero.
        for seme in 0..12 {
            let scelta = pick(&candidati, &mut StdRng::seed_from_u64(seme)).unwrap();
            assert_eq!(scelta.as_str(), "arretrata");
        }
    }

    #[test]
    fn le_scadenze_vicine_contano_come_pari() {
        let now = Utc::now();
        let candidati = vec![
            candidate("novanta", Some(now - TimeDelta::minutes(90))),
            candidate("ottanta", Some(now - TimeDelta::minutes(80))),
            candidate("dieci", Some(now - TimeDelta::minutes(10))),
        ];

        let usciti: HashSet<_> = (0..12)
            .filter_map(|seme| pick(&candidati, &mut StdRng::seed_from_u64(seme)))
            .map(|i| i.as_str().to_owned())
            .collect();

        assert!(
            usciti.contains("novanta") && usciti.contains("ottanta"),
            "dieci minuti di scarto non sono una precedenza: {usciti:?}"
        );
        assert!(
            !usciti.contains("dieci"),
            "un'ora e mezzo di ritardo invece conta: {usciti:?}"
        );
    }

    #[tokio::test]
    async fn una_sessione_nuova_non_segue_l_ordine_della_tabella() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Recognition);
        prepare(&db, &scope, now).await.unwrap();

        let candidati = due_items(&db, &scope, now).await.unwrap();
        assert_eq!(candidati.len(), 46, "vanno guardate tutte, non solo la prima");

        let primi: HashSet<_> = (0..12)
            .filter_map(|seme| pick(&candidati, &mut StdRng::seed_from_u64(seme)))
            .map(|i| i.as_str().to_owned())
            .collect();

        assert!(
            primi.len() > 1,
            "la prima domanda sarebbe sempre あ, e la sequenza del gojuon diventerebbe \
             piu' facile da ricordare dei segni: {primi:?}"
        );
    }
}
