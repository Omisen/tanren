//! I kanji joyo, la materia che viene dopo i kana.
//!
//! # Due dataset accanto, per poco
//!
//! `levels` e `facets` sono il redesign: i kanji come percorso di apprendimento, col
//! dato da kanjium. `data`, `exercise` e `session` sono la versione precedente, per
//! grado scolastico, e restano solo finche' l'interfaccia non passa alla nuova: si
//! portano dietro la schermata che oggi funziona.

pub mod facets;
pub mod levels;
pub mod progress;
pub mod study;

// La versione precedente, in uscita.
pub mod data;
pub mod exercise;
pub mod session;
