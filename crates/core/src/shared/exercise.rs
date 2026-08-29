//! Il contratto di un tipo di esercizio.
//!
//! Ogni esercizio, in qualunque materia, risponde a due domande: che domanda pongo a
//! partire da un elemento di studio, e come giudico la risposta. Un esercizio nuovo
//! si aggiunge implementando [`ExerciseType`], senza toccare chi lo usa.
//!
//! # Perche' la risposta e' sempre una stringa
//!
//! Le due modalita' dell'app sembrano diverse ma collassano sulla stessa forma. Nel
//! matching l'utente tocca un'opzione, nell'input digita con l'IME: in entrambi i
//! casi quello che torna al core e' del testo. Il matching manda il **valore**
//! dell'opzione scelta, non il suo indice, cosi' la correzione non dipende
//! dall'ordine in cui la UI ha mescolato le scelte, e la stessa funzione di giudizio
//! serve tutte e due le modalita'.
//!
//! # Perche' il core non conserva la domanda
//!
//! Giudicare richiede solo l'elemento e la risposta. Non serve ricordare la domanda
//! posta, quindi non c'e' stato da tenere tra una chiamata e l'altra e nessuna
//! sessione da invalidare.

use std::borrow::Cow;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::shared::error::Result;

/// Nome stabile di un tipo di esercizio.
///
/// Finisce nei progressi salvati, quindi non va cambiato a cuor leggero: rinominarlo
/// significa perdere lo storico di quell'esercizio.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExerciseTypeId(Cow<'static, str>);

impl ExerciseTypeId {
    /// Costruttore `const`: un tipo di esercizio dichiara il proprio nome una volta
    /// sola, come costante, senza allocare.
    pub const fn new(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ExerciseTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identificatore di un elemento di studio.
///
/// La forma della stringa la decide la feature che la produce: il livello condiviso
/// non deve sapere cosa ci sia scritto dentro.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(String);

impl ItemId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cosa mostrare all'utente, con l'indicazione di come va scritto.
///
/// La distinzione non e' estetica: il giapponese vuole un font e un corpo diversi, e
/// l'attributo `lang` giusto perche' il browser scelga le forme corrette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "script", content = "text", rename_all = "snake_case")]
pub enum Prompt {
    /// Testo giapponese, da mostrare in grande.
    Japanese(String),
    /// Testo in alfabeto latino.
    Latin(String),
}

/// In che modo l'utente risponde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AnswerFormat {
    /// Scelta multipla: le opzioni sono gia' mescolate e comprendono quella giusta.
    Choice { options: Vec<String> },
    /// Digitazione libera con l'IME del dispositivo.
    Input,
}

/// Una domanda pronta da mostrare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub exercise_type: ExerciseTypeId,
    pub item: ItemId,
    pub prompt: Prompt,
    pub format: AnswerFormat,
}

/// Cio' che l'utente ha prodotto: il testo digitato, o il valore dell'opzione scelta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer(String);

impl Answer {
    pub fn new(answer: impl Into<String>) -> Self {
        Self(answer.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// L'esito di una risposta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Verdict {
    Correct,
    /// Sbagliata, con le risposte che sarebbero state accettate: la UI le mostra
    /// invece di limitarsi a dire di no.
    Incorrect {
        accepted: Vec<String>,
    },
}

impl Verdict {
    pub fn is_correct(&self) -> bool {
        matches!(self, Self::Correct)
    }
}

/// Cosa serve per costruire una domanda.
#[derive(Debug, Clone, Copy)]
pub struct QuestionRequest<'a> {
    /// L'elemento su cui interrogare.
    pub item: &'a ItemId,
    /// Gli elementi tra cui pescare i distrattori, di norma quelli che l'utente sta
    /// studiando. Limitarli e' importante: proporre come alternativa un segno mai
    /// incontrato rende la scelta multipla piu' facile invece che piu' istruttiva.
    pub pool: &'a [ItemId],
    /// Quanti distrattori affiancare alla risposta giusta. Gli esercizi a input
    /// libero lo ignorano.
    pub distractors: usize,
}

/// Un tipo di esercizio.
///
/// L'implementazione conosce il proprio contenuto e se lo va a prendere da sola: il
/// livello condiviso non sa cosa sia un kana. Il tratto e' pensato per essere usato
/// dietro `dyn`, cosi' una sessione puo' tenere insieme esercizi di materie diverse.
pub trait ExerciseType {
    fn id(&self) -> ExerciseTypeId;

    /// Costruisce la domanda. L'`rng` arriva da fuori invece di essere preso dal
    /// sistema, cosi' i test possono fissare un seme e ottenere sempre lo stesso
    /// mescolamento.
    fn question(&self, request: QuestionRequest<'_>, rng: &mut dyn Rng) -> Result<Question>;

    /// Giudica una risposta all'elemento indicato.
    fn grade(&self, item: &ItemId, answer: &Answer) -> Result<Verdict>;
}
