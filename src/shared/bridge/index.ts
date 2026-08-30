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
  Grade,
  KanaScope,
  KanaSet,
  KanjiScope,
  KanjiSet,
  Queue,
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

/** Le famiglie di letture di un anno di scuola, con quanti item contengono. */
export function kanjiCatalogue(grade: Grade): Promise<KanjiSet[]> {
  return invoke('kanji_catalogue', { grade })
}

/** Comincia una sessione sui kanji. */
export function startKanjiSession(scope: KanjiScope): Promise<Step> {
  return invoke('start_kanji_session', { scope })
}

/** Come continua il giro sui kanji dopo una risposta. */
export function nextKanjiStep(
  scope: KanjiScope,
  queue: Queue,
  correct: boolean,
): Promise<Step> {
  return invoke('next_kanji_step', { scope, queue, correct })
}

/** Corregge una risposta sui kanji e la registra nello storico. */
export function submitKanjiAnswer(
  scope: KanjiScope,
  item: string,
  answer: string,
  responseTimeMs: number | null,
): Promise<Verdict> {
  return invoke('submit_kanji_answer', { scope, item, answer, responseTimeMs })
}
