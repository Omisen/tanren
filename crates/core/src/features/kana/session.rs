//! Il giro di una sessione di studio sui kana.
//!
//! Tiene insieme i pezzi che vivono separati: l'esercizio sa costruire e correggere le
//! domande, l'archivio sa cosa e' gia' successo, lo scheduler sa quando un segno
//! andrebbe ripresentato. Qui si decide in che ordine parlano.
//!
//! # Cos'e' una sessione
//!
//! Un giro completo sull'ambito scelto, mescolato: si vedono tutti i segni delle
//! famiglie selezionate, in ordine casuale, e **un segno esce dal giro solo quando lo
//! si indovina**. Chi sbaglia se lo ritrova poco piu' avanti, finche' non lo azzecca.
//! Finito il giro se ne puo' cominciare subito un altro, con un ordine nuovo.
//!
//! # La coda
//!
//! Il core non tiene viva nessuna sessione: la coda e' un **valore** che viene
//! restituito insieme a ogni domanda e che l'interfaccia si limita a conservare e a
//! rimandare indietro. Non la interpreta e non la manipola: cosa esce, cosa rientra e
//! dove, lo decide [`advance`]. Cosi' lo stato sta dove deve stare, cioe' in una cosa
//! che muore quando si esce, e la regola sta dove deve stare, cioe' qui.
//!
//! **Sui kana non c'e' ripetizione spaziata.** Non e' solo che le scadenze non
//! decidono cosa entra in una sessione: non vengono proprio calcolate. I kana sono
//! l'alfabeto di base, e far riaspettare due giorni una lettera appena sbagliata la fa
//! dimenticare del tutto invece di fissarla: all'inizio la memoria ha bisogno di
//! stimoli ravvicinati, di minuti o ore, non di giorni. FSRS resta nel core per i
//! kanji, dove gli intervalli lunghi hanno senso. Le risposte finiscono comunque nello
//! storico, che e' in sola aggiunta e non ha niente a che vedere con le scadenze.
//!
//! # Perche' sta nella feature e non nel livello condiviso
//!
//! Questo modulo sa cosa sia un kana, quali esercizi esistono e come si costruiscono i
//! loro identificatori. E' esattamente il genere di conoscenza che non deve salire in
//! `shared`. Quando arriveranno i kanji si vedra' cosa e' davvero comune e lo si
//! sposta: generalizzare adesso significherebbe indovinare.
//!
//! # Niente stato di sessione nel core
//!
//! Il core non tiene viva nessuna sessione. Produce l'ordine da seguire ([`plan`]) e
//! poi risponde a domande singole: quale domanda per questo segno, com'e' andata
//! questa risposta. A che punto e' il giro lo sa solo l'interfaccia, ed e' giusto
//! cosi', perche' e' informazione che deve morire quando si esce.

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};

use crate::features::kana::data::{KanaGroup, Syllabary, table};
use crate::features::kana::exercise::{KanaInput, KanaRecognition, item_id};
use crate::shared::error::Result;
use crate::shared::exercise::{
    Answer, ExerciseType, ExerciseTypeId, ItemId, Question, QuestionRequest, Verdict,
};
use crate::shared::storage::{Database, NewAnswer};

/// Quante opzioni mostrare nella scelta multipla, risposta giusta compresa.
const CHOICES: usize = 4;

/// Dopo quanti altri segni torna quello sbagliato.
///
/// Deve tornare presto, perche' la correzione va fissata mentre l'errore e' ancora
/// fresco: e' esattamente il motivo per cui sui kana non si usa la ripetizione
/// spaziata. Ma non subito: rispondere di nuovo un istante dopo aver letto la
/// soluzione non e' ricordare, e' copiare. La distanza varia perche' un ritorno a
/// scadenza fissa si impara come ritmo.
const RETRY_GAP: [usize; 3] = [2, 3, 4];

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

/// Una domanda aperta e la coda che resta da fare.
///
/// La coda e' opaca per chi la riceve: si conserva e si rimanda indietro alla chiamata
/// dopo, non si guarda dentro. `question` a `None` significa che il giro e' finito.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub question: Option<Question>,
    pub queue: Vec<ItemId>,
}

