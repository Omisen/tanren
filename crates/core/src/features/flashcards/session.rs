//! Il giro di studio su un mazzo.
//!
//! # Perche' non passa da `shared::session`
//!
//! Per la stessa ragione per cui le flashcard non implementano `ExerciseType`: quel
//! giro chiede una funzione di ricerca che restituisce un esercizio `&'static`, cioe'
//! senza stato, e formula la domanda **senza toccare il database**. Qui la domanda e'
//! una riga scritta dall'utente, e per leggerla ci vuole un'attesa.
//!
//! Quello che si riusa sono i **tipi** che attraversano il confine, `Task` e `Step`
//! compresi, e la **regola del ritentativo**, che e' [`session::requeue`]: chi sbaglia
//! torna dopo due, tre o quattro altre carte, e quella distanza e' scritta in un posto
//! solo per tutte le materie.
//!
//! # Cosa porta la coda, e cosa no
//!
//! Solo gli identificatori. Il testo delle carte resta nel database e si legge una
//! riga per volta, quella che si sta per chiedere: cosi' la coda che va e torna
//! dall'interfaccia resta piccola, e il contenuto ha una fonte sola. Vale la regola
//! gia' scritta per le altre materie: **la coda e' un valore opaco**.
//!
//! # Le due modalita', e perche' non e' una scelta dell'utente
//!
//! [`Mode::Review`] quando c'e' qualcosa di dovuto, [`Mode::Practice`] quando non c'e'
//! niente. Non si sceglie: si guarda cosa c'e' e si fa quello, perche' sono la
//! risposta a due domande diverse e una sola delle due ha senso in un dato momento.
//! E' la regola gia' decisa nella sezione 3 di CLAUDE.md, applicata qui:
//!
//! - **una sessione deve essere sempre avviabile**, quindi un mazzo senza niente di
//!   dovuto si puo' comunque ripassare;
//! - **il ripasso in anticipo non deve corrompere la calibrazione di FSRS**, quindi
//!   quel ripasso non tocca le scadenze. La risposta finisce lo stesso nello storico,
//!   che e' in sola aggiunta.
//!
//! # Perche' la modalita' viaggia e non si ricalcola
//!
//! Perche' si decide **all'inizio** e deve valere per tutto il giro. Ricalcolarla a
//! ogni risposta la farebbe cambiare sotto i piedi: una carta sbagliata torna dovuta
//! fra un minuto, quindi un giro di Practice diventerebbe un Review a meta' strada, e
//! i voti comincerebbero a pesare su carte che nessuno aveva chiesto di consolidare.
//! Il core non tiene viva nessuna sessione, quindi la modalita' la conserva chi la
//! sta facendo, esattamente come la coda.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::features::flashcards::deck;
use crate::features::flashcards::exercise::{self, Direction};
use crate::features::flashcards::steps::{self, Steps};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{Answer, ItemId, Verdict};
use crate::shared::session::{self, Retry, Step, Task};
use crate::shared::srs::{Grade, Scheduled, Scheduler};
use crate::shared::storage::{Card, CardFilter, Database, NewAnswer, Scheduling};

/// Cosa si sta studiando: quale mazzo, e in che verso.
///
/// La direzione si sceglie **prima di cominciare** e vale per tutto il giro.
/// Riconoscere e produrre sono due abilita' diverse, e poter decidere di allenare solo
/// la produzione, che e' la piu' difficile, e' proprio il motivo per cui la scelta
/// esiste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub deck: String,
    pub direction: Direction,
}

/// In che modo si sta studiando questo mazzo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// C'e' qualcosa di dovuto: si ripassa quello, e i voti spostano le scadenze.
    Review,
    /// Non e' dovuto niente: si ripassa tutto il mazzo, e l'algoritmo non se ne accorge.
    Practice,
}

impl Mode {
    /// Se i voti di questo giro arrivano a FSRS.
    fn schedules(self) -> bool {
        self == Self::Review
    }
}

/// Un giro pronto da cominciare.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub tasks: Vec<Task>,
}

