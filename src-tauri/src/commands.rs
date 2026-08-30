//! I comandi esposti al frontend.
//!
//! Qui non c'e' logica di dominio: ogni comando prende gli argomenti, li passa al
//! core e restituisce il risultato. Se una di queste funzioni comincia a decidere
//! qualcosa, quella decisione e' finita nel posto sbagliato.

use chrono::Utc;
use tanren_core::features::kana::data::{KanaGroup, Syllabary, table};
use tanren_core::features::kana::session as kana;
use tanren_core::features::kanji::data::Grade;
use tanren_core::features::kanji::exercise::{FAMILIES, Family, items as kanji_items};
use tanren_core::features::kanji::levels::Level;
use tanren_core::features::kanji::progress::{LevelProgress, Pacing};
use tanren_core::features::kanji::session as kanji;
use tanren_core::features::kanji::study;
use tanren_core::shared::error::CoreError;
use tanren_core::shared::session::{Step, Task};
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

/// Cosa si puo' allenare di un anno di scuola: una famiglia e quanti item contiene.
#[derive(Debug, serde::Serialize)]
pub struct KanjiSet {
    family: Family,
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
pub fn start_kana_session(scope: kana::Scope) -> Result<Step, CoreError> {
    // La casualita' vera entra qui, al bordo: il dominio la riceve, non se la prende.
    let mut rng = rand::rng();
    kana::start(&scope, &mut rng)
}

/// Come continua il giro dopo una risposta.
///
/// La coda torna indietro com'era arrivata: e' il core a decidere chi esce e chi
/// rientra, il frontend la conserva soltanto.
#[tauri::command]
pub fn next_kana_step(scope: kana::Scope, queue: Vec<Task>, correct: bool) -> Result<Step, CoreError> {
    let mut rng = rand::rng();
    kana::advance(&scope, &queue, correct, &mut rng)
}

/// Corregge una risposta e la registra.
///
/// `response_time_ms` lo misura il frontend, perche' e' l'unico a sapere quando la
/// domanda e' comparsa sullo schermo. Qui si limita a passare: **non entra nel
/// giudizio**, e la sezione 3 di CLAUDE.md spiega perche' non deve entrarci mai.
#[tauri::command]
pub async fn submit_kana_answer(
    state: State<'_, AppState>,
    scope: kana::Scope,
    item: String,
    answer: String,
    response_time_ms: Option<i64>,
) -> Result<Verdict, CoreError> {
    kana::submit(
        &state.db,
        &scope,
        &ItemId::new(item),
        &Answer::new(answer),
        response_time_ms,
        Utc::now(),
    )
    .await
}

/// Cosa si puo' allenare di un anno: le famiglie di letture con quanti item contengono.
///
/// Stessa forma del catalogo dei kana. I sette gradi invece non passano di qui: sono
/// sette e non cambiano, quindi l'interfaccia li conosce come conosce i due sillabari.
#[tauri::command]
pub fn kanji_catalogue(grade: Grade) -> Vec<KanjiSet> {
    FAMILIES
        .into_iter()
        .map(|family| KanjiSet {
            family,
            size: kanji_items(grade, &[family]).len(),
        })
        .collect()
}

/// Comincia una sessione sui kanji.
#[tauri::command]
pub fn start_kanji_session(scope: kanji::Scope) -> Result<Step, CoreError> {
    let mut rng = rand::rng();
    kanji::start(&scope, &mut rng)
}

/// Come continua il giro sui kanji dopo una risposta.
#[tauri::command]
pub fn next_kanji_step(
    scope: kanji::Scope,
    queue: Vec<Task>,
    correct: bool,
) -> Result<Step, CoreError> {
    let mut rng = rand::rng();
    kanji::advance(&scope, &queue, correct, &mut rng)
}

/// Corregge una risposta sui kanji e la registra.
#[tauri::command]
pub async fn submit_kanji_answer(
    state: State<'_, AppState>,
    scope: kanji::Scope,
    item: String,
    answer: String,
    response_time_ms: Option<i64>,
) -> Result<Verdict, CoreError> {
    kanji::submit(
        &state.db,
        &scope,
        &ItemId::new(item),
        &Answer::new(answer),
        response_time_ms,
        Utc::now(),
    )
    .await
}

// ---------------------------------------------------------------------------
// Il percorso sui kanji: le tre modalita'.
//
// I comandi qui sotto sono il redesign. Quelli sopra col nome `kanji_*` sono la
// versione precedente, per grado scolastico, e se ne vanno con la schermata che li usa.
// ---------------------------------------------------------------------------

/// Riduce un testo alla forma con cui viene confrontata una **lettura**.
///
/// E' l'altra normalizzazione, quella che ripiega tutto sull'hiragana: sulle letture
/// conta cosa si legge, non in quale sillabario lo si e' scritto, e chi digita せい a
/// una domanda su セイ ha risposto. E' il comando che lo step 11 aveva previsto senza
/// scriverlo, perche' allora non serviva a nessuno.
#[tauri::command]
pub fn normalize_reading(input: String) -> String {
    text::normalize_reading(&input)
}

/// A che punto e' un livello, e cosa si puo' fare adesso.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    progress: LevelProgress,
    available: study::Available,
}

/// Quanto si e' consolidato un livello, e quali modalita' sono aperte.
#[tauri::command]
pub async fn kanji_overview(
    state: State<'_, AppState>,
    scope: study::Scope,
) -> Result<Overview, CoreError> {
    let pacing = Pacing::default();
    let now = Utc::now();
    Ok(Overview {
        progress: study::progress(&state.db, &scope, &pacing).await?,
        available: study::available(&state.db, &scope, &pacing, now).await?,
    })
}

/// Fin dove si e' arrivati: il primo livello non ancora consolidato.
#[tauri::command]
pub async fn kanji_current_level(state: State<'_, AppState>) -> Result<Level, CoreError> {
    tanren_core::features::kanji::progress::current_level(&state.db, &Pacing::default()).await
}

/// Un giro appena cominciato.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    /// I kanji da presentare prima di interrogare. Vuoto fuori dal Learning.
    introducing: Vec<String>,
    step: Step,
}

/// Comincia un giro di studio.
///
/// La scelta di cosa mettere in coda tocca il database ed e' asincrona; mescolare e
/// formulare la domanda no, e restano puri perche' la casualita' non deve attraversare
/// un'attesa.
#[tauri::command]
pub async fn start_kanji_study(
    state: State<'_, AppState>,
    scope: study::Scope,
) -> Result<StudySession, CoreError> {
    let plan = study::plan(&state.db, &scope, &Pacing::default(), Utc::now()).await?;
    let step = {
        let mut rng = rand::rng();
        study::start(&plan, &mut rng)?
    };

    Ok(StudySession {
        introducing: plan.introducing,
        step,
    })
}

/// Come continua il giro dopo una risposta.
#[tauri::command]
pub fn next_kanji_study_step(
    mode: study::Mode,
    queue: Vec<Task>,
    correct: bool,
) -> Result<Step, CoreError> {
    let mut rng = rand::rng();
    study::advance(mode, &queue, correct, &mut rng)
}

/// Corregge una risposta e la registra.
#[tauri::command]
pub async fn submit_kanji_study_answer(
    state: State<'_, AppState>,
    mode: study::Mode,
    task: Task,
    answer: String,
    response_time_ms: Option<i64>,
) -> Result<Verdict, CoreError> {
    study::submit(
        &state.db,
        mode,
        &task,
        &Answer::new(answer),
        response_time_ms,
        Utc::now(),
    )
    .await
}
