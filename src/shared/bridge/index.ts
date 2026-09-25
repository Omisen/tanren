/**
 * L'unico punto da cui il frontend parla con il core Rust.
 *
 * Nessun altro modulo dovrebbe importare `invoke` direttamente: tenendo le chiamate
 * qui, i nomi dei comandi e le loro firme stanno in un posto solo, e la UI vede
 * funzioni tipizzate invece di stringhe.
 *
 * I comandi portano il nome della materia (`start_kana_session`, `start_kanji_session`)
 * perche' con due materie un `start_session` non direbbe di quale.
 */

import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

import type {
  Credit,
  Deck,
  DeckSummary,
  Edited,
  Flashcard,
  FlashcardAvailability,
  FlashcardMode,
  FlashcardScope,
  FlashcardSession,
  Grade,
  ImportReview,
  Kanji,
  KanjiCell,
  LevelSummary,
  Overview,
  StudyMode,
  StudyScope,
  StudySession,
  Level,
  Task,
  KanaPattern,
  KanaScope,
  KanaSet,
  Queue,
  Settings,
  Step,
  Syllabary,
  Verdict,
} from './types'

export * from './types'

/**
 * Riduce un testo alla forma con cui verra' confrontato, sillabario compreso.
 *
 * Serve a mostrare in tempo reale cosa sara' davvero valutato, mentre l'IME e' ancora
 * in mezzo alla conversione. Non ripiega sull'hiragana: per `kana.input` rispondere か
 * a una domanda su カ e' sbagliato, e l'anteprima non deve far credere il contrario.
 */
export function normalizeInput(input: string): Promise<string> {
  return invoke('normalize_input', { input })
}

/**
 * Chi ha fatto cosa, e sotto quale licenza Tanren ridistribuisce.
 *
 * L'edizione dei dati arriva dal dato stesso, non da una stringa scritta a mano: cosi'
 * non puo' divergere da quello che l'app spedisce davvero.
 */
export function credits(): Promise<Credit[]> {
  return invoke('credits')
}

/**
 * Apre un indirizzo **fuori** dall'app, nel browser del sistema.
 *
 * Sta qui come tutto il resto che attraversa il confine: passa dal plugin `opener`, che
 * e' il modo previsto da Tauri, e non da un'ancora lasciata alla WebView, che invece
 * navigherebbe sul posto e si mangerebbe l'app.
 */
export function openExternal(url: string): Promise<void> {
  return openUrl(url)
}

/** La versione dell'app, come la dichiara il pacchetto. */
export function appVersion(): Promise<string> {
  return invoke('app_version')
}

/**
 * Quello che l'utente ha scelto, coi limiti entro cui poteva sceglierlo.
 *
 * I limiti arrivano insieme al valore e non sono scritti nella schermata: sono una
 * decisione di dominio, e tenerne una copia di qua vorrebbe dire avere due verita' che
 * prima o poi si sganciano.
 */
export function settings(): Promise<Settings> {
  return invoke('settings')
}

/** Cambia quanti kanji nuovi si incontrano per lezione. */
export function setKanjiDailyNew(value: number): Promise<void> {
  return invoke('set_kanji_daily_new', { value })
}

/** Le famiglie di un sillabario, con quanti segni contengono. */
export function kanaCatalogue(syllabary: Syllabary): Promise<KanaSet[]> {
  return invoke('kana_catalogue', { syllabary })
}

/**
 * Le due regole di scrittura che si mostrano e non si chiedono.
 *
 * Sta in una chiamata sua e non nel catalogo perche' il catalogo dice cosa si puo'
 * **allenare**: da li' escono il conteggio e la regola che senza famiglie non si parte.
 */
export function kanaPatterns(syllabary: Syllabary): Promise<KanaPattern[]> {
  return invoke('kana_patterns', { syllabary })
}

/** Comincia una sessione sui kana: la coda mescolata e la prima domanda. */
export function startKanaSession(scope: KanaScope): Promise<Step> {
  return invoke('start_kana_session', { scope })
}

/**
 * Come continua il giro dopo una risposta.
 *
 * La coda torna al core com'era arrivata: chi esce e chi rientra lo decide lui.
 */
export function nextKanaStep(
  scope: KanaScope,
  queue: Queue,
  correct: boolean,
): Promise<Step> {
  return invoke('next_kana_step', { scope, queue, correct })
}

/**
 * Corregge una risposta e la registra nello storico.
 *
 * `responseTimeMs` e' quanto e' passato da quando la domanda e' comparsa a quando
 * l'utente ha risposto. Lo misura il frontend perche' e' l'unico a sapere quando la
 * domanda e' comparsa davvero; il core lo registra e basta, **non ci giudica sopra**.
 * `null` quando non e' stato misurato, che non e' la stessa cosa di zero.
 */
