//! I comandi esposti al frontend.
//!
//! Qui non c'e' logica di dominio: ogni comando prende gli argomenti, li passa al
//! core e restituisce il risultato. Se una di queste funzioni comincia a decidere
//! qualcosa, quella decisione e' finita nel posto sbagliato.

use chrono::Utc;
use tanren_core::features::flashcards::deck::{
    self as flashcards, Content, Deck, DeckSummary, Flashcard,
};
use tanren_core::features::flashcards::session::{self as flashcard_session, Answered};
use tanren_core::features::flashcards::steps as flashcard_steps;
use tanren_core::features::kana::data::{KanaGroup, KanaTable, Syllabary, Vowel, table};
use tanren_core::features::kana::patterns::{Pattern, patterns};
use tanren_core::features::kana::session as kana;
use tanren_core::features::kanji::levels::{Kanji, Level, table as levels_table};
use tanren_core::features::kanji::progress::{self, LevelProgress, LevelSummary};
use tanren_core::features::kanji::study;
use tanren_core::shared::credits::Credit;
use tanren_core::shared::error::CoreError;
use tanren_core::shared::session::{Step, Task};
use tanren_core::shared::srs::Grade;
use tanren_core::shared::exercise::{Answer, ItemId, Verdict};
use tanren_core::shared::text;
use tauri::State;

use crate::AppState;

/// Un segno nel catalogo: quello che serve a disegnarlo nella tavola.
#[derive(Debug, serde::Serialize)]
pub struct KanaCell {
    character: String,
    /// La trascrizione canonica, quella da mostrare sotto il segno.
    romaji: String,
    /// In quale colonna va messo, e `None` per ん, che sta da solo.
    column: Option<Vowel>,
}

/// Una riga della tavola, coi segni che la compongono.
#[derive(Debug, serde::Serialize)]
pub struct KanaRow {
    /// La riga del gojuon. **Non e' unica fra famiglie**: lo yoon riusa `ka`, `ga`,
    /// `sa`... quindi a identificarla e' la coppia con la famiglia, non questo da solo.
    row: String,
    cells: Vec<KanaCell>,
}

/// Cosa si puo' allenare: una famiglia di un sillabario, coi suoi segni in tavola.
///
/// # Perche' porta i segni e non solo il conteggio
///
/// Perche' la schermata li mostra tutti, aperti a cascata, e senza questi non avrebbe
/// in mano nient'altro che cinque numeri. Il confine cresce, il **modello dati no**:
/// i file dei kana non cambiano, non c'e' nessuna migrazione, e `Scope`, `items()` e
/// gli esercizi restano quelli. Quello che passa di qui e' gia' tutto nella tabella.
///
/// `size` resta anche se e' la somma dei segni: e' quello che alimenta il conteggio
/// sul bottone di avvio, e ricavarlo di la' vorrebbe dire contare cio' che il core
/// sa gia'.
#[derive(Debug, serde::Serialize)]
pub struct KanaSet {
    group: KanaGroup,
    size: usize,
    rows: Vec<KanaRow>,
}

/// I segni di una famiglia, raccolti nelle righe della tavola.
///
/// Le righe escono nell'ordine in cui compaiono nella tabella, che e' quello
/// tradizionale: あ, か, さ... Si cerca la riga gia' aperta invece di fidarsi che le
/// voci di una riga siano contigue, perche' costa niente (sono al massimo dodici) e
/// non lega questa funzione a come il generatore ordina il file.
fn rows(t: &KanaTable, group: KanaGroup) -> Vec<KanaRow> {
    let mut rows: Vec<KanaRow> = Vec::new();

    for k in t.group(group) {
        let cell = KanaCell {
            character: k.character.clone(),
            romaji: k.romaji.first().cloned().unwrap_or_default(),
            column: k.vowel(),
        };
        match rows.iter_mut().find(|r| r.row == k.row) {
            Some(r) => r.cells.push(cell),
            None => rows.push(KanaRow {
                row: k.row.clone(),
                cells: vec![cell],
            }),
        }
    }

    rows
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
            rows: rows(t, group),
        })
        .collect()
}

