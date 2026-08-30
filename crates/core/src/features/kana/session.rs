//! L'ambito di una sessione sui kana.
//!
//! # Cosa e' rimasto qui, e cosa e' salito
//!
//! La **regola del giro** (chi esce dalla coda, chi rientra e dove, come si mescola,
//! cosa si registra) sta in [`crate::shared::session`], perche' e' identica per tutte
//! le materie. Qui e' rimasto quello che sa di kana: cosa sia un sillabario e una
//! famiglia, quali segni compongono l'ambito scelto e quale esercizio si sta facendo.
//!
//! La divisione e' quella che il progetto aveva rimandato: generalizzare con una
//! materia sola significa indovinare, e coi kanji e' arrivato il secondo caso da
//! confrontare.
//!
//! # Cos'e' una sessione
//!
//! Un giro completo sull'ambito scelto, mescolato: si vedono tutti i segni delle
//! famiglie selezionate, in ordine casuale, e **un segno esce dal giro solo quando lo
//! si indovina**. Finito il giro se ne puo' cominciare subito un altro, con un ordine
//! nuovo.
//!
//! **Sui kana non c'e' ripetizione spaziata.** Non e' solo che le scadenze non
//! decidono cosa entra in una sessione: non vengono proprio calcolate. I kana sono
//! l'alfabeto di base, e far riaspettare due giorni una lettera appena sbagliata la fa
//! dimenticare del tutto invece di fissarla. FSRS resta nel core per i kanji, dove gli
//! intervalli lunghi hanno senso. Le risposte finiscono comunque nello storico, che e'
//! in sola aggiunta e non ha niente a che vedere con le scadenze.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::features::kana::data::{KanaGroup, Syllabary, table};
use crate::features::kana::exercise::{KanaInput, KanaRecognition, item_id};
use crate::shared::error::Result;
use crate::shared::exercise::{Answer, ExerciseType, ExerciseTypeId, ItemId, Verdict};
use crate::shared::session;
use crate::shared::storage::Database;

pub use crate::shared::session::Step;

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
}

/// Comincia il giro: l'ambito mescolato, e la prima domanda.
pub fn start(scope: &Scope, rng: &mut dyn Rng) -> Result<Step> {
    session::start(&scope.items(), scope.mode.exercise(), rng)
}

/// Come continua il giro dopo una risposta.
pub fn advance(scope: &Scope, queue: &[ItemId], correct: bool, rng: &mut dyn Rng) -> Result<Step> {
    session::advance(&scope.items(), scope.mode.exercise(), queue, correct, rng)
}

