//! I comandi esposti al frontend.
//!
//! Qui non c'e' logica di dominio: ogni comando prende gli argomenti, li passa al
//! core e restituisce il risultato. Se una di queste funzioni comincia a decidere
//! qualcosa, quella decisione e' finita nel posto sbagliato.

use chrono::Utc;
use tanren_core::features::kana::data::{KanaGroup, Syllabary, table};
use tanren_core::features::kana::session::{self, Scope, Step};
use tanren_core::shared::error::CoreError;
use tanren_core::shared::exercise::{Answer, ItemId, Verdict};
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

/// Riduce un testo alla forma con cui viene confrontato, sillabario compreso.
///
/// Serve al frontend per mostrare in tempo reale cosa verra' davvero valutato, mentre
/// l'IME e' ancora in mezzo alla conversione.
///
/// E' la pulizia che **non** ripiega sull'hiragana, la stessa che usa `kana.input`:
/// li' la domanda chiede un sillabario preciso, quindi rispondere か a una domanda su
/// カ e' sbagliato e l'anteprima non deve far credere il contrario. Quando arrivera'
/// un esercizio giudicato sulla lettura servira' anche `normalize_reading`, che e'
/// una funzione diversa e vorra' un comando suo.
#[tauri::command]
pub fn normalize_input(input: String) -> String {
    text::normalize_input(&input)
}

/// Comincia una sessione: la coda mescolata e la prima domanda.
///
/// Non serve il database: una sessione e' un giro completo sull'ambito scelto, e cosa
/// ci sia dentro l'ambito lo dice il contenuto, non i progressi.
#[tauri::command]
pub fn start_session(scope: Scope) -> Result<Step, CoreError> {
    // La casualita' vera entra qui, al bordo: il dominio la riceve, non se la prende.
    let mut rng = rand::rng();
    session::start(&scope, &mut rng)
}

/// Come continua il giro dopo una risposta.
///
/// La coda torna indietro com'era arrivata: e' il core a decidere chi esce e chi
/// rientra, il frontend la conserva soltanto.
#[tauri::command]
pub fn next_step(scope: Scope, queue: Vec<ItemId>, correct: bool) -> Result<Step, CoreError> {
    let mut rng = rand::rng();
    session::advance(&scope, &queue, correct, &mut rng)
}

/// Corregge una risposta e la registra.
///
/// `response_time_ms` lo misura il frontend, perche' e' l'unico a sapere quando la
/// domanda e' comparsa sullo schermo. Qui si limita a passare: **non entra nel
/// giudizio**, e la sezione 3 di CLAUDE.md spiega perche' non deve entrarci mai.
#[tauri::command]
pub async fn submit_answer(
    state: State<'_, AppState>,
    scope: Scope,
    item: String,
    answer: String,
    response_time_ms: Option<i64>,
) -> Result<Verdict, CoreError> {
    session::submit(
        &state.db,
        &scope,
        &ItemId::new(item),
        &Answer::new(answer),
        response_time_ms,
        Utc::now(),
    )
    .await
}