/// Le due regole di scrittura che si mostrano e non si chiedono: il sokuon e le vocali
/// lunghe.
///
/// **E' un comando a parte e non una voce del catalogo**, ed e' la ragione per cui
/// esiste: il catalogo dice cosa si puo' **allenare**, e da li' escono il conteggio sul
/// bottone di avvio e la regola che senza famiglie non si parte. Queste due cose non si
/// allenano, quindi metterle li' vorrebbe dire o farle entrare in una sessione o
/// riempire il catalogo di eccezioni per tenerle fuori.
#[tauri::command]
pub fn kana_patterns(syllabary: Syllabary) -> Vec<Pattern> {
    patterns(syllabary).to_vec()
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
    let pacing = progress::pacing(&state.db).await?;
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

/// Le preferenze dell'utente, coi limiti entro cui puo' muoverle.
///
/// I limiti arrivano dal core insieme al valore invece di essere scritti nella
/// schermata: sono una decisione di dominio, e duplicarli di la' vorrebbe dire avere
/// due verita' che possono sganciarsi.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub daily_new: usize,
    pub daily_new_min: usize,
    pub daily_new_max: usize,
    /// Dopo quanti minuti torna una flashcard sbagliata.
    pub flashcard_again: i64,
    pub flashcard_again_min: i64,
    pub flashcard_again_max: i64,
    /// Dopo quanti minuti torna una flashcard nuova appena indovinata.
    pub flashcard_good: i64,
    pub flashcard_good_min: i64,
    pub flashcard_good_max: i64,
    /// Quanti significati si accettano oltre a quello principale.
    ///
    /// Non e' una preferenza e non si muove: e' un **limite di dominio** che il modulo
    /// di scrittura deve conoscere per sapere quando smettere di offrire caselle.
    /// Viaggia di qua perche' questo e' gia' il comando che porta i limiti, e tenerne
    /// una copia scritta nella schermata vorrebbe dire due verita' che si sganciano.
    pub flashcard_max_alternatives: usize,
}

/// Quante cose l'utente ha deciso, e fra quali limiti poteva.
#[tauri::command]
pub async fn settings(state: State<'_, AppState>) -> Result<Settings, CoreError> {
    let pacing = progress::pacing(&state.db).await?;
    let steps = flashcard_steps::steps(&state.db).await?;
    Ok(Settings {
        daily_new: pacing.daily_new,
        daily_new_min: *progress::DAILY_NEW_RANGE.start(),
        daily_new_max: *progress::DAILY_NEW_RANGE.end(),
        flashcard_again: steps.again,
        flashcard_again_min: *flashcard_steps::AGAIN_RANGE.start(),
        flashcard_again_max: *flashcard_steps::AGAIN_RANGE.end(),
        flashcard_good: steps.good,
        flashcard_good_min: *flashcard_steps::GOOD_RANGE.start(),
        flashcard_good_max: *flashcard_steps::GOOD_RANGE.end(),
        flashcard_max_alternatives: flashcards::MAX_ALTERNATIVES,
    })
}

/// Cambia quanti kanji nuovi si incontrano per lezione.
#[tauri::command]
pub async fn set_kanji_daily_new(
    state: State<'_, AppState>,
    value: usize,
) -> Result<(), CoreError> {
    progress::set_daily_new(&state.db, value, Utc::now()).await
}

