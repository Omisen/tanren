//! Il giro di una sessione di studio, uguale per tutte le materie.
//!
//! # Perche' adesso e non prima
//!
//! Questo modulo e' nato dentro `features/kana/session.rs` e ci e' rimasto per tutto il
//! tempo in cui i kana erano l'unica materia. Il progetto aveva deciso di non
//! generalizzare prima di avere due casi, perche' generalizzare su uno solo significa
//! indovinare. Coi kanji il secondo caso e' arrivato, e si vede cosa era davvero
//! comune: **la regola del giro**, cioe' chi esce dalla coda, chi rientra e dove.
//!
//! Quello che resta nelle feature e' l'ambito: cosa sia un grado, una famiglia o un
//! sillabario qui non si sa, e non si deve sapere. Una feature calcola i propri item e
//! sceglie il proprio esercizio, e passa di qua solo quei due.
//!
//! # Cos'e' una sessione
//!
//! Un giro completo sull'ambito scelto, mescolato, dove **un item esce dalla coda solo
//! quando lo si indovina**. Chi sbaglia se lo ritrova poco piu' avanti, finche' non lo
//! azzecca. Finito il giro se ne puo' cominciare subito un altro, con un ordine nuovo.
//!
//! # La coda e' un valore, non uno stato
//!
//! Il core non tiene viva nessuna sessione: la coda viene restituita insieme a ogni
//! domanda e l'interfaccia si limita a conservarla e a rimandarla indietro. Non la
//! interpreta e non la modifica. Cosi' lo stato sta dove deve stare, cioe' in qualcosa
//! che muore quando si esce dalla schermata, e la regola sta dove deve stare, cioe'
//! qui, dove un seme fisso la rende verificabile.

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::seq::{IndexedRandom, SliceRandom};
use serde::Serialize;

use crate::shared::error::Result;
use crate::shared::exercise::{Answer, ExerciseType, ItemId, Question, QuestionRequest, Verdict};
use crate::shared::srs::{Grade, Scheduler};
use crate::shared::storage::{Card, Database, NewAnswer, Scheduling};

/// Quante opzioni mostrare nella scelta multipla, risposta giusta compresa.
pub const CHOICES: usize = 4;

/// Dopo quanti altri item torna quello sbagliato.
///
/// Deve tornare presto, perche' la correzione va fissata mentre l'errore e' ancora
/// fresco. Ma non subito: rispondere di nuovo un istante dopo aver letto la soluzione
/// non e' ricordare, e' copiare. La distanza varia perche' un ritorno a scadenza fissa
/// si impara come ritmo.
const RETRY_GAP: [usize; 3] = [2, 3, 4];

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
///
/// # Perche' mescolato
///
/// L'ordine naturale di una tabella dice qualcosa: per i kana e' il gojuon, per i kanji
/// la frequenza. Seguirlo significherebbe insegnare la sequenza invece degli item,
/// perche' dopo due giri la posizione si ricorda meglio del carattere.
///
/// La casualita' arriva da fuori, come per i distrattori: i test fissano un seme e
/// ottengono sempre lo stesso giro.
pub fn start(pool: &[ItemId], exercise: &dyn ExerciseType, rng: &mut dyn Rng) -> Result<Step> {
    let mut queue = pool.to_vec();
    queue.shuffle(rng);
    open(pool, exercise, queue, rng)
}

/// Come continua il giro dopo una risposta.
///
/// Indovinato, l'item esce dalla coda. Sbagliato, ci rientra qualche domanda piu'
/// avanti: e' l'unico modo di uscirne, quindi una sessione finisce solo quando ogni
/// item e' stato azzeccato almeno una volta.
pub fn advance(
    pool: &[ItemId],
    exercise: &dyn ExerciseType,
    queue: &[ItemId],
    correct: bool,
    rng: &mut dyn Rng,
) -> Result<Step> {
    open(pool, exercise, requeue(queue, correct, rng), rng)
}

/// Formula la domanda in cima alla coda, se ce n'e' una.
///
/// E' pura e non tocca il database, quindi e' verificabile con un seme fisso. Serve
/// anche tecnicamente: il generatore di numeri casuali non deve attraversare un'attesa
/// asincrona, altrimenti il future non e' `Send` e Tauri non lo accetta.
fn open(
    pool: &[ItemId],
    exercise: &dyn ExerciseType,
    queue: Vec<ItemId>,
    rng: &mut dyn Rng,
) -> Result<Step> {
    let question = match queue.first() {
        Some(item) => Some(exercise.question(
            QuestionRequest {
                item,
                pool,
                distractors: CHOICES - 1,
            },
            rng,
        )?),
        None => None,
    };

    Ok(Step { question, queue })
}

