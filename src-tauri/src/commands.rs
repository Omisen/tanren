//! I comandi esposti al frontend.
//!
//! Qui non c'e' logica di dominio: ogni comando prende gli argomenti, li passa al
//! core e restituisce il risultato. Se una di queste funzioni comincia a decidere
//! qualcosa, quella decisione e' finita nel posto sbagliato.

use chrono::Utc;
use tanren_core::features::kana::data::{KanaGroup, Syllabary, table};
use tanren_core::features::kana::session as kana;
use tanren_core::features::kanji::levels::{Kanji, Level, table as levels_table};
use tanren_core::features::kanji::progress::{LevelProgress, LevelSummary, Pacing};
use tanren_core::features::kanji::study;
use tanren_core::shared::credits::Credit;
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

/// Il catalogo di un sillabario, per costruire la schermata di scelta.
#[tauri::command]
pub fn kana_catalogue(syllabary: Syllabary) -> Vec<KanaSet> {
    let t = table(syllabary);
    // I 外来音 esistono solo in katakana, e la conseguenza sta tutta qui: il catalogo e'
    // gia' per sillabario, quindi basta non elencarli per l'hiragana e la schermata,
    // che disegna quello che riceve, non ha bisogno di nessuna condizione. La regola
    // vive nel posto che sa cosa sia un sillabario.
    let mut groups = vec![
        KanaGroup::Base,
        KanaGroup::Dakuten,
        KanaGroup::Handakuten,
        KanaGroup::Yoon,
    ];
    if syllabary == Syllabary::Katakana {
        groups.push(KanaGroup::Gairaion);
    }

    groups
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

/// Chi ha fatto cosa, e sotto quale licenza Tanren ridistribuisce.
///
/// Le fonti dei dati le dichiara la materia che li usa, perche' e' l'unica a sapere da
/// quale edizione vengono; il font e la licenza del progetto stanno nel livello
/// condiviso. Qui si mettono in fila e basta.
///
/// **Non e' una schermata di cortesia**: la CC BY-SA e la licenza dell'EDRDG obbligano
/// ad attribuire dentro il mezzo in cui l'opera viaggia, e per un'app quel mezzo e'
/// l'APK. Senza questo, ogni release coi dati dei kanji sarebbe in violazione.
#[tauri::command]
pub fn credits() -> Vec<Credit> {
    let mut tutti = tanren_core::features::kanji::levels::credits();
    tutti.extend(tanren_core::shared::credits::app());
    tutti
}

/// La versione dell'app, come la dichiara il pacchetto.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Quanti livelli conta il percorso.
///
/// L'interfaccia ne ha bisogno per non offrire livelli che non esistono. Arriva dal
/// core e non e' scritto di la': il numero cambia quando si rigenera il contenuto, e
/// due copie dello stesso numero divergerebbero senza che nessuno se ne accorga.
#[tauri::command]
pub fn kanji_level_count() -> u8 {
    tanren_core::features::kanji::levels::LEVELS
}

/// Fin dove si e' arrivati: il primo livello non ancora consolidato.
#[tauri::command]
pub async fn kanji_current_level(state: State<'_, AppState>) -> Result<Level, CoreError> {
    tanren_core::features::kanji::progress::current_level(&state.db, &Pacing::default()).await
}

/// Una cella della griglia di un livello.
#[derive(Debug, serde::Serialize)]
pub struct KanjiCell {
    character: String,
    standing: tanren_core::features::kanji::progress::Standing,
}

/// I kanji di un livello con lo stato di ciascuno, nell'ordine della tabella.
///
/// L'ordine e' per frequenza e non cambia: una griglia che si riordina a ogni risposta
/// non si potrebbe guardare.
#[tauri::command]
pub async fn kanji_grid(
    state: State<'_, AppState>,
    level: Level,
) -> Result<Vec<KanjiCell>, CoreError> {
    let stati =
        tanren_core::features::kanji::progress::standings(&state.db, level, &Pacing::default())
            .await?;

    Ok(stati
        .into_iter()
        .map(|(character, standing)| KanjiCell {
            character,
            standing,
        })
        .collect())
}

/// Come sta andando tutto il percorso, livello per livello.
///
/// Misura **quanto sei consolidato**, che lo dice FSRS e lo alimentano solo il Learning
/// e il Ripasso. Il Drill non compare qui e non deve: e' pratica in piu', e le sue
/// statistiche vivono e muoiono dentro la sessione.
#[tauri::command]
pub async fn kanji_dashboard(
    state: State<'_, AppState>,
) -> Result<Vec<LevelSummary>, CoreError> {
    tanren_core::features::kanji::progress::all_levels(&state.db, &Pacing::default(), Utc::now())
        .await
}

/// I kanji chiesti, per intero.
///
/// Serve a presentarli prima di interrogarli, e sara' la stessa cosa che alimenta la
/// scheda di dettaglio: quello che si mostra per conoscere un kanji e quello che si
/// mostra per riguardarlo sono la stessa scheda.
#[tauri::command]
pub fn kanji_details(level: Level, characters: Vec<String>) -> Vec<Kanji> {
    let t = levels_table(level);
    characters
        .iter()
        .filter_map(|c| t.get(c))
        .cloned()
        .collect()
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