/// Fin dove si e' arrivati: il primo livello non ancora consolidato.
#[tauri::command]
pub async fn kanji_current_level(state: State<'_, AppState>) -> Result<Level, CoreError> {
    let pacing = progress::pacing(&state.db).await?;
    progress::current_level(&state.db, &pacing).await
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
    let pacing = progress::pacing(&state.db).await?;
    let stati = progress::standings(&state.db, level, &pacing).await?;

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
    let pacing = progress::pacing(&state.db).await?;
    progress::all_levels(&state.db, &pacing, Utc::now()).await
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
    let pacing = progress::pacing(&state.db).await?;
    let plan = study::plan(&state.db, &scope, &pacing, Utc::now()).await?;
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

// ---------------------------------------------------------------------------
// Le flashcard: i mazzi e le carte che l'utente si scrive.
//
// E' l'unica materia il cui contenuto non sta nel binario ma nel database, quindi e'
// anche l'unica che ha comandi per **scriverlo** e non solo per leggerlo.
// ---------------------------------------------------------------------------

/// Tutti i mazzi, in ordine alfabetico, con quante carte contengono.
#[tauri::command]
pub async fn flashcard_decks(state: State<'_, AppState>) -> Result<Vec<DeckSummary>, CoreError> {
    flashcards::decks(&state.db).await
}

/// Crea un mazzo. Serve solo il nome: cosa ci va dentro si decide dopo.
#[tauri::command]
pub async fn create_flashcard_deck(
    state: State<'_, AppState>,
    name: String,
) -> Result<Deck, CoreError> {
    flashcards::create_deck(&state.db, &name, Utc::now()).await
}

/// Cambia il nome di un mazzo.
#[tauri::command]
pub async fn rename_flashcard_deck(
    state: State<'_, AppState>,
    deck: String,
    name: String,
) -> Result<(), CoreError> {
    flashcards::rename_deck(&state.db, &deck, &name, Utc::now()).await
}

/// Elimina un mazzo con tutte le sue carte, e ne ritira la pianificazione.
#[tauri::command]
pub async fn delete_flashcard_deck(
    state: State<'_, AppState>,
    deck: String,
) -> Result<(), CoreError> {
    flashcards::delete_deck(&state.db, &deck, Utc::now()).await
}

/// Le carte di un mazzo, nell'ordine in cui sono state aggiunte.
#[tauri::command]
pub async fn flashcard_cards(
    state: State<'_, AppState>,
    deck: String,
) -> Result<Vec<Flashcard>, CoreError> {
    flashcards::cards(&state.db, &deck).await
}

/// Aggiunge una carta a un mazzo.
///
/// `alternatives` sono gli altri significati accettati e `furigana` la lettura: le
/// caselle lasciate vuote non sono risposte e il core le scarta, quindi l'interfaccia
/// puo' mandare quello che ha senza ripulirlo.
#[tauri::command]
pub async fn create_flashcard(
    state: State<'_, AppState>,
    deck: String,
    japanese: String,
    meaning: String,
    alternatives: Vec<String>,
    furigana: String,
) -> Result<Flashcard, CoreError> {
    flashcards::create_card(
        &state.db,
        &deck,
        Content {
            japanese: &japanese,
            meaning: &meaning,
            alternatives: &alternatives,
            furigana: &furigana,
        },
        Utc::now(),
    )
    .await
}

/// Com'e' andata una correzione, cioe' se c'e' qualcosa da chiedere.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edited {
    /// Se il testo e' davvero cambiato, **dopo** la pulizia: una casella vuota, uno
    /// spazio ai bordi o un doppione non sono una modifica.
    changed: bool,
    /// Se la carta ha dei progressi, cioe' se c'e' qualcosa da azzerare.
    studied: bool,
}

/// Corregge una carta gia' scritta. **Non tocca lo stato di studio.**
///
/// Torna le due cose che decidono se chiedere cosa farne dei progressi: se la
/// correzione ha invalidato quello che si era imparato lo sa **solo chi ha corretto**,
/// e la domanda gliela fa l'interfaccia, ma solo dove c'e' davvero qualcosa da
/// decidere. Le risposte arrivano insieme alla scrittura perche' e' la scrittura stessa
/// a sapere su cosa e' passata.
#[tauri::command]
pub async fn update_flashcard(
    state: State<'_, AppState>,
    card: String,
    japanese: String,
    meaning: String,
    alternatives: Vec<String>,
    furigana: String,
) -> Result<Edited, CoreError> {
    let changed = flashcards::update_card(
        &state.db,
        &card,
        Content {
            japanese: &japanese,
            meaning: &meaning,
            alternatives: &alternatives,
            furigana: &furigana,
        },
        Utc::now(),
    )
    .await?;

    Ok(Edited {
        changed,
        studied: flashcards::studied(&state.db, &card).await?,
    })
}

/// Riporta i progressi di una carta a zero, in tutti e due i versi.
///
/// **Non succede mai da solo**: ci si arriva solo perche' qualcuno ha scelto di
/// ricominciare da capo su quella carta.
#[tauri::command]
pub async fn reset_flashcard(
    state: State<'_, AppState>,
    card: String,
) -> Result<(), CoreError> {
    flashcards::reset_card(&state.db, &card, Utc::now()).await
}

/// Elimina una carta, e con lei la sua pianificazione.
#[tauri::command]
pub async fn delete_flashcard(
    state: State<'_, AppState>,
    card: String,
) -> Result<(), CoreError> {
    flashcards::delete_card(&state.db, &card, Utc::now()).await
}

