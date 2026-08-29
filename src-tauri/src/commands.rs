//! I comandi esposti al frontend.
//!
//! Qui non c'e' logica di dominio: ogni comando prende gli argomenti, li passa al
//! core e restituisce il risultato. Se una di queste funzioni comincia a decidere
//! qualcosa, quella decisione e' finita nel posto sbagliato.

use chrono::Utc;
use tanren_core::features::kana::data::{KanaGroup, Syllabary, table};
use tanren_core::features::kana::session::{self, Outcome, Progress, Scope};
use tanren_core::shared::error::CoreError;
use tanren_core::shared::exercise::{Answer, ItemId, Question};
use tanren_core::shared::text;
use tauri::State;

use crate::AppState;

/// Cosa si puo' allenare: le famiglie di un sillabario con quanti segni contengono.
#[derive(Debug, serde::Serialize)]
pub struct KanaSet {
    group: KanaGroup,
    size: usize,
}

/// Il catalogo di un sillabario, per costruire la schermata di scelta.
#[tauri::command]
pub fn kana_catalogue(syllabary: Syllabary) -> Vec<KanaSet> {
    let t = table(syllabary);
    [
        KanaGroup::Base,
        KanaGroup::Dakuten,
        KanaGroup::Handakuten,
        KanaGroup::Yoon,
    ]
    .into_iter()
    .map(|group| KanaSet {
        group,
        size: t.group(group).count(),
    })
    .collect()
}

/// Riduce un testo alla forma con cui viene confrontato.
///
/// Serve al frontend per mostrare in tempo reale cosa verra' davvero valutato, mentre
/// l'IME e' ancora in mezzo alla conversione.
#[tauri::command]
pub fn normalize_reading(input: String) -> String {
    text::normalize_input(&input)
}

/// Prepara l'ambito scelto e riporta a che punto si e'.
#[tauri::command]
pub async fn prepare_session(
    state: State<'_, AppState>,
    scope: Scope,
) -> Result<Progress, CoreError> {
    session::prepare(&state.db, &scope, Utc::now()).await
}

/// A che punto e' l'ambito, senza modificare niente.
#[tauri::command]
pub async fn session_progress(
    state: State<'_, AppState>,
    scope: Scope,
) -> Result<Progress, CoreError> {
    session::progress(&state.db, &scope, Utc::now()).await
}

/// La prossima domanda, o niente se per adesso non c'e' altro da ripassare.
#[tauri::command]
pub async fn next_question(
    state: State<'_, AppState>,
    scope: Scope,
) -> Result<Option<Question>, CoreError> {
    let candidates = session::due_items(&state.db, &scope, Utc::now()).await?;

    // La casualita' vera entra qui, al bordo: il dominio la riceve, non se la prende.
    // Serve due volte, per scegliere il segno tra quelli ugualmente urgenti e per
    // mescolare le opzioni della domanda.
    let mut rng = rand::rng();
    let Some(item) = session::pick(&candidates, &mut rng) else {
        return Ok(None);
    };

    Ok(Some(session::question_for(&scope, &item, &mut rng)?))
}

/// Corregge una risposta, ripianifica il segno e registra tutto.
#[tauri::command]
pub async fn submit_answer(
    state: State<'_, AppState>,
    scope: Scope,
    item: String,
    answer: String,
) -> Result<Outcome, CoreError> {
    session::submit(
        &state.db,
        &state.scheduler,
        &scope,
        &ItemId::new(item),
        &Answer::new(answer),
        Utc::now(),
    )
    .await
}