/// Cosa si troverebbe partendo adesso.
///
/// Serve a dire cosa fa il tasto di avvio **prima** di premerlo: ripassare tre carte e
/// rifare tutto il mazzo sono due cose diverse, e chi studia ha diritto di saperlo
/// prima.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    /// Quante carte sono dovute adesso, mai viste comprese.
    pub due: usize,
    /// Quante carte ha il mazzo in tutto.
    pub total: usize,
}

/// Cosa si troverebbe partendo adesso.
pub async fn available(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<Available> {
    let (due, all) = split(db, scope, now).await?;
    Ok(Available {
        due: due.len(),
        total: all.len(),
    })
}

/// Sceglie cosa mettere in coda, e quindi anche in che modalita' si sta per studiare.
///
/// Tocca il database, quindi e' asincrona e non puo' mescolare: mescolare vuole il
/// generatore di numeri casuali, che non deve attraversare un'attesa, altrimenti il
/// future non e' `Send` e Tauri non lo accetta. E' lo stesso taglio fra `plan` e
/// `start` che hanno i kanji.
pub async fn plan(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<Plan> {
    let (due, all) = split(db, scope, now).await?;

    Ok(if due.is_empty() {
        Plan {
            mode: Mode::Practice,
            tasks: all,
        }
    } else {
        Plan {
            mode: Mode::Review,
            tasks: due,
        }
    })
}

/// Le carte dovute e quelle del mazzo intero, in una lettura sola.
///
/// Una carta e' dovuta se non e' mai stata studiata (non ha ancora una riga, o ce l'ha
/// con `due_at` a NULL) oppure se la sua scadenza e' passata. Le due liste si ricavano
/// insieme perche' la fonte e' la stessa, e chiederla due volte vorrebbe dire due giri
/// nel database per la stessa risposta.
async fn split(db: &Database, scope: &Scope, now: DateTime<Utc>) -> Result<(Vec<Task>, Vec<Task>)> {
    let cards = deck::cards(db, &scope.deck).await?;
    let exercise = scope.direction.exercise_id();

    let ids: Vec<String> = cards
        .iter()
        .map(|c| deck::item_id(&c.id).as_str().to_owned())
        .collect();
    let studiate = db
        .cards(CardFilter {
            items: Some(&ids),
            exercise_type: Some(exercise.as_str()),
        })
        .await?;
    let per_id: HashMap<&str, &Card> = studiate.iter().map(|c| (c.item_id.as_str(), c)).collect();

    let mut due = Vec::new();
    let mut all = Vec::with_capacity(cards.len());

    for card in &cards {
        let id = deck::item_id(&card.id);
        let dovuta = per_id
            .get(id.as_str())
            .is_none_or(|c| c.due_at.is_none_or(|scadenza| scadenza <= now));

        let task = Task::new(id, exercise.clone());
        if dovuta {
            due.push(task.clone());
        }
        all.push(task);
    }

    Ok((due, all))
}

/// Mescola il giro.
///
/// L'ordine dell'archivio e' quello in cui le carte sono state scritte, e seguirlo
/// insegnerebbe la sequenza invece delle carte: e' la stessa ragione per cui i kana
/// non escono nell'ordine del gojuon. La casualita' arriva da fuori, cosi' i test
/// fissano un seme e ottengono sempre lo stesso giro.
pub fn shuffle(mut tasks: Vec<Task>, rng: &mut dyn Rng) -> Vec<Task> {
    tasks.shuffle(rng);
    tasks
}

/// Toglie dalla coda la carta appena chiesta, e la rimette dentro se e' andata male.
///
/// La regola e' quella condivisa: chi sbaglia torna dopo due, tre o quattro altre
/// carte. Presto, perche' la correzione va fissata mentre l'errore e' fresco; non
/// subito, perche' rispondere un istante dopo aver letto la soluzione e' copiare.
pub fn requeue(queue: &[Task], correct: bool, rng: &mut dyn Rng) -> Vec<Task> {
    session::requeue(queue, correct, Retry::UntilRight, rng)
}

/// Apre la domanda in cima alla coda, se c'e' ancora qualcosa da chiedere.
///
/// Una carta che non si risolve piu' viene **saltata in silenzio**: la coda e' in mano
/// all'interfaccia e puo' nominare una carta cancellata nel frattempo. Far fallire il
/// giro per una riga sparita sarebbe la reazione sbagliata, ed e' la stessa difesa che
/// i kanji hanno su una carta rimasta indietro.
pub async fn open(db: &Database, scope: &Scope, queue: Vec<Task>) -> Result<Step> {
    let mut queue = queue;

    while let Some(task) = queue.first() {
        let id = card_id(&task.item);
        match deck::card(db, id).await? {
            Some(card) => {
                return Ok(Step {
                    question: Some(exercise::question(&card, scope.direction)),
                    queue,
                });
            }
            None => {
                queue.remove(0);
            }
        }
    }

    Ok(Step {
        question: None,
        queue,
    })
}

/// Corregge una risposta **senza registrare niente**.
///
/// Serve perche' il voto arriva dopo: l'utente deve prima sapere se ha indovinato, e
/// solo allora puo' dire quanto gli e' costato. Registrare qui vorrebbe dire scrivere
/// la riga prima di conoscere il voto e poi tornarci sopra, e `answers` e' in sola
/// aggiunta proprio per non doverlo mai fare.
///
/// La conseguenza da sapere: **una risposta lasciata a meta' non viene registrata**.
/// Chi esce dopo aver risposto ma prima di andare avanti la butta via, che e' la stessa
/// cosa che gia' succede al giro intero.
pub async fn check(
    db: &Database,
    scope: &Scope,
    item: &ItemId,
    answer: &Answer,
) -> Result<Verdict> {
    let card = card_of(db, item).await?;
    Ok(exercise::grade(&card, scope.direction, answer))
}

/// Una risposta chiusa: cosa si e' risposto, con che voto, e in quanto tempo.
///
/// Sta insieme perche' e' una cosa sola, e perche' passarne i pezzi sciolti voleva
/// dire una firma con nove argomenti in fila, dove scambiarne due di posto non se ne
/// accorge nessuno.
#[derive(Debug, Clone, Copy)]
pub struct Answered<'a> {
    pub item: &'a ItemId,
    pub answer: &'a Answer,
    /// Il voto scelto da chi ha indovinato. Su una risposta sbagliata non conta.
    pub grade: Option<Grade>,
    /// Da quando la domanda e' comparsa a quando si e' risposto. Lo misura chi mostra.
    pub response_time_ms: Option<i64>,
}

/// Registra la risposta e, in Review, sposta la scadenza.
///
/// # Chi sceglie il voto
///
/// Su una risposta **giusta** lo sceglie l'utente fra Hard, Good ed Easy: il sistema
/// non puo' sapere se ha risposto a fatica o al volo, e dedurlo dal tempo immetterebbe
/// rumore nell'unico dato da cui l'algoritmo impara (sezione 3 di CLAUDE.md). Manca il
/// voto e vale `Good`, che e' la risposta normale.
///
/// Su una risposta **sbagliata** non lo sceglie nessuno: vale `Again`, e questa riga e'
/// il motivo per cui l'interfaccia non mostra nessun bottone. Un voto che arrivasse
/// diverso viene ignorato, perche' la regola e' del dominio e non di chi la chiama.
pub async fn submit(
    db: &Database,
    scope: &Scope,
    mode: Mode,
    answered: Answered<'_>,
    steps: &Steps,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    let Answered {
        item,
        answer,
        grade,
        response_time_ms,
    } = answered;

    let card = card_of(db, item).await?;
    let verdict = exercise::grade(&card, scope.direction, answer);
    let correct = verdict.is_correct();
    let exercise_type = scope.direction.exercise_id();

    let scheduling = if mode.schedules() {
        let grade = if correct {
            grade.unwrap_or(Grade::Good)
        } else {
            Grade::Again
        };

        let studio = db.card(item.as_str(), exercise_type.as_str()).await?;
        let motore = Scheduler::default().schedule(
            studio.as_ref().and_then(Card::memory),
            studio.as_ref().and_then(|c| c.last_reviewed_at),
            grade,
            now,
        )?;

        // Lo stato di memoria e' sempre quello di FSRS: i passi brevi spostano **solo
        // quando** si rivede, non quanto l'algoritmo crede che il ricordo regga.
        let due_at = steps.due_at(grade, steps::learning(studio.as_ref()), motore.due_at, now);

        Some(Scheduling {
            grade,
            next: Scheduled {
                memory: motore.memory,
                due_at,
                interval_days: (due_at - now).num_seconds() as f32 / 86_400.0,
            },
        })
    } else {
        None
    };

    db.record_answer(NewAnswer {
        item_id: item.as_str(),
        exercise_type: exercise_type.as_str(),
        correct,
        answer: answer.as_str(),
        answered_at: now,
        response_time_ms,
        scheduling,
    })
    .await?;

    Ok(verdict)
}

/// La carta dietro un identificatore di studio, o l'errore se non c'e' piu'.
async fn card_of(db: &Database, item: &ItemId) -> Result<deck::Flashcard> {
    deck::card(db, card_id(item))
        .await?
        .ok_or_else(|| CoreError::UnknownItem {
            id: item.to_string(),
        })
}

/// L'identificatore della carta dentro quello di studio.
///
/// `flashcard:<uuid>` ha due segmenti e il secondo e' l'UUID. Se il prefisso non c'e',
/// la stringa non e' nostra e non risolvera' nessuna carta, che e' esattamente cosa
/// deve succedere.
fn card_id(item: &ItemId) -> &str {
    item.as_str().strip_prefix("flashcard:").unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::flashcards::deck::Deck;
    use chrono::TimeDelta;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn adesso() -> DateTime<Utc> {
        "2026-03-15T08:00:00Z".parse().expect("istante valido")
    }

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    async fn mazzo(db: &Database, quante: usize) -> Deck {
        let deck = deck::create_deck(db, "N5", adesso()).await.unwrap();
        for i in 0..quante {
            deck::create_card(
                db,
                &deck.id,
                &format!("かーど{i}"),
                &format!("card {i}"),
                adesso(),
            )
            .await
            .unwrap();
        }
        deck
    }

    fn scope(deck: &Deck, direction: Direction) -> Scope {
        Scope {
            deck: deck.id.clone(),
            direction,
        }
    }

    /// La risposta giusta alla carta indicata, nel verso del significato.
    fn giusta(task: &Task, cards: &[deck::Flashcard]) -> Answer {
        let id = card_id(&task.item);
        let card = cards.iter().find(|c| c.id == id).expect("la carta c'e'");
        Answer::new(card.meaning.clone())
    }

    /* --- cosa entra nel giro ---------------------------------------------- */

    #[tokio::test]
    async fn un_mazzo_mai_studiato_e_tutto_dovuto() {
        let db = db().await;
        let deck = mazzo(&db, 5).await;
        let plan = plan(&db, &scope(&deck, Direction::JpToMeaning), adesso())
            .await
            .unwrap();

        assert_eq!(plan.mode, Mode::Review, "mai visto vuol dire dovuto");
        assert_eq!(plan.tasks.len(), 5);
    }

    #[tokio::test]
    async fn senza_niente_di_dovuto_si_ripassa_e_basta() {
        let db = db().await;
        let deck = mazzo(&db, 3).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let cards = deck::cards(&db, &deck.id).await.unwrap();
        let steps = Steps::default();

        // Si studia tutto una volta: da li' in poi le scadenze sono nel futuro.
        for task in plan(&db, &scope, adesso()).await.unwrap().tasks {
            submit(
                &db,
                &scope,
                Mode::Review,
                Answered {
                    item: &task.item,
                    answer: &giusta(&task, &cards),
                    grade: Some(Grade::Easy),
                    response_time_ms: None,
                },
                &steps,
                adesso(),
            )
            .await
            .unwrap();
        }

        let dopo = plan(&db, &scope, adesso()).await.unwrap();
        assert_eq!(dopo.mode, Mode::Practice);
        assert_eq!(dopo.tasks.len(), 3, "si ripassa il mazzo intero");

        let quanto = available(&db, &scope, adesso()).await.unwrap();
        assert_eq!(quanto, Available { due: 0, total: 3 });
    }

    #[tokio::test]
    async fn le_due_direzioni_scadono_per_conto_loro() {
        let db = db().await;
        let deck = mazzo(&db, 2).await;
        let avanti = scope(&deck, Direction::JpToMeaning);
        let indietro = scope(&deck, Direction::MeaningToJp);
        let cards = deck::cards(&db, &deck.id).await.unwrap();

        for task in plan(&db, &avanti, adesso()).await.unwrap().tasks {
            submit(
                &db,
                &avanti,
                Mode::Review,
                Answered {
                    item: &task.item,
                    answer: &giusta(&task, &cards),
                    grade: Some(Grade::Easy),
                    response_time_ms: None,
                },
                &Steps::default(),
                adesso(),
            )
            .await
            .unwrap();
        }

        assert_eq!(
            available(&db, &avanti, adesso()).await.unwrap().due,
            0,
            "questo verso e' a posto"
        );
        assert_eq!(
            available(&db, &indietro, adesso()).await.unwrap().due,
            2,
            "l'altro non e' stato toccato"
        );
    }

    /* --- come gira la coda ------------------------------------------------ */

    #[tokio::test]
    async fn si_chiede_sempre_la_prima_della_coda() {
        let db = db().await;
        let deck = mazzo(&db, 5).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let tasks = plan(&db, &scope, adesso()).await.unwrap().tasks;
        let step = open(&db, &scope, shuffle(tasks, &mut rng())).await.unwrap();

        assert_eq!(
            step.question.expect("c'e' da chiedere").item,
            step.queue[0].item
        );
    }

    #[tokio::test]
    async fn indovinare_toglie_dalla_coda_e_sbagliare_ci_rimette() {
        let db = db().await;
        let deck = mazzo(&db, 8).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let queue = plan(&db, &scope, adesso()).await.unwrap().tasks;
        let chiesta = queue[0].clone();

        let dopo = requeue(&queue, true, &mut rng());
        assert_eq!(dopo.len(), 7);
        assert!(!dopo.contains(&chiesta));

        let dopo = requeue(&queue, false, &mut rng());
        assert_eq!(dopo.len(), 8, "la coda non si accorcia");
        let posizione = dopo.iter().position(|t| *t == chiesta).unwrap();
        assert!(
            (2..=4).contains(&posizione),
            "torna poco piu' avanti: era {posizione}"
        );
    }

    #[tokio::test]
    async fn una_carta_cancellata_mentre_la_coda_era_in_giro_si_salta() {
        let db = db().await;
        let deck = mazzo(&db, 3).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let queue = plan(&db, &scope, adesso()).await.unwrap().tasks;

        let sparita = card_id(&queue[0].item).to_owned();
        deck::delete_card(&db, &sparita, adesso()).await.unwrap();

        let step = open(&db, &scope, queue).await.unwrap();
        assert_eq!(step.queue.len(), 2, "quella sparita esce dalla coda");
        assert!(
            step.question.is_some(),
            "il giro continua invece di fallire"
        );
    }

    /* --- il voto e la scadenza -------------------------------------------- */

    /// La prima carta del mazzo, col suo compito e la risposta giusta.
    async fn una(db: &Database, scope: &Scope) -> (Task, Answer) {
        let cards = deck::cards(db, &scope.deck).await.unwrap();
        let task = plan(db, scope, adesso()).await.unwrap().tasks[0].clone();
        let answer = giusta(&task, &cards);
        (task, answer)
    }

    async fn carta_di(db: &Database, scope: &Scope, task: &Task) -> Option<Card> {
        db.card(task.item.as_str(), scope.direction.exercise_id().as_str())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn ripassando_il_voto_sposta_la_scadenza() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, answer) = una(&db, &scope).await;

        let verdict = submit(
            &db,
            &scope,
            Mode::Review,
            Answered {
                item: &task.item,
                answer: &answer,
                grade: Some(Grade::Good),
                response_time_ms: Some(900),
            },
            &Steps::default(),
            adesso(),
        )
        .await
        .unwrap();
        assert!(verdict.is_correct());

        let carta = carta_di(&db, &scope, &task).await.expect("la carta nasce");
        assert!(carta.memory().is_some(), "FSRS ha lasciato il suo stato");
        // Prima risposta giusta: comanda il passo breve, non i 2,31 giorni del motore.
        assert_eq!(carta.due_at, Some(adesso() + TimeDelta::minutes(10)));

        let storico = db
            .answers(task.item.as_str(), scope.direction.exercise_id().as_str())
            .await
            .unwrap();
        assert_eq!(storico[0].grade(), Some(Grade::Good));
        assert_eq!(storico[0].response_time_ms, Some(900));
    }

    #[tokio::test]
    async fn sbagliando_il_voto_non_lo_sceglie_nessuno() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, _) = una(&db, &scope).await;

        // Anche passando un voto alto: sbagliata e' sbagliata.
        submit(
            &db,
            &scope,
            Mode::Review,
            Answered {
                item: &task.item,
                answer: &Answer::new("tutt'altro"),
                grade: Some(Grade::Easy),
                response_time_ms: None,
            },
            &Steps::default(),
            adesso(),
        )
        .await
        .unwrap();

        let storico = db
            .answers(task.item.as_str(), scope.direction.exercise_id().as_str())
            .await
            .unwrap();
        assert_eq!(storico[0].grade(), Some(Grade::Again));

        let carta = carta_di(&db, &scope, &task).await.unwrap();
        assert_eq!(
            carta.due_at,
            Some(adesso() + TimeDelta::minutes(1)),
            "deve tornare dentro questa sessione"
        );
        assert_eq!(carta.lapses, 1);
    }

    #[tokio::test]
    async fn graduata_la_carta_passa_al_motore() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, answer) = una(&db, &scope).await;
        let steps = Steps::default();

        let rispondi = async |quando| {
            submit(
                &db,
                &scope,
                Mode::Review,
                Answered {
                    item: &task.item,
                    answer: &answer,
                    grade: Some(Grade::Good),
                    response_time_ms: None,
                },
                &steps,
                quando,
            )
            .await
            .unwrap();
        };

        rispondi(adesso()).await;
        let dopo_il_passo = adesso() + TimeDelta::minutes(10);
        rispondi(dopo_il_passo).await;

        let carta = carta_di(&db, &scope, &task).await.unwrap();
        let attesa = carta.due_at.unwrap() - dopo_il_passo;
        assert!(
            attesa > TimeDelta::days(1),
            "la seconda volta decide FSRS, e parla di giorni: {attesa}"
        );
    }

    #[tokio::test]
    async fn ripassare_in_anticipo_non_tocca_le_scadenze() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, answer) = una(&db, &scope).await;
        let steps = Steps::default();

        submit(
            &db,
            &scope,
            Mode::Review,
            Answered {
                item: &task.item,
                answer: &answer,
                grade: Some(Grade::Easy),
                response_time_ms: None,
            },
            &steps,
            adesso(),
        )
        .await
        .unwrap();
        let prima = carta_di(&db, &scope, &task).await.unwrap();

        // Un giro di Practice sulla stessa carta, subito dopo.
        submit(
            &db,
            &scope,
            Mode::Practice,
            Answered {
                item: &task.item,
                answer: &answer,
                grade: Some(Grade::Easy),
                response_time_ms: None,
            },
            &steps,
            adesso() + TimeDelta::minutes(1),
        )
        .await
        .unwrap();
        let dopo = carta_di(&db, &scope, &task).await.unwrap();

        assert_eq!(prima, dopo, "l'algoritmo non se ne accorge");

        // La risposta pero' c'e', e senza voto.
        let storico = db
            .answers(task.item.as_str(), scope.direction.exercise_id().as_str())
            .await
            .unwrap();
        assert_eq!(storico.len(), 2);
        assert_eq!(storico[1].rating, None, "nessun voto in una pratica");
    }

    #[tokio::test]
    async fn correggere_non_registra_niente() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, answer) = una(&db, &scope).await;

        assert!(
            check(&db, &scope, &task.item, &answer)
                .await
                .unwrap()
                .is_correct()
        );

        assert!(
            db.answers(task.item.as_str(), scope.direction.exercise_id().as_str())
                .await
                .unwrap()
                .is_empty(),
            "una risposta lasciata a meta' non e' una risposta"
        );
        assert_eq!(carta_di(&db, &scope, &task).await, None);
    }

    #[tokio::test]
    async fn una_carta_che_non_esiste_piu_non_si_puo_giudicare() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let (task, answer) = una(&db, &scope).await;
        deck::delete_card(&db, card_id(&task.item), adesso())
            .await
            .unwrap();

        assert!(matches!(
            check(&db, &scope, &task.item, &answer).await,
            Err(CoreError::UnknownItem { .. })
        ));
    }
}