/// Comincia il giro: l'ambito mescolato, e la prima domanda.
pub fn start(scope: &Scope, rng: &mut dyn Rng) -> Result<Step> {
    open(scope, plan(scope, rng), rng)
}

/// Come continua il giro dopo una risposta.
///
/// Indovinato, il segno esce dalla coda. Sbagliato, ci rientra qualche domanda piu'
/// avanti: e' l'unico modo di uscirne, quindi una sessione finisce solo quando ogni
/// segno e' stato azzeccato almeno una volta.
pub fn advance(
    scope: &Scope,
    queue: &[ItemId],
    correct: bool,
    rng: &mut dyn Rng,
) -> Result<Step> {
    open(scope, requeue(queue, correct, rng), rng)
}

/// Formula la domanda in cima alla coda, se ce n'e' una.
fn open(scope: &Scope, queue: Vec<ItemId>, rng: &mut dyn Rng) -> Result<Step> {
    let question = match queue.first() {
        Some(item) => Some(question_for(scope, item, rng)?),
        None => None,
    };

    Ok(Step { question, queue })
}

/// L'ordine in cui affrontare l'ambito all'inizio di una sessione.
///
/// # Perche' mescolato
///
/// L'ordine naturale della tabella e' quello del gojuon: あ, い, う, え, お, か, き...
/// Seguirlo significa insegnare la sequenza invece dei segni, perche' dopo due giri la
/// posizione nella filastrocca si ricorda meglio del carattere. Qui si allena il
/// riconoscimento, quindi l'ordine deve essere imprevedibile.
///
/// La casualita' arriva da fuori, come per i distrattori: i test fissano un seme e
/// ottengono sempre lo stesso giro.
fn plan(scope: &Scope, rng: &mut dyn Rng) -> Vec<ItemId> {
    let mut items = scope.items();
    items.shuffle(rng);
    items
}

/// Toglie dalla coda il segno appena chiesto, e lo rimette dentro se e' andato male.
fn requeue(queue: &[ItemId], correct: bool, rng: &mut dyn Rng) -> Vec<ItemId> {
    let mut rest = queue.to_vec();
    if rest.is_empty() {
        return rest;
    }

    let asked = rest.remove(0);
    if !correct {
        // `choose` torna un'opzione perche' una fetta puo' essere vuota, e questa e'
        // una costante di tre elementi. Se davanti non c'e' abbastanza roba il segno
        // finisce in fondo: piu' lontano di cosi' non si puo' metterlo.
        let gap = RETRY_GAP.choose(rng).copied().unwrap_or(RETRY_GAP[0]);
        let at = gap.min(rest.len());
        rest.insert(at, asked);
    }

    rest
}

/// Costruisce la domanda su un segno.
///
/// E' pura e non tocca il database, quindi e' verificabile con un seme fisso. Serve
/// anche tecnicamente: il generatore di numeri casuali non deve attraversare
/// un'attesa asincrona, altrimenti il future non e' `Send` e Tauri non lo accetta.
fn question_for(scope: &Scope, item: &ItemId, rng: &mut dyn Rng) -> Result<Question> {
    scope.mode.exercise().question(
        QuestionRequest {
            item,
            pool: &scope.items(),
            distractors: CHOICES - 1,
        },
        rng,
    )
}

