//! Le tre modalita' di studio dei kanji.
//!
//! Non sono tre sistemi separati: e' **lo stesso giro** configurato in modo diverso, e
//! le differenze sono tre e sole.
//!
//! | | da dove pesca | rifa' chi sbaglia | nutre FSRS |
//! |---|---|---|---|
//! | [`Mode::Learning`] | kanji mai visti del livello | si' | si', ed e' qui che le carte nascono |
//! | [`Mode::Review`] | faccette scadute, di qualunque livello | **no** | si' |
//! | [`Mode::Drill`] | qualunque faccetta gia' vista | si' | **no** |
//!
//! # Perche' il Review non ripete chi sbaglia
//!
//! Perche' e' FSRS a decidere quando un item torna, e ha gia' deciso nel momento in
//! cui si e' risposto male: rimetterlo in coda vorrebbe dire rispondere due volte alla
//! stessa domanda e contarle tutte e due, falsando l'unico dato da cui l'algoritmo
//! impara. Nel Learning invece la ripetizione ravvicinata e' il punto, perche' li' il
//! ricordo si sta formando adesso.
//!
//! # Perche' il Drill non nutre FSRS
//!
//! Perche' e' esercizio in piu', chiesto da chi vuole macinare ripetizioni senza
//! aspettare. Se contasse, ripassare in anticipo sposterebbe le scadenze e la
//! calibrazione dell'algoritmo si perderebbe. E' la stessa Practice decisa nella
//! sezione 3 di CLAUDE.md, col nome che si legge nell'interfaccia.
//!
//! # Cosa e' pigro e cosa no
//!
//! Scegliere cosa mettere in coda tocca il database, quindi e' asincrono. Formulare la
//! domanda no: e' puro, e deve restarlo, perche' il generatore di numeri casuali non
//! puo' attraversare un'attesa senza rendere il future non `Send`.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::features::kanji::facets::{
    Facet, exercise_for, facet_of, item_id, items, level_of, resolves,
};
use crate::features::kanji::levels::{Level, table};
use crate::features::kanji::progress::{Gate, LevelProgress, Pacing, learning_gate, level_progress};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{Answer, ExerciseTypeId, ItemId, Verdict};
use crate::shared::session::{self, Retry, Step, Task};
use crate::shared::srs::Scheduler;
use crate::shared::storage::{Card, CardFilter, Database};

/// In che modo si sta studiando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Si conoscono kanji nuovi, e da qui in poi FSRS li prende in carico.
    Learning,
    /// Si rivede quello che sta per essere dimenticato. Lo decide FSRS.
    Review,
    /// Si pratica a volonta' su quello che si e' gia' visto, senza conseguenze.
    Drill,
}

impl Mode {
    /// Se una risposta sbagliata torna in coda dentro lo stesso giro.
    fn retry(self) -> Retry {
        match self {
            Self::Learning | Self::Drill => Retry::UntilRight,
            Self::Review => Retry::Once,
        }
    }

    /// Se le risposte spostano le scadenze.
    fn schedules(self) -> bool {
        match self {
            Self::Learning | Self::Review => true,
            Self::Drill => false,
        }
    }
}

/// Cosa si sta per studiare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub mode: Mode,
    /// Il livello su cui si sta lavorando.
    ///
    /// Vincola il Learning e il Drill. Il **Review no**: quello che e' scaduto va
    /// rivisto da qualunque livello venga, altrimenti salire di livello vorrebbe dire
    /// dimenticare quello di prima.
    pub level: Level,
}

/// Un giro pronto da cominciare.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// I kanji che si stanno conoscendo adesso, da presentare prima di interrogare.
    ///
    /// Vuoto fuori dal Learning: nel Review e nel Drill non c'e' niente da presentare,
    /// si e' gia' visto tutto.
    pub introducing: Vec<String>,
    pub tasks: Vec<Task>,
}

/// Cosa si puo' fare adesso, e perche' no.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    pub learning: Gate,
    /// Quante faccette sono scadute, di qualunque livello.
    pub due: usize,
    /// Su quante faccette si puo' praticare.
    pub practiced: usize,
}

/// Cosa si puo' fare adesso.
pub async fn available(
    db: &Database,
    scope: &Scope,
    pacing: &Pacing,
    now: DateTime<Utc>,
) -> Result<Available> {
    Ok(Available {
        learning: learning_gate(db, scope.level, pacing, now).await?,
        due: due_tasks(db, now).await?.len(),
        practiced: seen_tasks(db).await?.len(),
    })
}