/// Un giro appena cominciato.
///
/// La modalita' viene decisa dal core guardando cosa e' dovuto, e torna insieme al
/// primo passo perche' **vale per tutto il giro**: chi studia se la conserva e la
/// rimanda indietro a ogni risposta, come gia' fa con la coda.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashcardSession {
    mode: flashcard_session::Mode,
    step: Step,
}

/// Cosa si troverebbe partendo adesso su questo mazzo, in questo verso.
#[tauri::command]
pub async fn flashcard_availability(
    state: State<'_, AppState>,
    scope: flashcard_session::Scope,
) -> Result<flashcard_session::Available, CoreError> {
    flashcard_session::available(&state.db, &scope, Utc::now()).await
}

/// Comincia un giro su un mazzo: la modalita', la coda mescolata e la prima domanda.
///
/// Tre passi e non uno perche' due toccano il database e quello in mezzo no: leggere
/// il mazzo e leggere la carta da chiedere sono attese, mescolare vuole il generatore
/// di numeri casuali, che non deve attraversarne nessuna. Il generatore vive dentro il
/// blocco e muore li', altrimenti il future non sarebbe `Send` e Tauri lo rifiuterebbe.
#[tauri::command]
pub async fn start_flashcard_session(
    state: State<'_, AppState>,
    scope: flashcard_session::Scope,
) -> Result<FlashcardSession, CoreError> {
    let plan = flashcard_session::plan(&state.db, &scope, Utc::now()).await?;
    let queue = {
        let mut rng = rand::rng();
        flashcard_session::shuffle(plan.tasks, &mut rng)
    };

    Ok(FlashcardSession {
        mode: plan.mode,
        step: flashcard_session::open(&state.db, &scope, queue).await?,
    })
}

/// Come continua il giro dopo una risposta.
///
/// Una carta sbagliata torna in coda poco piu' avanti, che e' la regola condivisa con
/// le altre materie.
#[tauri::command]
pub async fn next_flashcard_step(
    state: State<'_, AppState>,
    scope: flashcard_session::Scope,
    queue: Vec<Task>,
    correct: bool,
) -> Result<Step, CoreError> {
    let queue = {
        let mut rng = rand::rng();
        flashcard_session::requeue(&queue, correct, &mut rng)
    };
    flashcard_session::open(&state.db, &scope, queue).await
}

/// Corregge una risposta **senza registrare niente**.
///
/// Serve perche' il voto arriva dopo: prima si sa se si ha indovinato, e solo allora
/// si puo' dire quanto e' costato.
#[tauri::command]
pub async fn check_flashcard_answer(
    state: State<'_, AppState>,
    scope: flashcard_session::Scope,
    item: String,
    answer: String,
) -> Result<Verdict, CoreError> {
    flashcard_session::check(
        &state.db,
        &scope,
        &ItemId::new(item),
        &Answer::new(answer),
    )
    .await
}

/// Registra la risposta e, in Review, sposta la scadenza.
///
/// Il voto e' quello che ha scelto l'utente su una risposta giusta. Su una sbagliata
/// non lo sceglie nessuno e quello che arriva viene ignorato: la regola sta nel core.
#[tauri::command]
pub async fn submit_flashcard_answer(
    state: State<'_, AppState>,
    scope: flashcard_session::Scope,
    mode: flashcard_session::Mode,
    item: String,
    answer: String,
    grade: Option<Grade>,
    response_time_ms: Option<i64>,
) -> Result<Verdict, CoreError> {
    let steps = flashcard_steps::steps(&state.db).await?;
    flashcard_session::submit(
        &state.db,
        &scope,
        mode,
        Answered {
            item: &ItemId::new(item),
            answer: &Answer::new(answer),
            grade,
            response_time_ms,
        },
        &steps,
        Utc::now(),
    )
    .await
}

/// Cambia dopo quanti minuti torna una flashcard sbagliata.
#[tauri::command]
pub async fn set_flashcard_again(
    state: State<'_, AppState>,
    value: i64,
) -> Result<(), CoreError> {
    flashcard_steps::set_again(&state.db, value, Utc::now()).await
}

/// Cambia dopo quanti minuti torna una flashcard nuova appena indovinata.
#[tauri::command]
pub async fn set_flashcard_good(
    state: State<'_, AppState>,
    value: i64,
) -> Result<(), CoreError> {
    flashcard_steps::set_good(&state.db, value, Utc::now()).await
}
