//! I kanji joyo, come percorso di apprendimento.
//!
//! Il dato viene da kanjium ([`levels`]), un kanji diventa piu' item di studio uno per
//! faccetta ([`facets`]), lo stato di ciascuno e la regola di sblocco stanno in
//! [`progress`], e le tre modalita' in [`study`].

pub mod facets;
pub mod levels;
pub mod progress;
pub mod study;
