//! Le flashcard, cioe' i mazzi di carte che l'utente si scrive da solo.
//!
//! E' la prima materia il cui **contenuto lo fa chi studia**. Kana e kanji sono
//! tabelle generate da uno script, versionate e uguali per tutti; qui i mazzi e le
//! carte nascono nel database dell'utente e cambiano quando lui decide.
//!
//! La conseguenza pratica e' che questa materia non ha un modulo `data`: ha [`deck`],
//! che e' la raccolta di quello che l'utente ha scritto e cosa se ne puo' fare.
//! [`exercise`] sa in che verso si puo' chiedere una carta, [`session`] fa girare un
//! giro di studio su un mazzo, [`steps`] tiene i passi brevi con cui una carta nuova
//! entra in circolo prima che sia FSRS a decidere.

pub mod deck;
pub mod exercise;
pub mod session;
pub mod steps;