export function submitKanaAnswer(
  scope: KanaScope,
  item: string,
  answer: string,
  responseTimeMs: number | null,
): Promise<Verdict> {
  return invoke('submit_kana_answer', { scope, item, answer, responseTimeMs })
}


/* --- Il percorso sui kanji ------------------------------------------------- */

/**
 * Riduce un testo alla forma con cui verra' confrontata una **lettura**.
 *
 * E' l'altra normalizzazione rispetto a `normalizeInput`: ripiega tutto sull'hiragana,
 * perche' su una lettura conta cosa si legge e non in quale sillabario lo si e'
 * scritto. Chi digita せい a una domanda su セイ ha risposto.
 */
export function normalizeReading(input: string): Promise<string> {
  return invoke('normalize_reading', { input })
}

/** I kanji di un livello con lo stato di ciascuno, nell'ordine per frequenza. */
export function kanjiGrid(level: Level): Promise<KanjiCell[]> {
  return invoke('kanji_grid', { level })
}

/**
 * I kanji chiesti, per intero.
 *
 * Quello che si mostra per conoscere un kanji e quello che si mostra per riguardarlo
 * sono la stessa scheda.
 */
export function kanjiDetails(level: Level, characters: string[]): Promise<Kanji[]> {
  return invoke('kanji_details', { level, characters })
}

/**
 * Come sta andando tutto il percorso, livello per livello.
 *
 * Misura **quanto sei consolidato**, che lo dice FSRS e lo alimentano solo il Learning
 * e il Ripasso. Il Drill non compare qui e non deve.
 */
export function kanjiDashboard(): Promise<LevelSummary[]> {
  return invoke('kanji_dashboard')
}

/** Quanto si e' consolidato un livello, e quali modalita' sono aperte. */
export function kanjiOverview(scope: StudyScope): Promise<Overview> {
  return invoke('kanji_overview', { scope })
}

/** Fin dove si e' arrivati: il primo livello non ancora consolidato. */
export function kanjiCurrentLevel(): Promise<Level> {
  return invoke('kanji_current_level')
}

/** Comincia un giro di studio: i kanji da presentare e la prima domanda. */
export function startKanjiStudy(scope: StudyScope): Promise<StudySession> {
  return invoke('start_kanji_study', { scope })
}

/** Come continua il giro dopo una risposta. */
export function nextKanjiStudyStep(
  mode: StudyMode,
  queue: Queue,
  correct: boolean,
): Promise<Step> {
  return invoke('next_kanji_study_step', { mode, queue, correct })
}

/** Corregge una risposta e la registra. Nel Drill non sposta nessuna scadenza. */
export function submitKanjiStudyAnswer(
  mode: StudyMode,
  task: Task,
  answer: string,
  responseTimeMs: number | null,
): Promise<Verdict> {
  return invoke('submit_kanji_study_answer', { mode, task, answer, responseTimeMs })
}

/* --- Le flashcard ---------------------------------------------------------- */

/** Tutti i mazzi, in ordine alfabetico, con quante carte contengono. */
export function flashcardDecks(): Promise<DeckSummary[]> {
  return invoke('flashcard_decks')
}

/** Crea un mazzo. Serve solo il nome: cosa ci va dentro si decide dopo. */
export function createFlashcardDeck(name: string): Promise<Deck> {
  return invoke('create_flashcard_deck', { name })
}

/** Cambia il nome di un mazzo. Le carte e i loro progressi non si toccano. */
export function renameFlashcardDeck(deck: string, name: string): Promise<void> {
  return invoke('rename_flashcard_deck', { deck, name })
}

/** Elimina un mazzo con tutte le sue carte, e ne ritira la pianificazione. */
export function deleteFlashcardDeck(deck: string): Promise<void> {
  return invoke('delete_flashcard_deck', { deck })
}

/** Le carte di un mazzo, nell'ordine in cui sono state aggiunte. */
export function flashcardCards(deck: string): Promise<Flashcard[]> {
  return invoke('flashcard_cards', { deck })
}

/**
 * Aggiunge una carta a un mazzo.
 *
 * Le caselle lasciate vuote non sono risposte e le scarta il core, quindi si manda
 * quello che si ha senza ripulirlo prima.
 */
export function createFlashcard(
  deck: string,
  japanese: string,
  meaning: string,
  alternatives: string[],
  furigana: string,
): Promise<Flashcard> {
  return invoke('create_flashcard', { deck, japanese, meaning, alternatives, furigana })
}