/// Corregge una risposta e la registra.
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

    #[test]
    fn il_piano_copre_l_ambito_una_volta_sola() {
        let scope = base(Mode::Recognition);
        let giro = start(&scope, &mut rng()).unwrap().queue;

        assert_eq!(giro.len(), 46, "la famiglia di base ha 46 segni");

        let unici: HashSet<_> = giro.iter().collect();
        assert_eq!(unici.len(), giro.len(), "nessun segno chiesto due volte");

        let ambito: HashSet<_> = scope.items().into_iter().collect();
        assert_eq!(
            giro.into_iter().collect::<HashSet<_>>(),
            ambito,
            "e nessuno lasciato fuori"
        );
    }

    #[test]
    fn il_piano_non_segue_l_ordine_della_tabella() {
        let scope = base(Mode::Recognition);

        let primi: HashSet<_> = (0..12)
            .filter_map(|seme| {
                start(&scope, &mut StdRng::seed_from_u64(seme))
                    .unwrap()
                    .queue
                    .first()
                    .map(|i| i.as_str().to_owned())
            })
            .collect();

        assert!(
            primi.len() > 1,
            "la prima domanda sarebbe sempre あ, e la sequenza del gojuon diventerebbe \
             piu' facile da ricordare dei segni: {primi:?}"
        );
    }

    #[test]
    fn un_mix_di_famiglie_le_comprende_tutte() {
        let scope = Scope {
            syllabary: Syllabary::Katakana,
            groups: vec![KanaGroup::Handakuten, KanaGroup::Dakuten],
            mode: Mode::Recognition,
        };

        let giro = start(&scope, &mut rng()).unwrap().queue;
        assert_eq!(giro.len(), 25, "20 sonori e 5 semisonori");
    }

    #[test]
    fn senza_famiglie_scelte_si_allena_tutto_il_sillabario() {
        let scope = Scope {
            syllabary: Syllabary::Hiragana,
            groups: Vec::new(),
            mode: Mode::Recognition,
        };

        assert_eq!(start(&scope, &mut rng()).unwrap().queue.len(), 107);
    }

    #[test]
    fn la_domanda_resta_dentro_l_ambito() {
        let scope = base(Mode::Recognition);
        let step = start(&scope, &mut rng()).unwrap();
        let q = step.question.expect("il giro comincia con una domanda");

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
        assert_eq!(options.len(), session::CHOICES);
    }

    fn handakuten() -> Scope {
        Scope {
            syllabary: Syllabary::Katakana,
            groups: vec![KanaGroup::Handakuten],
            mode: Mode::Recognition,
        }
    }

    #[test]
    fn il_giro_finisce_solo_indovinando_tutto() {
        let scope = handakuten();
        let mut rng = rng();
        let mut coda = start(&scope, &mut rng).unwrap().queue;
        assert_eq!(coda.len(), 5);

        // Sbagliare non avvicina la fine, per quante volte lo si faccia.
        for _ in 0..50 {
            coda = advance(&scope, &coda, false, &mut rng).unwrap().queue;
        }
        assert_eq!(coda.len(), 5);

        // Indovinando invece si svuota, un segno per volta.
        let mut ultimo = None;
        while !coda.is_empty() {
            let step = advance(&scope, &coda, true, &mut rng).unwrap();
            coda = step.queue;
            ultimo = Some(step.question);
        }

        assert_eq!(
            ultimo.expect("almeno un passo"),
            None,
            "a coda vuota non c'e' piu' niente da chiedere"
        );
    }

    #[tokio::test]
    async fn rispondere_bene_finisce_nello_storico_e_non_crea_carte() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Input);
        let mut rng = rng();

        let q = start(&scope, &mut rng)
            .unwrap()
            .question
            .expect("il giro comincia con una domanda");
        let Prompt::Latin(romaji) = &q.prompt else {
            panic!("l'input mostra la trascrizione");
        };
        assert_eq!(q.format, AnswerFormat::Input);
        assert!(!romaji.is_empty());

        // Si risale al segno atteso dall'identificatore della domanda.
        let segno = q.item.as_str().rsplit(':').next().unwrap().to_owned();

        let verdict = submit(&db, &scope, &q.item, &Answer::new(&segno), Some(1_500), now)
            .await
            .unwrap();

        assert_eq!(verdict, Verdict::Correct);

        let esercizio = Mode::Input.exercise_id();
        let storico = db.answers(q.item.as_str(), esercizio.as_str()).await.unwrap();
        assert_eq!(storico.len(), 1);
        assert!(storico[0].correct);

        // Il tempo di risposta viene registrato anche senza pianificazione: e' dato
        // per il dataset, non per le scadenze.
        assert_eq!(storico[0].response_time_ms, Some(1_500));

        // Nessuna carta: sui kana non c'e' ripetizione spaziata da tenere.
        assert_eq!(
            db.card(q.item.as_str(), esercizio.as_str()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn rispondere_male_dice_cosa_si_accettava() {
        let db = db().await;
        let now = Utc::now();
        let scope = base(Mode::Input);
        let mut rng = rng();

        let item = start(&scope, &mut rng).unwrap().queue.remove(0);

        let verdict = submit(&db, &scope, &item, &Answer::new("ん"), None, now)
            .await
            .unwrap();

        let Verdict::Incorrect { accepted } = verdict else {
            panic!("ん non e' la risposta a nessuno degli altri segni");
        };
        assert!(!accepted.is_empty(), "va detto cosa sarebbe andato bene");
    }

    #[tokio::test]
    async fn un_giro_intero_si_puo_rifare_subito() {
        let db = db().await;
        let now = Utc::now();
        let scope = handakuten();
        let mut rng = rng();

        let mut step = start(&scope, &mut rng).unwrap();
        while let Some(q) = step.question.clone() {
            let Prompt::Japanese(segno) = &q.prompt else {
                unreachable!()
            };
            // La risposta giusta e' la trascrizione canonica del segno mostrato.
            let atteso = table(Syllabary::Katakana)
                .all()
                .iter()
                .find(|k| &k.character == segno)
                .unwrap()
                .romaji[0]
                .clone();

            let verdict = submit(&db, &scope, &q.item, &Answer::new(atteso), Some(900), now)
                .await
                .unwrap();
            assert_eq!(verdict, Verdict::Correct);

            step = advance(&scope, &step.queue, true, &mut rng).unwrap();
        }

        // Il giro appena finito non chiude la porta: le scadenze non decidono cosa
        // entra in una sessione, quindi il secondo giro e' pieno quanto il primo.
        assert_eq!(start(&scope, &mut rng).unwrap().queue.len(), 5);
    }
}
