/**
 * I tipi che attraversano il confine con il core Rust.
 *
 * Sono scritti a mano e rispecchiano i tipi in `crates/core`. A tenerli allineati
 * ci pensano i test in `crates/core/tests/dto_shape.rs`, che fissano il JSON
 * prodotto da ogni tipo: chi rinomina un campo in Rust vede fallire un test e sa che
 * deve passare anche di qui.
 */

export type Syllabary = 'hiragana' | 'katakana'

/** Le famiglie di segni, dalle piu' semplici alle combinazioni. */
export type KanaGroup = 'base' | 'dakuten' | 'handakuten' | 'yoon'

/** In che modo si sta allenando. */
export type Mode =
  /** Si vede il segno e si sceglie la trascrizione. */
  | 'recognition'
  /** Si vede la trascrizione e si scrive il segno con l'IME. */
  | 'input'

/** Cosa si sta allenando. `groups` vuoto significa tutte le famiglie. */
export interface Scope {
  syllabary: Syllabary
  groups: KanaGroup[]
  mode: Mode
}

export interface KanaSet {
  group: KanaGroup
  size: number
}

/**
 * Cosa mostrare, con l'indicazione di come va scritto.
 *
 * La distinzione non e' estetica: il giapponese vuole un font e un corpo diversi, e
 * l'attributo `lang` giusto perche' il browser scelga le forme corrette.
 */
export type Prompt =
  | { script: 'japanese'; text: string }
  | { script: 'latin'; text: string }

/** In che modo si risponde. */
export type AnswerFormat =
  /** Le opzioni arrivano gia' mescolate e comprendono quella giusta. */
  | { mode: 'choice'; options: string[] }
  /** Digitazione libera con l'IME del dispositivo. */
  | { mode: 'input' }

/**
 * Cosa resta da fare in una sessione.
 *
 * E' **opaca**: si conserva e si rimanda indietro al core alla chiamata successiva,
 * non si guarda dentro e non si modifica. Chi esce, chi rientra e dove lo decide il
 * core, perche' e' la regola dell'esercizio e non un dettaglio di presentazione.
 */
export type Queue = string[]

export interface Question {
  exerciseType: string
  /** L'identificatore del segno, da rimandare indietro con la risposta. */
  item: string
  prompt: Prompt
  format: AnswerFormat
}

/** Una domanda aperta e la coda che resta. `question` a `null` vuol dire giro finito. */
export interface Step {
  question: Question | null
  queue: Queue
}

/**
 * Com'e' andata una risposta.
 *
 * Non porta nessuna scadenza: sui kana non c'e' ripetizione spaziata, il segno torna
 * al prossimo giro e basta.
 */
export type Verdict =
  | { outcome: 'correct' }
  /** Le risposte che sarebbero state accettate, da mostrare invece di un secco no. */
  | { outcome: 'incorrect'; accepted: string[] }

/** Un errore arrivato dal core. Il campo `kind` dice di che si tratta. */
export type CoreError =
  | { kind: 'unknown_item'; id: string }
  | { kind: 'item_not_supported'; exercise: string; id: string }
  | { kind: 'storage'; message: string }
  | { kind: 'scheduling'; message: string }

/** Riconosce un errore del core tra quelli che possono arrivare da `invoke`. */
export function isCoreError(error: unknown): error is CoreError {
  return typeof error === 'object' && error !== null && 'kind' in error
}