/// Toglie dalla coda l'item appena chiesto, e lo rimette dentro se e' andato male.
fn requeue(queue: &[ItemId], correct: bool, rng: &mut dyn Rng) -> Vec<ItemId> {
    let mut rest = queue.to_vec();
    if rest.is_empty() {
        return rest;
    }

    let asked = rest.remove(0);
    if !correct {
        // `choose` torna un'opzione perche' una fetta puo' essere vuota, e questa e'
        // una costante di tre elementi. Se davanti non c'e' abbastanza roba l'item
        // finisce in fondo: piu' lontano di cosi' non si puo' metterlo.
        let gap = RETRY_GAP.choose(rng).copied().unwrap_or(RETRY_GAP[0]);
        let at = gap.min(rest.len());
        rest.insert(at, asked);
    }

    rest
}

/// Corregge una risposta e la registra nello storico.
///
/// # Chi pianifica e chi no
///
/// `scheduler` a `None` vuol dire **non toccare la carta**: la risposta entra solo
/// nello storico, che e' in sola aggiunta, e `rating` resta vuoto perche' nessuno ha
/// dato un giudizio. E' il caso dei kana, dove la ripetizione spaziata lavora contro
/// chi impara (vedi [`crate::shared::srs`]), ed e' anche il caso del Drill, che e'
/// esercizio in piu' e non deve spostare le scadenze di niente.
///
/// Con uno scheduler invece la carta si aggiorna: si legge lo stato di memoria che
/// aveva, si calcola quello nuovo e si scrive tutto **nella stessa transazione** della
/// risposta.
///
/// # Il tempo di risposta
///
/// `response_time_ms` e' quanto e' passato da quando la domanda e' comparsa a quando
/// l'utente ha risposto. Lo misura l'interfaccia, perche' e' l'unica a sapere quando la
/// domanda e' comparsa davvero, e arriva qui per essere solo registrato: **non entra
/// nel giudizio**, che resta giusto o sbagliato.
pub async fn submit(
    db: &Database,
    exercise: &dyn ExerciseType,
    item: &ItemId,
    answer: &Answer,
    response_time_ms: Option<i64>,
    scheduler: Option<&Scheduler>,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    let exercise_id = exercise.id();
    let verdict = exercise.grade(item, answer)?;
    let correct = verdict.is_correct();

    // Un rilievo sulla grafia non cambia il voto: chi ha ricordato la lettura ha
    // ricordato, e dire il contrario a FSRS falserebbe l'unico dato da cui impara.
    let scheduling = match scheduler {
        None => None,
        Some(scheduler) => {
            let card = db.card(item.as_str(), exercise_id.as_str()).await?;
            let grade = Grade::from_correct(correct);
            let next = scheduler.schedule(
                card.as_ref().and_then(Card::memory),
                card.as_ref().and_then(|c| c.last_reviewed_at),
                grade,
                now,
            )?;
            Some(Scheduling { grade, next })
        }
    };

    db.record_answer(NewAnswer {
        item_id: item.as_str(),
        exercise_type: exercise_id.as_str(),
        correct,
        answer: answer.as_str(),
        answered_at: now,
        response_time_ms,
        scheduling,
    })
    .await?;

    Ok(verdict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::exercise::{AnswerFormat, ExerciseTypeId, Prompt};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Un esercizio finto: la regola del giro si prova senza tirare dentro una
    /// materia, che e' esattamente il punto di averla spostata qui.
    struct Finto;

    impl Finto {
        const ID: ExerciseTypeId = ExerciseTypeId::new("finto");
    }

    impl ExerciseType for Finto {
        fn id(&self) -> ExerciseTypeId {
            Self::ID
        }

        fn question(&self, request: QuestionRequest<'_>, _rng: &mut dyn Rng) -> Result<Question> {
            Ok(Question {
                exercise_type: Self::ID,
                item: request.item.clone(),
                prompt: Prompt::Latin(request.item.to_string()),
                format: AnswerFormat::Input,
                asks: None,
            })
        }

        fn grade(&self, _item: &ItemId, _answer: &Answer) -> Result<Verdict> {
            Ok(Verdict::correct())
        }
    }

    fn pool(n: usize) -> Vec<ItemId> {
        (0..n).map(|i| ItemId::new(format!("finto:{i}"))).collect()
    }

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    #[test]
    fn il_giro_copre_l_ambito_una_volta_sola() {
        let items = pool(20);
        let step = start(&items, &Finto, &mut rng()).unwrap();

        assert_eq!(step.queue.len(), 20);
        let unici: std::collections::HashSet<_> = step.queue.iter().collect();
        assert_eq!(unici.len(), 20, "nessun item chiesto due volte");
        assert_eq!(
            step.question.unwrap().item,
            step.queue[0],
            "si chiede sempre il primo della coda"
        );
    }

    #[test]
    fn l_ordine_non_e_quello_della_tabella() {
        let items = pool(30);
        let step = start(&items, &Finto, &mut rng()).unwrap();
        assert_ne!(step.queue, items, "il giro va mescolato");
    }

    #[test]
    fn indovinare_toglie_dalla_coda() {
        let items = pool(10);
        let primo = start(&items, &Finto, &mut rng()).unwrap();
        let dopo = advance(&items, &Finto, &primo.queue, true, &mut rng()).unwrap();

        assert_eq!(dopo.queue.len(), 9);
        assert!(!dopo.queue.contains(&primo.queue[0]), "l'item indovinato esce");
    }

    #[test]
    fn sbagliare_rimette_in_coda_poco_piu_avanti() {
        let items = pool(10);
        let primo = start(&items, &Finto, &mut rng()).unwrap();
        let sbagliato = primo.queue[0].clone();
        let dopo = advance(&items, &Finto, &primo.queue, false, &mut rng()).unwrap();

        assert_eq!(dopo.queue.len(), 10, "la coda non si accorcia");
        let posizione = dopo.queue.iter().position(|i| *i == sbagliato).unwrap();
        assert!(
            (2..=4).contains(&posizione),
            "torna dopo 2, 3 o 4 altri item, non subito e non fra molto: era {posizione}"
        );
    }

    #[test]
    fn con_pochi_item_davanti_il_ritentativo_va_in_fondo() {
        // Con due soli item in coda non c'e' spazio per il distacco previsto: piu'
        // lontano della fine non si puo' mettere.
        let items = pool(2);
        let primo = start(&items, &Finto, &mut rng()).unwrap();
        let sbagliato = primo.queue[0].clone();
        let dopo = advance(&items, &Finto, &primo.queue, false, &mut rng()).unwrap();

        assert_eq!(dopo.queue.last(), Some(&sbagliato));
    }

    #[test]
    fn il_giro_finisce_senza_domanda() {
        let items = pool(1);
        let primo = start(&items, &Finto, &mut rng()).unwrap();
        let dopo = advance(&items, &Finto, &primo.queue, true, &mut rng()).unwrap();

        assert!(dopo.queue.is_empty());
        assert_eq!(dopo.question, None, "niente coda, niente domanda");
    }

    #[test]
    fn un_ambito_vuoto_non_produce_domande() {
        let step = start(&[], &Finto, &mut rng()).unwrap();
        assert!(step.queue.is_empty());
        assert_eq!(step.question, None);
    }

    #[tokio::test]
    async fn la_risposta_finisce_nello_storico_e_non_nelle_carte() {
        let db = Database::in_memory().await.unwrap();
        let item = ItemId::new("finto:0");

        let verdict = submit(
            &db,
            &Finto,
            &item,
            &Answer::new("qualcosa"),
            Some(1_200),
            None,
            Utc::now(),
        )
        .await
        .unwrap();

        assert_eq!(verdict, Verdict::correct());

        let storico = db.answers(item.as_str(), "finto").await.unwrap();
        assert_eq!(storico.len(), 1);
        assert_eq!(storico[0].response_time_ms, Some(1_200));

        // Nessuna carta: senza pianificazione non c'e' stato da tenere.
        assert_eq!(db.card(item.as_str(), "finto").await.unwrap(), None);
    }

    #[tokio::test]
    async fn con_uno_scheduler_la_carta_nasce_e_si_aggiorna() {
        let db = Database::in_memory().await.unwrap();
        let scheduler = Scheduler::default();
        let item = ItemId::new("finto:0");
        let now = Utc::now();

        let prima = submit(
            &db,
            &Finto,
            &item,
            &Answer::new("qualcosa"),
            None,
            Some(&scheduler),
            now,
        )
        .await
        .unwrap();
        assert!(prima.is_correct());

        let carta = db
            .card(item.as_str(), "finto")
            .await
            .unwrap()
            .expect("con lo scheduler la carta nasce");
        assert_eq!(carta.reps, 1);
        let memoria = carta.memory().expect("ha uno stato di memoria");
        assert!(memoria.stability > 0.0);
        assert!(carta.due_at.unwrap() > now, "e' stata programmata nel futuro");

        // Alla seconda risposta si riparte da li' e la stabilita' cresce.
        let dopo_un_giorno = carta.due_at.unwrap();
        submit(
            &db,
            &Finto,
            &item,
            &Answer::new("qualcosa"),
            None,
            Some(&scheduler),
            dopo_un_giorno,
        )
        .await
        .unwrap();

        let carta = db.card(item.as_str(), "finto").await.unwrap().unwrap();
        assert_eq!(carta.reps, 2);
        assert!(
            carta.memory().unwrap().stability > memoria.stability,
            "ricordare allunga l'intervallo"
        );
    }
}