/// Sceglie cosa mettere in coda.
///
/// Tocca il database, quindi e' asincrona e non puo' mescolare: mescolare e' compito
/// di [`start`], che e' puro.
pub async fn plan(
    db: &Database,
    scope: &Scope,
    pacing: &Pacing,
    now: DateTime<Utc>,
) -> Result<Plan> {
    match scope.mode {
        Mode::Learning => {
            let Gate::Open { room } = learning_gate(db, scope.level, pacing, now).await? else {
                // Chiedere di imparare quando la porta e' chiusa non e' un errore
                // dell'utente: e' un giro vuoto, e l'interfaccia sa gia' dire perche'.
                return Ok(Plan {
                    introducing: Vec::new(),
                    tasks: Vec::new(),
                });
            };

            let nuovi = new_kanji(db, scope.level, room).await?;
            let tasks = items(scope.level)
                .into_iter()
                .filter(|i| nuovi.iter().any(|k| i.form.starts_with(k.as_str())))
                .map(|i| Task::new(i.id, i.facet.exercise_id()))
                .collect();

            Ok(Plan {
                introducing: nuovi,
                tasks,
            })
        }
        Mode::Review => Ok(Plan {
            introducing: Vec::new(),
            tasks: due_tasks(db, now).await?,
        }),
        Mode::Drill => {
            let mut tasks = seen_tasks(db).await?;
            // Il taglio si fa qui e non mescolando: chi chiama mescola dopo, e cosi'
            // un giro di Drill resta della lunghezza dichiarata.
            tasks.truncate(pacing.drill_size);
            Ok(Plan {
                introducing: Vec::new(),
                tasks,
            })
        }
    }
}

/// I kanji del livello che non sono mai stati introdotti, i piu' frequenti per primi.
async fn new_kanji(db: &Database, level: Level, quanti: usize) -> Result<Vec<String>> {
    let meaning = Facet::Meaning.exercise_id();
    let ids: Vec<String> = table(level)
        .all()
        .iter()
        .map(|k| item_id(&k.character).as_str().to_owned())
        .collect();

    let esistenti = db
        .cards(CardFilter {
            items: Some(&ids),
            exercise_type: Some(meaning.as_str()),
        })
        .await?;

    Ok(table(level)
        .all()
        .iter()
        .filter(|k| {
            let id = item_id(&k.character);
            !esistenti.iter().any(|c| c.item_id == id.as_str())
        })
        .take(quanti)
        .map(|k| k.character.clone())
        .collect())
}

/// Le faccette scadute, le piu' arretrate per prime.
async fn due_tasks(db: &Database, now: DateTime<Utc>) -> Result<Vec<Task>> {
    let cards = db.due_cards(CardFilter::default(), now, i64::MAX).await?;
    Ok(cards.into_iter().filter_map(as_task).collect())
}

/// Tutte le faccette gia' incontrate, cioe' quelle che hanno una carta.
async fn seen_tasks(db: &Database) -> Result<Vec<Task>> {
    let cards = db.cards(CardFilter::default()).await?;
    Ok(cards.into_iter().filter_map(as_task).collect())
}

/// Una carta diventa un compito solo se e' di questa materia **e se esiste ancora**.
///
/// L'archivio non e' diviso per materia, e non deve esserlo: e' qui che si tiene fuori
/// quello che non ci riguarda.
///
/// # Perche' non basta riconoscere la materia
///
/// Perche' una carta scritta mesi fa nomina una forma che il contenuto di oggi
/// potrebbe non avere piu': basta che una rigenerazione tolga una lettura con
/// okurigana, e `kanji:生かす` resta un identificatore ben formato che pero' non si
/// risolve. L'errore risalirebbe fino a far fallire l'avvio del giro. **Una traccia
/// rimasta indietro deve sparire in silenzio, non rompere il Drill.**
///
/// Questa difesa era nata per un'altra ragione, cioe' i kanji che cambiavano livello
/// mentre il livello stava dentro l'identificatore. Quella ragione non c'e' piu',
/// perche' l'identificatore non porta piu' il livello; questa invece resta, e vive di
/// vita propria.
fn as_task(card: Card) -> Option<Task> {
    let exercise = ExerciseTypeId::owned(card.exercise_type);
    exercise_for(&exercise)?;
    let item = ItemId::new(card.item_id);
    level_of(&item)?;

    // Che l'item esista ancora lo chiede il contenuto, non un giudizio inventato.
    facet_of(&exercise).filter(|f| resolves(&item, *f))?;

    Some(Task::new(item, exercise))
}

