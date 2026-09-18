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
//! compresi, quindi l'interfaccia non ha dovuto imparare una seconda forma di
//! sessione.
//!
//! # Cosa porta la coda, e cosa no
//!
//! Solo gli identificatori. Il testo delle carte resta nel database e si legge una
//! riga per volta, quella che si sta per chiedere: cosi' la coda che va e torna
//! dall'interfaccia resta piccola, e il contenuto ha una fonte sola. Vale la regola
//! gia' scritta per le altre materie: **la coda e' un valore opaco**, il frontend la
//! conserva e la rimanda indietro senza guardarci dentro.
//!
//! # Cosa non c'e' ancora
//!
//! La ripetizione spaziata. Qui un giro passa una volta sola su ogni carta del mazzo e
//! la risposta finisce solo nello storico, con `scheduling` a `None`, esattamente come
//! sui kana. I voti a quattro gradini, il ritorno della carta sbagliata e le scadenze
//! arrivano col passo successivo.

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::features::flashcards::deck;
use crate::features::flashcards::exercise::{self, Direction};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{Answer, ItemId, Verdict};
use crate::shared::session::{Step, Task};
use crate::shared::storage::{Database, NewAnswer};

/// Cosa si sta studiando: quale mazzo, e in che verso.
///
/// La direzione si sceglie **prima di cominciare** e vale per tutto il giro. Non e' un
/// dettaglio di comodo: riconoscere e produrre sono due abilita' diverse, e poter
/// decidere di allenare solo la produzione, che e' la piu' difficile, e' proprio il
/// motivo per cui la scelta esiste. Mescolarle darebbe un giro lungo il doppio che
/// nessuno ha chiesto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub deck: String,
    pub direction: Direction,
}

