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

/** In che modo ci si allena sui kana. */
export type KanaMode =
  /** Si vede il segno e si sceglie la trascrizione. */
  | 'recognition'
  /** Si vede la trascrizione e si scrive il segno con l'IME. */
  | 'input'

/** Cosa si sta allenando. `groups` vuoto significa tutte le famiglie. */
export interface KanaScope {
  syllabary: Syllabary
  groups: KanaGroup[]
  mode: KanaMode
}

export interface KanaSet {
  group: KanaGroup
  size: number
}

/** L'anno di scuola in cui un kanji si insegna. `secondary` sono medie e superiori. */
export type Grade =
  | 'first'
  | 'second'
  | 'third'
  | 'fourth'
  | 'fifth'
  | 'sixth'
  | 'secondary'

/**
 * Quale lettura di un kanji si sta allenando.
 *
 * `on` e `kun` mostrano il kanji da solo e dicono quale lettura vogliono;
 * `okurigana` mostra la forma scritta col suo okurigana (生きる), che dice gia' da se'
 * cosa chiede e infatti arriva con `asks` a `null`.
 */
export type Family = 'on' | 'kun' | 'okurigana'

/** In che modo ci si allena sui kanji. La scrittura con l'IME arriva dopo. */
export type KanjiMode = 'recognition'

/** Cosa si sta allenando. `families` vuoto significa tutte. */
export interface KanjiScope {
  grade: Grade
  families: Family[]
  mode: KanjiMode
}

export interface KanjiSet {
  family: Family
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
  /** L'identificatore dell'item, da rimandare indietro con la risposta. */
  item: string
  prompt: Prompt
  format: AnswerFormat
  /**
   * Che cosa si vuole sapere, quando lo stimolo da solo non lo dice.
   *
   * I kana non ne hanno bisogno e lo lasciano a `null`; un kanji si', perche' 生 ha
   * letture on e letture kun. **E' un'etichetta da mappare, non testo da mostrare:**
   * il core dice `on`, la schermata decide che si scrive «On reading», come gia'
   * succede per i gruppi dei kana.
   */
  asks: string | null
}

/** Una domanda aperta e la coda che resta. `question` a `null` vuol dire giro finito. */
export interface Step {
  question: Question | null
  queue: Queue
}

/**
 * Com'e' andata una risposta.
 *
 * Non porta nessuna scadenza: nessuna delle materie di oggi usa la ripetizione
 * spaziata, l'item torna al prossimo giro e basta.
 */
export type Verdict =
  | { outcome: 'correct'; note?: Note }
  /** Le risposte che sarebbero state accettate, da mostrare invece di un secco no. */
  | { outcome: 'incorrect'; accepted: string[] }

/**
 * Un rilievo su una risposta **giusta**.
 *
 * Serve a insegnare una convenzione senza punire chi non la segue: chi digita いち
 * invece di イチ ha ricordato la lettura, e trattarlo come un errore direbbe a FSRS che
 * il ricordo e' debole quando il problema era solo ortografico. Non cambia il giudizio.
 *
 * `kind` e' **un'etichetta da mappare, non testo da mostrare**, come `asks`: il core
 * dice `on_in_hiragana`, la schermata scrive la frase.
 */
export interface Note {
  kind: string
  /** Come si sarebbe scritta seguendo la convenzione. */
  expected: string
}

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