/// Comincia il giro: la coda mescolata, e la prima domanda.
pub fn start(plan: &Plan, rng: &mut dyn Rng) -> Result<Step> {
    let mut queue = plan.tasks.clone();
    queue.shuffle(rng);
    open(queue, rng)
}

/// Come continua il giro dopo una risposta.
pub fn advance(mode: Mode, queue: &[Task], correct: bool, rng: &mut dyn Rng) -> Result<Step> {
    let pool = pool(queue);
    session::advance(&pool, exercise_for, queue, correct, mode.retry(), rng)
}

fn open(queue: Vec<Task>, rng: &mut dyn Rng) -> Result<Step> {
    let pool = pool(&queue);
    session::open_queue(&pool, exercise_for, queue, rng)
}

/// Fra chi pescare i distrattori: gli item dei livelli che il giro tocca.
///
/// Non la coda, che si accorcia domanda dopo domanda e finirebbe per offrire sempre le
/// stesse alternative; e non tutti i 2.136 joyo, che darebbero opzioni prese da un
/// livello mai visto.
fn pool(queue: &[Task]) -> Vec<Task> {
    let livelli: BTreeSet<Level> = queue.iter().filter_map(|t| level_of(&t.item)).collect();
    livelli
        .into_iter()
        .flat_map(items)
        .map(|i| Task::new(i.id, i.facet.exercise_id()))
        .collect()
}

/// Corregge una risposta e la registra.
///
/// **Il Drill non passa lo scheduler**, quindi non tocca nessuna scadenza: la risposta
/// finisce solo nello storico.
pub async fn submit(
    db: &Database,
    mode: Mode,
    task: &Task,
    answer: &Answer,
    response_time_ms: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    let exercise = exercise_for(&task.exercise).ok_or_else(|| CoreError::ItemNotSupported {
        exercise: task.exercise.to_string(),
        id: task.item.to_string(),
    })?;

    let scheduler = mode.schedules().then(Scheduler::default);
    session::submit(
        db,
        exercise,
        &task.item,
        answer,
        response_time_ms,
        scheduler.as_ref(),
        now,
    )
    .await
}

