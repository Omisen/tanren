/**
 * L'unico punto da cui il frontend parla con il core Rust.
 *
 * Nessun altro modulo dovrebbe importare `invoke` direttamente: tenendo le chiamate
 * qui, i nomi dei comandi e le loro firme stanno in un posto solo, e la UI vede
 * funzioni tipizzate invece di stringhe.
 */

import { invoke } from '@tauri-apps/api/core'

import type { KanaSet, Queue, Scope, Step, Syllabary, Verdict } from './types'

export * from './types'

/** Le famiglie di un sillabario, con quanti segni contengono. */
export function kanaCatalogue(syllabary: Syllabary): Promise<KanaSet[]> {
  return invoke('kana_catalogue', { syllabary })
}

/**
 * Riduce un testo alla forma con cui verra' confrontato.
 *
 * Serve a mostrare in tempo reale cosa sara' davvero valutato, mentre l'IME e' ancora
 * in mezzo alla conversione.
 */
export function normalizeReading(input: string): Promise<string> {
  return invoke('normalize_reading', { input })
}

/** Comincia una sessione: la coda mescolata e la prima domanda. */
export function startSession(scope: Scope): Promise<Step> {
  return invoke('start_session', { scope })
}

/**
 * Come continua il giro dopo una risposta.
 *
 * La coda torna al core com'era arrivata: chi esce e chi rientra lo decide lui.
 */
export function nextStep(scope: Scope, queue: Queue, correct: boolean): Promise<Step> {
  return invoke('next_step', { scope, queue, correct })
}

/** Corregge una risposta e la registra nello storico. */
export function submitAnswer(
  scope: Scope,
  item: string,
  answer: string,
): Promise<Verdict> {
  return invoke('submit_answer', { scope, item, answer })
}
