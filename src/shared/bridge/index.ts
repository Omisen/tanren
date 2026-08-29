/**
 * L'unico punto da cui il frontend parla con il core Rust.
 *
 * Nessun altro modulo dovrebbe importare `invoke` direttamente: tenendo le chiamate
 * qui, i nomi dei comandi e le loro firme stanno in un posto solo, e la UI vede
 * funzioni tipizzate invece di stringhe.
 */

import { invoke } from '@tauri-apps/api/core'

import type { KanaSet, Outcome, Progress, Question, Scope, Syllabary } from './types'

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

/** Prepara l'ambito scelto, creando le carte che ancora non esistono. */
export function prepareSession(scope: Scope): Promise<Progress> {
  return invoke('prepare_session', { scope })
}

/** A che punto e' l'ambito, senza modificare niente. */
export function sessionProgress(scope: Scope): Promise<Progress> {
  return invoke('session_progress', { scope })
}

/** La prossima domanda, o `null` se per adesso non c'e' altro da ripassare. */
export function nextQuestion(scope: Scope): Promise<Question | null> {
  return invoke('next_question', { scope })
}

/** Corregge una risposta, ripianifica il segno e registra tutto. */
export function submitAnswer(
  scope: Scope,
  item: string,
  answer: string,
): Promise<Outcome> {
  return invoke('submit_answer', { scope, item, answer })
}
