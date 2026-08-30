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

/* --- Il percorso sui kanji: livelli, faccette, tre modalita' --------------- */

/** Un livello del percorso, da 1 a 69. */
export type Level = number

/**
 * In che modo si sta studiando.
 *
 * Non sono tre sistemi separati: e' lo stesso giro configurato in modo diverso, e le
 * differenze sono da dove pesca, se rifa' chi sbaglia, e se nutre FSRS.
 */
export type StudyMode =
  /** Si conoscono kanji nuovi, ed e' qui che le carte nascono. */
  | 'learning'
  /** Si rivede cio' che sta per essere dimenticato. Lo decide FSRS. */
  | 'review'
  /** Pratica a volonta' su quello che si e' gia' visto. **Non** tocca le scadenze. */
  | 'drill'

export interface StudyScope {
  mode: StudyMode
  level: Level
}

/** Perche' non si puo' imparare altro adesso. */
export type Blocked =
  /** Quello che c'e' gia' non regge abbastanza. E' il freno che conta. */
  | { reason: 'consolidate'; current: number; needed: number }
  /** Si e' introdotto troppo di recente. */
  | { reason: 'too_soon'; until: string }
  /** La quota di oggi e' finita. */
  | { reason: 'daily_cap'; done: number; cap: number }
  /** Non c'e' piu' niente di nuovo in questo livello. */
  | { reason: 'nothing_new' }

/**
 * Se si puo' imparare, e altrimenti perche' no.
 *
 * Il motivo non e' un dettaglio: dire «consolida quello che hai» e «torna fra quattro
 * ore» sono due consigli diversi, e chi studia ha diritto di sapere quale vale.
 */
export type Gate = { state: 'open'; room: number } | ({ state: 'closed' } & Blocked)

/** A che punto e' un livello. */
export interface LevelProgress {
  level: Level
  total: number
  new: number
  learning: number
  mature: number
  /** La quota di kanji maturi, da 0 a 1. */
  ratio: number
  /** Se il livello e' abbastanza consolidato da aprire il successivo. */
  complete: boolean
}

/** Cosa si puo' fare adesso. */
export interface Available {
  learning: Gate
  /** Quante faccette sono scadute, di qualunque livello. */
  due: number
  /** Su quante faccette si puo' praticare. */
  practiced: number
}

export interface Overview {
  progress: LevelProgress
  available: Available
}

/** Un giro appena cominciato. */
export interface StudySession {
  /** I kanji da presentare prima di interrogare. Vuoto fuori dal Learning. */
  introducing: string[]
  step: Step
}

/** In che stato e' un kanji. */
export type Standing =
  /** Mai introdotto. */
  | 'new'
  /** Introdotto, con almeno una faccetta ancora acerba. */
  | 'learning'
  /** Tutte le faccette attive hanno superato la soglia. */
  | 'mature'

export interface KanjiCell {
  character: string
  standing: Standing
}

/** Una forma scritta col suo okurigana: 生きる, che si legge いきる. */
export interface Okurigana {
  form: string
  readings: string[]
  /** Se e' una parola che si incontra davvero. */
  common: boolean
}

export interface Example {
  word: string
  reading: string
  meaning: string
}

/** Un kanji, con tutto quello che serve a impararlo. */
export interface Kanji {
  character: string
  strokes: number
  level: Level
  frequency: number | null
  /** Quanto ricorre da solo invece che dentro un composto, da 0 a 1. */
  aloneRatio: number | null
  /** I significati: **il primo e' il primario**. */
  meanings: string[]
  on: string[]
  onRare?: string[]
  /** La lettura on che pesa di piu' nei composti veri. */
  primaryOn: string | null
  kun: string[]
  kunRare?: string[]
  okurigana: Okurigana[]
  /** Le letture nei nomi propri: si mostrano, non si chiedono. */
  nanori: string[]
  examples: Example[]
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
export type Queue = Task[]

/**
 * Una domanda da fare: su quale item, e che cosa se ne chiede.
 *
 * Un giro puo' mescolare esercizi diversi. Sui kana no, sono tutti uguali; sui kanji
 * si', perche' dello stesso 生 si chiede il significato, la lettura on e la lettura
 * kun, e sono tre domande con tre carte e tre scadenze.
 */
export interface Task {
  item: string
  exercise: string
}

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
