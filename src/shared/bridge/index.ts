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

import type {
  Credit,
  Kanji,
  KanjiCell,
  LevelSummary,
  Overview,
  StudyMode,
  StudyScope,
  StudySession,
  Level,
  Task,
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