/// Corregge una risposta e la registra.
///
/// # Perche' qui non si pianifica niente
///
/// I kana sono l'alfabeto di base, e la ripetizione spaziata su un alfabeto lavora
/// contro chi impara: far riaspettare due giorni una lettera appena sbagliata la fa
/// dimenticare del tutto. All'inizio la memoria ha bisogno di stimoli ravvicinati, di
/// minuti o ore, non di giorni. FSRS resta nel core ([`crate::shared::srs`]) per i
/// kanji, dove gli intervalli lunghi hanno senso.
///
/// La risposta entra comunque nello storico: `answers` e' in sola aggiunta e sapere
/// quante volte un segno e' stato sbagliato serve a prescindere dalle scadenze. La
/// carta invece non viene toccata, perche' esiste per tenere lo stato della
/// ripetizione spaziata e qui non ce n'e' nessuno.
pub async fn submit(
    db: &Database,
    scope: &Scope,
    item: &ItemId,
    answer: &Answer,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    let exercise = scope.mode.exercise();
    let exercise_id = exercise.id();
    let verdict = exercise.grade(item, answer)?;

    db.record_answer(NewAnswer {
        item_id: item.as_str(),
        exercise_type: exercise_id.as_str(),
        correct: verdict.is_correct(),
        answer: answer.as_str(),
        answered_at: now,
        scheduling: None,
    })
    .await?;

    Ok(verdict)
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
        let giro = plan(&scope, &mut rng());

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
                plan(&scope, &mut StdRng::seed_from_u64(seme))
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

        assert_eq!(plan(&scope, &mut rng()).len(), 25, "20 sonori e 5 semisonori");
    }

    #[test]
    fn senza_famiglie_scelte_si_allena_tutto_il_sillabario() {
        let scope = Scope {
            syllabary: Syllabary::Hiragana,
            groups: Vec::new(),
            mode: Mode::Recognition,
        };

        assert_eq!(plan(&scope, &mut rng()).len(), 107);
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
        assert_eq!(options.len(), CHOICES);
    }

    fn handakuten() -> Scope {
        Scope {
            syllabary: Syllabary::Katakana,
            groups: vec![KanaGroup::Handakuten],
            mode: Mode::Recognition,
        }
    }

    #[test]
    fn un_segno_indovinato_esce_dalla_coda() {
        let scope = handakuten();
        let mut rng = rng();
        let step = start(&scope, &mut rng).unwrap();
        let chiesto = step.queue[0].clone();

        let dopo = advance(&scope, &step.queue, true, &mut rng).unwrap();

        assert_eq!(dopo.queue.len(), step.queue.len() - 1);
        assert!(!dopo.queue.contains(&chiesto), "indovinato, non torna piu'");
    }

    #[test]
    fn un_segno_sbagliato_rientra_poco_piu_avanti() {
        let scope = base(Mode::Recognition);
        let mut rng = rng();
        let step = start(&scope, &mut rng).unwrap();
        let chiesto = step.queue[0].clone();

        let dopo = advance(&scope, &step.queue, false, &mut rng).unwrap();

        assert_eq!(dopo.queue.len(), step.queue.len(), "sbagliando non esce nessuno");
        let posizione = dopo
            .queue
            .iter()
            .position(|i| i == &chiesto)
            .expect("il segno sbagliato resta in coda");
        assert!(
            (2..=4).contains(&posizione),
            "ne' subito ne' alla prossima vita: {posizione}"
        );
    }

    #[test]
    fn con_un_solo_segno_rimasto_lo_si_rivede_subito() {
        let scope = handakuten();
        let mut rng = rng();
        let coda = vec![ItemId::new("kana:katakana:パ")];

        let dopo = advance(&scope, &coda, false, &mut rng).unwrap();

        assert_eq!(dopo.queue, coda, "non c'e' niente da mettergli davanti");
        assert!(dopo.question.is_some());
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

    #[test]
    fn una_coda_vuota_non_ha_domande() {
        let step = advance(&handakuten(), &[], true, &mut rng()).unwrap();

        assert!(step.queue.is_empty());
        assert_eq!(step.question, None);
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

        let verdict = submit(&db, &scope, &q.item, &Answer::new(&segno), now)
            .await
            .unwrap();

        assert_eq!(verdict, Verdict::Correct);

        let esercizio = Mode::Input.exercise_id();
        let storico = db.answers(q.item.as_str(), esercizio.as_str()).await.unwrap();
        assert_eq!(storico.len(), 1);
        assert!(storico[0].correct);

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

        let verdict = submit(&db, &scope, &item, &Answer::new("ん"), now)
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

            let verdict = submit(&db, &scope, &q.item, &Answer::new(atteso), now)
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
