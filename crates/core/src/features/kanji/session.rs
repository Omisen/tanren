//! L'ambito di una sessione sui kanji.
//!
//! Come per i kana, la **regola del giro** sta in [`crate::shared::session`] e qui
//! resta solo cio' che sa di kanji: cosa sia un grado e una famiglia di letture, quali
//! item ne fanno parte e quale esercizio si sta facendo.
//!
//! # L'ambito e' un grado piu' delle famiglie
//!
//! Si sceglie un anno di scuola e una o piu' fra [`Family::On`], [`Family::Kun`] e
//! [`Family::Okurigana`]. E' la stessa forma dell'ambito dei kana, dove si sceglie un
//! sillabario e delle famiglie di segni, e i numeri sono paragonabili: il giro piu'
//! corto del primo anno fa 69 item, il piu' lungo 226, contro i 107 di un sillabario
//! intero.
//!
//! **Non c'e' una fetta per frequenza**, del tipo «i primi 20 del primo anno», anche se
//! il dato e' gia' ordinato per frequenza e costerebbe poco. E' interfaccia in piu'
//! prima di sapere se serve.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::features::kanji::data::Grade;
use crate::features::kanji::exercise::{self, Family, KanjiRecognition, item_id};
use crate::shared::error::Result;
use crate::shared::exercise::{Answer, ExerciseType, ExerciseTypeId, ItemId, Verdict};
use crate::shared::session::{self, Retry, Task};
use crate::shared::storage::Database;

pub use crate::shared::session::Step;

static RECOGNITION: KanjiRecognition = KanjiRecognition;

/// In che modo si sta allenando.
///
/// Per ora ce n'e' uno solo. La scrittura con l'IME e' il passo successivo, e sara' la
/// stessa domanda con un formato di risposta diverso: sui kanji **entrambe le modalita'
/// vanno da kanji a lettura**, perche' il verso opposto non ha una risposta sola.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Si vede la forma scritta e si sceglie la lettura.
    Recognition,
}

impl Mode {
    pub fn exercise(self) -> &'static dyn ExerciseType {
        match self {
            Self::Recognition => &RECOGNITION,
        }
    }

    pub fn exercise_id(self) -> ExerciseTypeId {
        self.exercise().id()
    }
}

/// Cosa si sta allenando in questa sessione.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub grade: Grade,
    /// Le famiglie scelte. Vuoto significa tutte.
    #[serde(default)]
    pub families: Vec<Family>,
    pub mode: Mode,
}

impl Scope {
    /// Gli item compresi nell'ambito, nell'ordine della tabella.
    pub fn items(&self) -> Vec<ItemId> {
        exercise::items(self.grade, &self.families)
            .into_iter()
            .map(|i| item_id(self.grade, i.family, &i.form))
            .collect()
    }
}

/// I compiti dell'ambito: gli stessi item, tutti con l'esercizio della modalita'.
fn tasks(scope: &Scope) -> Vec<Task> {
    let exercise = scope.mode.exercise_id();
    scope
        .items()
        .into_iter()
        .map(|item| Task::new(item, exercise.clone()))
        .collect()
}

fn lookup(id: &ExerciseTypeId) -> Option<&'static dyn ExerciseType> {
    (Mode::Recognition.exercise_id() == *id).then(|| Mode::Recognition.exercise())
}

/// Comincia il giro: l'ambito mescolato, e la prima domanda.
pub fn start(scope: &Scope, rng: &mut dyn Rng) -> Result<Step> {
    session::start(&tasks(scope), lookup, rng)
}

/// Come continua il giro dopo una risposta.
pub fn advance(scope: &Scope, queue: &[Task], correct: bool, rng: &mut dyn Rng) -> Result<Step> {
    session::advance(&tasks(scope), lookup, queue, correct, Retry::UntilRight, rng)
}

/// Corregge una risposta e la registra.
///
/// FSRS non entra ancora qui: la risposta finisce nello storico e le carte non vengono
/// toccate, esattamente come per i kana. Collegarlo e' un passo a se', con lo stato
/// esplicito del ciclo di vita e le tre modalita' della sezione 3 di CLAUDE.md.
pub async fn submit(
    db: &Database,
    scope: &Scope,
    item: &ItemId,
    answer: &Answer,
    response_time_ms: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    session::submit(
        db,
        scope.mode.exercise(),
        item,
        answer,
        response_time_ms,
        // La versione precedente dell'esercizio non collega FSRS. Lo fa il
        // redesign, con le faccette.
        None,
        now,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::exercise::{AnswerFormat, Prompt};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn scope(families: Vec<Family>) -> Scope {
        Scope {
            grade: Grade::First,
            families,
            mode: Mode::Recognition,
        }
    }

    #[test]
    fn l_ambito_e_un_grado_piu_delle_famiglie() {
        assert_eq!(scope(vec![Family::On]).items().len(), 80);
        assert_eq!(scope(vec![Family::Kun]).items().len(), 69);
        assert_eq!(
            scope(vec![Family::On, Family::Kun]).items().len(),
            149,
            "le famiglie si sommano"
        );
        assert_eq!(scope(Vec::new()).items().len(), 226, "vuoto significa tutte");
    }

    #[test]
    fn la_domanda_resta_dentro_l_ambito() {
        let scope = scope(vec![Family::On]);
        let step = start(&scope, &mut rng()).unwrap();
        let q = step.question.expect("il giro comincia con una domanda");

        let Prompt::Japanese(forma) = &q.prompt else {
            panic!("si mostra la forma scritta");
        };
        assert!(
            scope.items().iter().any(|i| i.as_str().ends_with(forma)),
            "forma fuori ambito: {forma}"
        );
        assert_eq!(q.asks.as_deref(), Some("on"));

        let AnswerFormat::Choice { options } = &q.format else {
            panic!("il riconoscimento e' a scelta multipla");
        };
        assert_eq!(options.len(), session::CHOICES);
    }

    #[tokio::test]
    async fn rispondere_finisce_nello_storico_e_non_crea_carte() {
        let db = Database::in_memory().await.unwrap();
        let scope = scope(vec![Family::On]);
        let item = item_id(Grade::First, Family::On, "生");

        let verdict = submit(
            &db,
            &scope,
            &item,
            &Answer::new("せい"),
            Some(900),
            Utc::now(),
        )
        .await
        .unwrap();

        assert_eq!(verdict, Verdict::correct(), "せい e' una lettura on di 生");

        let esercizio = Mode::Recognition.exercise_id();
        let storico = db.answers(item.as_str(), esercizio.as_str()).await.unwrap();
        assert_eq!(storico.len(), 1);
        assert_eq!(storico[0].response_time_ms, Some(900));
        assert_eq!(db.card(item.as_str(), esercizio.as_str()).await.unwrap(), None);
    }

    #[test]
    fn un_giro_intero_si_puo_rifare_subito() {
        let scope = scope(vec![Family::Kun]);
        let mut rng = rng();

        let mut step = start(&scope, &mut rng).unwrap();
        let mut risposte = 0;
        while step.question.is_some() {
            step = advance(&scope, &step.queue, true, &mut rng).unwrap();
            risposte += 1;
        }
        assert_eq!(risposte, 69, "il giro copre l'ambito una volta sola");

        // Le scadenze non decidono cosa entra in una sessione: il secondo giro e' pieno
        // quanto il primo.
        assert_eq!(start(&scope, &mut rng).unwrap().queue.len(), 69);
    }
}