/// Le carte del mazzo, nell'ordine in cui stanno nell'archivio.
///
/// Tocca il database, quindi e' asincrona e non puo' mescolare: mescolare vuole il
/// generatore di numeri casuali, che non deve attraversare un'attesa, altrimenti il
/// future non e' `Send` e Tauri non lo accetta. E' lo stesso taglio fra `plan` e
/// `start` che hanno i kanji.
pub async fn plan(db: &Database, scope: &Scope) -> Result<Vec<Task>> {
    let cards = deck::cards(db, &scope.deck).await?;
    let exercise = scope.direction.exercise_id();

    Ok(cards
        .into_iter()
        .map(|c| Task::new(deck::item_id(&c.id), exercise.clone()))
        .collect())
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

/// Come continua il giro dopo una risposta.
///
/// La carta appena chiesta esce dalla coda e non torna: **un giro passa una volta sola
/// su ogni carta**. Il ritorno di quella sbagliata non e' una dimenticanza, e' il
/// mestiere dei voti e delle scadenze, che arrivano col passo successivo.
pub async fn advance(db: &Database, scope: &Scope, queue: &[Task]) -> Result<Step> {
    let mut rest = queue.to_vec();
    if !rest.is_empty() {
        rest.remove(0);
    }
    open(db, scope, rest).await
}

/// Corregge una risposta e la registra nello storico.
///
/// `scheduling` e' `None`, quindi la risposta entra in `answers` e nessuna carta di
/// studio viene toccata: lo stesso ramo che usano i kana. E' il punto in cui il passo
/// successivo attacchera' FSRS.
///
/// `response_time_ms` lo misura l'interfaccia, perche' e' l'unica a sapere quando la
/// domanda e' comparsa davvero. Qui si registra e basta: **non entra nel giudizio**.
pub async fn submit(
    db: &Database,
    scope: &Scope,
    item: &ItemId,
    answer: &Answer,
    response_time_ms: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Verdict> {
    let card = deck::card(db, card_id(item))
        .await?
        .ok_or_else(|| CoreError::UnknownItem {
            id: item.to_string(),
        })?;

    let verdict = exercise::grade(&card, scope.direction, answer);
    let exercise_type = scope.direction.exercise_id();

    db.record_answer(NewAnswer {
        item_id: item.as_str(),
        exercise_type: exercise_type.as_str(),
        correct: verdict.is_correct(),
        answer: answer.as_str(),
        answered_at: now,
        response_time_ms,
        scheduling: None,
    })
    .await?;

    Ok(verdict)
}

/// L'identificatore della carta dentro quello di studio.
///
/// `flashcard:<uuid>` ha due segmenti e il secondo e' l'UUID: quello che non e' il
/// prefisso e' l'identificatore della riga. Se il prefisso non c'e', la stringa non e'
/// nostra e non risolvera' nessuna carta, che e' esattamente cosa deve succedere.
fn card_id(item: &ItemId) -> &str {
    item.as_str().strip_prefix("flashcard:").unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::flashcards::deck::Deck;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn adesso() -> DateTime<Utc> {
        "2026-03-15T08:00:00Z".parse().expect("istante valido")
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

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn il_giro_copre_il_mazzo_una_volta_sola() {
        let db = db().await;
        let deck = mazzo(&db, 8).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let tasks = plan(&db, &scope).await.unwrap();
        assert_eq!(tasks.len(), 8);
        let unici: std::collections::HashSet<_> = tasks.iter().collect();
        assert_eq!(unici.len(), 8, "nessuna carta due volte");
    }

    #[tokio::test]
    async fn il_giro_non_segue_l_ordine_in_cui_le_carte_sono_state_scritte() {
        let db = db().await;
        let deck = mazzo(&db, 20).await;
        let tasks = plan(&db, &scope(&deck, Direction::JpToMeaning))
            .await
            .unwrap();

        assert_ne!(shuffle(tasks.clone(), &mut rng()), tasks, "va mescolato");
    }

    #[tokio::test]
    async fn si_chiede_sempre_la_prima_della_coda() {
        let db = db().await;
        let deck = mazzo(&db, 5).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let queue = shuffle(plan(&db, &scope).await.unwrap(), &mut rng());
        let step = open(&db, &scope, queue).await.unwrap();

        let question = step.question.expect("c'e' da chiedere");
        assert_eq!(question.item, step.queue[0].item);
    }

    #[tokio::test]
    async fn rispondere_toglie_la_carta_dalla_coda() {
        let db = db().await;
        let deck = mazzo(&db, 5).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let primo = open(&db, &scope, plan(&db, &scope).await.unwrap())
            .await
            .unwrap();
        let chiesta = primo.queue[0].clone();
        let dopo = advance(&db, &scope, &primo.queue).await.unwrap();

        assert_eq!(dopo.queue.len(), 4);
        assert!(
            !dopo.queue.contains(&chiesta),
            "non torna dentro questo giro"
        );
    }

    #[tokio::test]
    async fn il_giro_finisce_senza_domanda() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let primo = open(&db, &scope, plan(&db, &scope).await.unwrap())
            .await
            .unwrap();
        let dopo = advance(&db, &scope, &primo.queue).await.unwrap();

        assert!(dopo.queue.is_empty());
        assert_eq!(dopo.question, None);
    }

    #[tokio::test]
    async fn un_mazzo_vuoto_non_produce_domande() {
        let db = db().await;
        let deck = mazzo(&db, 0).await;
        let scope = scope(&deck, Direction::JpToMeaning);

        let step = open(&db, &scope, plan(&db, &scope).await.unwrap())
            .await
            .unwrap();
        assert_eq!(step.question, None);
    }

    #[tokio::test]
    async fn una_carta_cancellata_mentre_la_coda_era_in_giro_si_salta() {
        let db = db().await;
        let deck = mazzo(&db, 3).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let queue = plan(&db, &scope).await.unwrap();

        // La coda e' gia' in mano all'interfaccia quando la prima carta se ne va.
        let sparita = card_id(&queue[0].item).to_owned();
        deck::delete_card(&db, &sparita, adesso()).await.unwrap();

        let step = open(&db, &scope, queue).await.unwrap();
        assert_eq!(step.queue.len(), 2, "quella sparita esce dalla coda");
        assert!(
            step.question.is_some(),
            "il giro continua invece di fallire"
        );
    }

    #[tokio::test]
    async fn la_risposta_finisce_nello_storico_e_non_nelle_carte_di_studio() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let queue = plan(&db, &scope).await.unwrap();
        let item = queue[0].item.clone();

        let verdict = submit(
            &db,
            &scope,
            &item,
            &Answer::new("card 0"),
            Some(1_200),
            adesso(),
        )
        .await
        .unwrap();
        assert!(verdict.is_correct());

        let esercizio = Direction::JpToMeaning.exercise_id();
        let storico = db.answers(item.as_str(), esercizio.as_str()).await.unwrap();
        assert_eq!(storico.len(), 1);
        assert_eq!(storico[0].response_time_ms, Some(1_200));
        assert_eq!(storico[0].rating, None, "nessuno ha dato un voto");

        // Senza pianificazione non nasce nessuna carta di studio: e' il ramo dei kana.
        assert_eq!(
            db.card(item.as_str(), esercizio.as_str()).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn le_due_direzioni_scrivono_su_due_esercizi_diversi() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let item = plan(&db, &scope(&deck, Direction::JpToMeaning))
            .await
            .unwrap()[0]
            .item
            .clone();

        submit(
            &db,
            &scope(&deck, Direction::JpToMeaning),
            &item,
            &Answer::new("card 0"),
            None,
            adesso(),
        )
        .await
        .unwrap();
        submit(
            &db,
            &scope(&deck, Direction::MeaningToJp),
            &item,
            &Answer::new("かーど0"),
            None,
            adesso(),
        )
        .await
        .unwrap();

        for d in [Direction::JpToMeaning, Direction::MeaningToJp] {
            let storico = db
                .answers(item.as_str(), d.exercise_id().as_str())
                .await
                .unwrap();
            assert_eq!(storico.len(), 1, "ogni verso ha il suo storico");
        }
    }

    #[tokio::test]
    async fn una_carta_che_non_esiste_piu_non_si_puo_giudicare() {
        let db = db().await;
        let deck = mazzo(&db, 1).await;
        let scope = scope(&deck, Direction::JpToMeaning);
        let item = plan(&db, &scope).await.unwrap()[0].item.clone();
        deck::delete_card(&db, card_id(&item), adesso())
            .await
            .unwrap();

        let esito = submit(&db, &scope, &item, &Answer::new("x"), None, adesso()).await;
        assert!(matches!(esito, Err(CoreError::UnknownItem { .. })));
    }
}
