//! Cuore del dominio di Tanren.
//!
//! Questa crate non sa nulla di Tauri, di React o di come i dati arrivino allo
//! schermo: espone tipi e funzioni pure di dominio, piu' l'accesso ai dati.
//!
//! # Organizzazione
//!
//! - [`shared`] contiene cio' che vale per tutte le materie di studio: il
//!   contratto degli esercizi, la normalizzazione del testo, lo scheduler SRS,
//!   la persistenza e il caricamento del contenuto.
//! - [`features`] contiene una materia per modulo (kana, e in futuro kanji e
//!   grammatica).
//!
//! # Regola di dipendenza
//!
//! Una feature puo' dipendere da `shared`. Una feature non puo' mai dipendere da
//! un'altra feature: se due materie hanno bisogno della stessa cosa, quella cosa
//! sale in `shared`.

pub mod features;
pub mod shared;