/**
 * Corregge una carta gia' scritta.
 *
 * **Non tocca lo stato di studio**: se la correzione abbia invalidato quello che si e'
 * imparato lo sa solo chi ha corretto, quindi lo decide lui.
 *
 * Torna le due cose che dicono se c'e' qualcosa da chiedere: se il testo e' davvero
 * cambiato e se la carta ha dei progressi. Servono tutte e due, perche' su una carta
 * mai studiata non c'e' niente da azzerare e su un salvataggio che non cambia niente
 * non c'e' niente da invalidare.
 */
export function updateFlashcard(
  card: string,
  japanese: string,
  meaning: string,
  alternatives: string[],
  furigana: string,
): Promise<Edited> {
  return invoke('update_flashcard', { card, japanese, meaning, alternatives, furigana })
}

/**
 * Riporta i progressi di una carta a zero, in tutti e due i versi.
 *
 * Tutti e due perche' il contenuto e' uno solo: il lato giapponese e' lo stimolo in un
 * verso e la risposta nell'altro. **Non succede mai da solo.**
 */
export function resetFlashcard(card: string): Promise<void> {
  return invoke('reset_flashcard', { card })
}

/** Elimina una carta, e con lei la sua pianificazione. */
export function deleteFlashcard(card: string): Promise<void> {
  return invoke('delete_flashcard', { card })
}

/** Cosa si troverebbe partendo adesso su questo mazzo, in questo verso. */
export function flashcardAvailability(
  scope: FlashcardScope,
): Promise<FlashcardAvailability> {
  return invoke('flashcard_availability', { scope })
}

/** Comincia un giro su un mazzo: la modalita', la coda mescolata e la prima domanda. */
export function startFlashcardSession(scope: FlashcardScope): Promise<FlashcardSession> {
  return invoke('start_flashcard_session', { scope })
}

/** Come continua il giro. Una carta sbagliata torna in coda poco piu' avanti. */
export function nextFlashcardStep(
  scope: FlashcardScope,
  queue: Queue,
  correct: boolean,
): Promise<Step> {
  return invoke('next_flashcard_step', { scope, queue, correct })
}

/**
 * Corregge una risposta **senza registrare niente**.
 *
 * Il voto arriva dopo: prima si sa se si ha indovinato, e solo allora si puo' dire
 * quanto e' costato. La conseguenza da sapere e' che una risposta lasciata a meta' non
 * viene registrata.
 */
export function checkFlashcardAnswer(
  scope: FlashcardScope,
  item: string,
  answer: string,
): Promise<Verdict> {
  return invoke('check_flashcard_answer', { scope, item, answer })
}

/**
 * Registra la risposta e, in Review, sposta la scadenza.
 *
 * `grade` e' quello che ha scelto chi ha indovinato. Su una risposta sbagliata vale
 * `again` e lo decide il core: quello che si manda da qui viene ignorato.
 */
export function submitFlashcardAnswer(
  scope: FlashcardScope,
  mode: FlashcardMode,
  item: string,
  answer: string,
  grade: Grade | null,
  responseTimeMs: number | null,
): Promise<Verdict> {
  return invoke('submit_flashcard_answer', {
    scope,
    mode,
    item,
    answer,
    grade,
    responseTimeMs,
  })
}

/**
 * Il file da compilare: la sola riga di intestazione, con le colonne in ordine.
 *
 * Lo scrive il core perche' quali colonne ci sono, e quante ne sono, e' dominio.
 */
export function flashcardImportTemplate(): Promise<string> {
  return invoke('flashcard_import_template')
}

/**
 * Guarda un CSV e dice cosa ne verrebbe, **senza scrivere niente**.
 *
 * Il testo arriva qui gia' decodificato: chi legge il file risponde alla domanda
 * «questo file e' UTF-8?», e quello che passa di qui lo e' per forza.
 */
export function checkFlashcardImport(csv: string): Promise<ImportReview> {
  return invoke('check_flashcard_import', { csv })
}

/**
 * Importa un CSV dentro un mazzo, in una transazione sola.
 *
 * Il core ricontrolla il file invece di fidarsi del controllo gia' fatto: se non va
 * torna `rejected` e non scrive nessuna riga.
 */
export function importFlashcards(deck: string, csv: string): Promise<ImportReview> {
  return invoke('import_flashcards', { deck, csv })
}

/** Cambia dopo quanti minuti torna una flashcard sbagliata. */
export function setFlashcardAgain(value: number): Promise<void> {
  return invoke('set_flashcard_again', { value })
}

/** Cambia dopo quanti minuti torna una flashcard nuova appena indovinata. */
export function setFlashcardGood(value: number): Promise<void> {
  return invoke('set_flashcard_good', { value })
}