/// A che punto e' il livello su cui si sta lavorando.
pub async fn progress(db: &Database, scope: &Scope, pacing: &Pacing) -> Result<LevelProgress> {
    level_progress(db, scope.level, pacing).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::kanji::progress::Blocked;
    use crate::shared::srs::{Grade, MemoryState, Scheduled};
    use crate::shared::storage::{NewAnswer, Scheduling};
    use chrono::TimeDelta;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn primo() -> Level {
        Level::new(1).unwrap()
    }

    fn scope(mode: Mode) -> Scope {
        Scope {
            mode,
            level: primo(),
        }
    }

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    /// Un istante fisso, lontano dai confini della giornata: la quota giornaliera si
    /// azzera a mezzanotte UTC, e un test che somma ore all'orologio vero cambierebbe
    /// risposta a seconda di quando lo si lancia.
    fn mattina() -> DateTime<Utc> {
        "2026-03-15T08:00:00Z".parse().expect("istante valido")
    }

    /// Porta tutte le faccette di un kanji alla stabilita' chiesta.
    async fn impara(db: &Database, level: Level, kanji: &str, stability: f32, quando: DateTime<Utc>) {
        for item in items(level)
            .into_iter()
            .filter(|i| i.form.starts_with(kanji))
        {
            let esercizio = item.facet.exercise_id();
            db.record_answer(NewAnswer {
                item_id: item.id.as_str(),
                exercise_type: esercizio.as_str(),
                correct: true,
                answer: "x",
                answered_at: quando,
                response_time_ms: None,
                scheduling: Some(Scheduling {
                    grade: Grade::Good,
                    next: Scheduled {
                        memory: MemoryState {
                            stability,
                            difficulty: 5.0,
                        },
                        due_at: quando + TimeDelta::days(stability as i64),
                        interval_days: stability,
                    },
                }),
            })
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn imparare_presenta_i_kanji_e_ne_mette_in_coda_le_faccette() {
        let db = db().await;
        let pacing = Pacing::default();
        let plan = plan(&db, &scope(Mode::Learning), &pacing, Utc::now())
            .await
            .unwrap();

        assert_eq!(plan.introducing.len(), pacing.daily_new, "quanti ne concede la porta");
        assert!(
            plan.tasks.len() > plan.introducing.len(),
            "ogni kanji vale piu' di una domanda: significato, letture, forme"
        );

        // Tutte le domande parlano dei kanji che si stanno presentando, e di nessun altro.
        for task in &plan.tasks {
            let forma = task.item.as_str().rsplit(':').next().unwrap();
            assert!(
                plan.introducing.iter().any(|k| forma.starts_with(k.as_str())),
                "{forma} non e' fra quelli presentati"
            );
        }
    }

    #[tokio::test]
    async fn imparare_a_porta_chiusa_da_un_giro_vuoto_e_non_un_errore() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = mattina();

        for k in table(primo()).all().iter().take(pacing.daily_new) {
            impara(&db, primo(), &k.character, 40.0, now).await;
        }

        // La quota di oggi e' finita: chiedere di imparare non e' un errore
        // dell'utente, e' semplicemente che non c'e' niente da mettere in coda.
        let dopo = now + pacing.floor;
        assert!(matches!(
            learning_gate(&db, primo(), &pacing, dopo).await.unwrap(),
            Gate::Closed(Blocked::Wait { .. })
        ));
        let plan = plan(&db, &scope(Mode::Learning), &pacing, dopo).await.unwrap();
        assert!(plan.tasks.is_empty());
        assert!(plan.introducing.is_empty());
    }

    #[tokio::test]
    async fn il_ripasso_pesca_solo_cio_che_e_scaduto_e_da_qualunque_livello() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();

        // Uno al primo livello e uno al secondo, entrambi scaduti da un pezzo.
        impara(&db, primo(), &table(primo()).all()[0].character.clone(), 1.0, now - TimeDelta::days(30)).await;
        let secondo = Level::new(2).unwrap();
        impara(&db, secondo, &table(secondo).all()[0].character.clone(), 1.0, now - TimeDelta::days(30)).await;
        // E uno che non e' ancora dovuto.
        impara(&db, primo(), &table(primo()).all()[1].character.clone(), 40.0, now).await;

        let plan = plan(&db, &scope(Mode::Review), &pacing, now).await.unwrap();
        assert!(!plan.tasks.is_empty());
        assert!(plan.introducing.is_empty(), "nel ripasso non si presenta niente");

        let livelli: BTreeSet<Level> = plan.tasks.iter().filter_map(|t| level_of(&t.item)).collect();
        assert!(
            livelli.contains(&secondo),
            "salire di livello non deve far dimenticare quello di prima"
        );

        let non_dovuto = item_id(&table(primo()).all()[1].character);
        assert!(
            !plan.tasks.iter().any(|t| t.item == non_dovuto),
            "cio' che non e' scaduto resta fuori"
        );
    }

    #[tokio::test]
    async fn il_drill_pesca_da_tutto_il_gia_visto_ma_con_un_tetto() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();

        for k in table(primo()).all().iter().take(10) {
            impara(&db, primo(), &k.character, 40.0, now).await;
        }

        let plan = plan(&db, &scope(Mode::Drill), &pacing, now).await.unwrap();
        assert_eq!(plan.tasks.len(), pacing.drill_size, "un giro lungo quanto dichiarato");
    }

    #[tokio::test]
    async fn una_carta_rimasta_indietro_non_rompe_il_giro() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();

        // Qualcosa di buono, per avere un giro da fare.
        let uno = table(primo()).all()[0].character.clone();
        impara(&db, primo(), &uno, 40.0, now).await;

        // E una carta che parla di un kanji che in quel livello non c'e' (piu'): e'
        // quello che resta quando i livelli si rifanno e un kanji si sposta.
        db.record_answer(NewAnswer {
            item_id: "kanji:1:年",
            exercise_type: Facet::Meaning.exercise_id().as_str(),
            correct: true,
            answer: "year",
            answered_at: now,
            response_time_ms: None,
            scheduling: None,
        })
        .await
        .unwrap();

        // Il Drill deve ignorarla e partire lo stesso, non fallire.
        let plan = plan(&db, &scope(Mode::Drill), &pacing, now).await.unwrap();
        assert!(!plan.tasks.is_empty(), "il giro c'e' comunque");
        assert!(
            !plan.tasks.iter().any(|t| t.item.as_str() == "kanji:1:年"),
            "la traccia rimasta indietro sparisce in silenzio"
        );
        assert!(start(&plan, &mut rng()).is_ok(), "e il giro si apre");
    }

    #[tokio::test]
    async fn a_mani_vuote_non_c_e_niente_da_ripassare_ne_da_praticare() {
        let db = db().await;
        let pacing = Pacing::default();
        let a = available(&db, &scope(Mode::Learning), &pacing, Utc::now()).await.unwrap();

        assert_eq!(a.due, 0);
        assert_eq!(a.practiced, 0);
        assert!(matches!(a.learning, Gate::Open { .. }), "si comincia da qui");
    }

    #[tokio::test]
    async fn il_drill_non_sposta_nessuna_scadenza() {
        let db = db().await;
        let now = Utc::now();
        let uno = table(primo()).all()[0].character.clone();
        impara(&db, primo(), &uno, 40.0, now).await;

        let task = Task::new(item_id(&uno), Facet::Meaning.exercise_id());
        let prima = db
            .card(task.item.as_str(), task.exercise.as_str())
            .await
            .unwrap()
            .unwrap();

        submit(&db, Mode::Drill, &task, &Answer::new("qualsiasi"), None, now + TimeDelta::days(1))
            .await
            .unwrap();

        let dopo = db
            .card(task.item.as_str(), task.exercise.as_str())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dopo.due_at, prima.due_at, "la scadenza non si muove");
        assert_eq!(dopo.reps, prima.reps, "e nemmeno il conteggio");

        // Nello storico invece la risposta c'e', perche' quella tabella e' in sola
        // aggiunta e serve a sapere cosa e' successo.
        let storico = db
            .answers(task.item.as_str(), task.exercise.as_str())
            .await
            .unwrap();
        assert_eq!(storico.len(), 2, "quella dell'apprendimento piu' questa");
    }

    #[tokio::test]
    async fn il_ripasso_invece_la_sposta() {
        let db = db().await;
        let now = Utc::now();
        let uno = table(primo()).all()[0].character.clone();
        impara(&db, primo(), &uno, 1.0, now - TimeDelta::days(30)).await;

        let task = Task::new(item_id(&uno), Facet::Meaning.exercise_id());
        let prima = db
            .card(task.item.as_str(), task.exercise.as_str())
            .await
            .unwrap()
            .unwrap();

        let significato = table(primo()).get(&uno).unwrap().meanings[0].clone();
        let verdict = submit(
            &db,
            Mode::Review,
            &task,
            &Answer::new(significato),
            None,
            now,
        )
        .await
        .unwrap();
        assert!(verdict.is_correct());

        let dopo = db
            .card(task.item.as_str(), task.exercise.as_str())
            .await
            .unwrap()
            .unwrap();
        assert!(dopo.due_at.unwrap() > prima.due_at.unwrap());
        assert_eq!(dopo.reps, prima.reps + 1);
    }

    #[test]
    fn nel_ripasso_chi_sbaglia_non_torna_subito() {
        let mut rng = rng();
        let tasks: Vec<Task> = items(primo())
            .into_iter()
            .take(6)
            .map(|i| Task::new(i.id, i.facet.exercise_id()))
            .collect();

        let sbagliato = tasks[0].clone();
        let step = advance(Mode::Review, &tasks, false, &mut rng).unwrap();
        assert_eq!(step.queue.len(), tasks.len() - 1);
        assert!(
            !step.queue.contains(&sbagliato),
            "lo decide FSRS, non il giro: richiederlo adesso conterebbe due risposte"
        );

        // Nel Learning invece torna, perche' li' il ricordo si sta formando adesso.
        let step = advance(Mode::Learning, &tasks, false, &mut rng).unwrap();
        assert_eq!(step.queue.len(), tasks.len());
        assert!(step.queue.contains(&sbagliato));
    }

    #[tokio::test]
    async fn un_giro_si_apre_senza_perdere_domande() {
        let db = db().await;
        let pacing = Pacing::default();
        let plan = plan(&db, &scope(Mode::Learning), &pacing, Utc::now())
            .await
            .unwrap();

        let step = start(&plan, &mut rng()).unwrap();
        assert_eq!(step.queue.len(), plan.tasks.len(), "aprire non consuma niente");
        let q = step.question.expect("c'e' una domanda");
        assert_eq!(q.item, step.queue[0].item);
        assert_eq!(q.exercise_type, step.queue[0].exercise);
    }

    #[tokio::test]
    async fn i_distrattori_vengono_dai_livelli_che_il_giro_tocca() {
        let db = db().await;
        let pacing = Pacing::default();
        let plan = plan(&db, &scope(Mode::Learning), &pacing, Utc::now())
            .await
            .unwrap();

        // Il pool e' tutto il livello, non la sola coda: con cinque kanji in coda le
        // alternative sarebbero sempre le stesse quattro.
        let pool = pool(&plan.tasks);
        assert_eq!(pool.len(), items(primo()).len());
    }
}
