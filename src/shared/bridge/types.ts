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
export type KanaGroup = 'base' | 'dakuten' | 'handakuten' | 'yoon' | 'gairaion'

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

/**
 * La colonna della tavola: a quale vocale appartiene un segno.
 *
 * Serve a disporre i segni come stanno nella tavola vera, buchi compresi. Chi la
 * decide e' il core: dedurre qui che を sta nella colonna o vorrebbe dire mettere la
 * tavola del gojuon dalla parte sbagliata del confine.
 */
export type Vowel = 'a' | 'i' | 'u' | 'e' | 'o'

/** Un segno nel catalogo, con quello che serve a disegnarlo in tavola. */
export interface KanaCell {
  character: string
  /** La trascrizione canonica, da mostrare sotto il segno. */
  romaji: string
  /** In quale colonna va messo. `null` solo per ん, che sta da solo. */
  column: Vowel | null
}

/** Una riga della tavola. `row` non e' unico fra famiglie: lo yoon riusa `ka`, `ga`... */
export interface KanaRow {
  row: string
  cells: KanaCell[]
}

export interface KanaSet {
  group: KanaGroup
  /** Quanti segni contiene: e' quello che alimenta il conteggio sul bottone di avvio. */
  size: number
  rows: KanaRow[]
}

/**
 * Le due regole di scrittura che si mostrano e non si chiedono.
 *
 * Non sono famiglie dell'ambito e **non sono un `KanaGroup`**, che e' il tipo che vive
 * dentro `KanaScope.groups`: se lo fossero diventerebbero selezionabili, entrerebbero
 * nel conteggio e potrebbero finire in una sessione. Qui l'isolamento sta nel tipo.
 */
export type PatternGroup =
  /** Il sokuon: っ raddoppia la consonante che segue. */
  | 'double'
  /** Le vocali lunghe. */
  | 'long'

export interface PatternCell {
  /** Quello che si vede, che nel sokuon non e' nemmeno tutto giapponese: `っ+k`. */
  character: string
  romaji: string
  /**
   * In quale colonna va messa, e `null` quando non ne ha una.
   *
   * A differenza delle tabelle, qui la colonna arriva **scritta nel dato**: えい e' una
   * e lunga con la trascrizione che finisce per `i`, quindi ricavarla dal romaji la
   * metterebbe nella colonna sbagliata.
   */
  column: Vowel | null
}

/** Una regola, con le sue caselle disposte in righe. Le righe non hanno un nome. */
export interface KanaPattern {
  group: PatternGroup
  rows: PatternCell[][]
}

/* --- Il percorso sui kanji: livelli, faccette, tre modalita' --------------- */

/** Un livello del percorso. Quanti siano lo dice la dashboard, una riga per livello. */
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
  /**
   * Bisogna aspettare, e `until` e' l'istante in cui si riapre.
   *
   * Un motivo solo per i due freni a tempo, il floor e la quota: danno lo stesso
   * consiglio, e `until` e' il piu' lontano dei due che mordono.
   */
  | { reason: 'wait'; until: string }
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

/**
 * Una riga della dashboard: un livello, con quanto regge e se e' aperto.
 *
 * I campi dell'avanzamento arrivano appiattiti dentro la riga, non annidati.
 */
export interface LevelSummary extends LevelProgress {
  /** Quanto reggono adesso le faccette che si stanno portando avanti, da 0 a 1. */
  recall: number | null
  /** Se il livello si puo' studiare, cioe' se i precedenti sono consolidati. */
  unlocked: boolean
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
  /**
   * Quanto e' consolidato, da 0 a 1, e `null` se non e' mai stato incontrato: li' non
   * c'e' un progresso basso, non c'e' niente da misurare.
   *
   * E' la **stabilita' di FSRS** rapportata alla soglia di maturita', presa sulla
   * faccetta piu' debole, non un tasso di risposte giuste. Non cala da sola col tempo,
   * perche' la stabilita' cambia solo quando si risponde.
   */
  progress: number | null
}

/** Una forma scritta col suo okurigana: 生きる, che si legge いきる. */
export interface Okurigana {
  form: string
  /** Come si legge la forma intera: 生きる fa いきる. E' quello che si mostra. */
  readings: string[]
  /**
   * Come si legge la sola parte coperta dal kanji: 生きる fa い.
   *
   * E' quello su cui l'esercizio giudica, perche' l'okurigana sta gia' nella
   * domanda e chiederlo vorrebbe dire farlo ricopiare.
   */
  stem: string[]
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
  /** La kun con cui il kanji si legge piu' spesso da solo. Quasi sempre `null`. */
  primaryKun: string | null
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
  /**
   * La porzione dello stimolo su cui verte la domanda, quando non e' tutto.
   *
   * E' un **prefisso** di `prompt.text`: su 大きい vale 大, perche' quello che si
   * chiede e' come si legge il kanji e il きい e' li' a dire quale lettura vale. Si usa
   * per dividere in due lo stimolo, non per stamparlo un'altra volta. A `null` la
   * domanda verte su tutto quello che si vede.
   */
  focus: string | null
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

/**
 * Una fonte, con quello che serve a darle credito.
 *
 * Non e' una schermata di cortesia: la CC BY-SA e la licenza dell'EDRDG obbligano ad
 * attribuire **dentro il mezzo in cui l'opera viaggia**, e per un'app quel mezzo e'
 * l'APK, non il README della repo.
 */
export interface Credit {
  name: string
  /** Che cosa di quello che si vede nell'app viene da qui. */
  covers: string
  /** La frase esatta che quella fonte chiede di riportare. Non si parafrasa. */
  notice: string | null
  licence: string
  licenceUrl: string
  /** Il testo della licenza imbarcato, se c'e'. Altrimenti resta il link. */
  licenceFile: string | null
  sourceUrl: string | null
  /** Quale edizione esatta di quella fonte l'app sta spedendo. */
  edition: string | null
}

/** Un errore arrivato dal core. Il campo `kind` dice di che si tratta. */
export type CoreError =
  | { kind: 'unknown_item'; id: string }
  | { kind: 'item_not_supported'; exercise: string; id: string }
  | { kind: 'storage'; message: string }
  | { kind: 'scheduling'; message: string }
  /** Un campo che vuole del testo e' arrivato vuoto. `field` dice quale. */
  | { kind: 'empty_field'; field: string }
  /** Sono arrivati piu' valori di quanti se ne accettino. */
  | { kind: 'too_many_values'; field: string; max: number }

/** Riconosce un errore del core tra quelli che possono arrivare da `invoke`. */
export function isCoreError(error: unknown): error is CoreError {
  return typeof error === 'object' && error !== null && 'kind' in error
}

/**
 * Le preferenze, col loro intervallo valido.
 *
 * Ce n'e' una sola perche' una sola e' stata aperta: le altre grandezze del ritmo
 * restano decisioni del progetto finche' non si dimostra che vadano scelte.
 */
export interface Settings {
  /** Quanti kanji nuovi per lezione. */
  dailyNew: number
  dailyNewMin: number
  dailyNewMax: number
  /** Dopo quanti minuti torna una flashcard sbagliata. */
  flashcardAgain: number
  flashcardAgainMin: number
  flashcardAgainMax: number
  /** Dopo quanti minuti torna una flashcard nuova appena indovinata. */
  flashcardGood: number
  flashcardGoodMin: number
  flashcardGoodMax: number
  /**
   * Quanti significati si accettano oltre a quello principale.
   *
   * Non e' una preferenza: e' un limite di dominio, e viaggia insieme agli altri
   * perche' il modulo di scrittura deve sapere quando smettere di offrire caselle.
   */
  flashcardMaxAlternatives: number
}

/* --- Le flashcard: i mazzi e le carte che l'utente si scrive --------------- */

/** Un mazzo, come lo vede chi lo ha creato. */
export interface Deck {
  id: string
  name: string
}

/**
 * Un mazzo nell'elenco, con quante carte contiene.
 *
 * Il conteggio non e' una proprieta' del mazzo ma una cosa che si calcola: per questo
 * e' un tipo a se' e non un campo in piu' su `Deck`.
 */
export interface DeckSummary extends Deck {
  cards: number
}

/**
 * Una carta: il giapponese da una parte, il significato dall'altra.
 *
 * Il testo e' quello che l'utente ha scritto, non normalizzato: la normalizzazione
 * serve al confronto e si fa al momento del confronto.
 *
 * # Le risposte in piu' sono due campi, e servono a due domande diverse
 *
 * `alternatives` vale dove si risponde **col significato**: sono traduzioni.
 * `furigana` vale dove si risponde **in giapponese**: e' la stessa parola scritta in
 * kana. Non sono intercambiabili, e il core non li mescola.
 */
export interface Flashcard {
  id: string
  deckId: string
  japanese: string
  /** Il significato principale, quello che si mostra. */
  meaning: string
  /** Gli altri significati accettati, nell'ordine in cui sono stati scritti. */
  alternatives: string[]
  /**
   * La lettura dell'intera parola o frase, quando contiene kanji.
   *
   * La scrive chi crea la carta: la lettura di un kanji dipende dal contesto, e nessun
   * derivatore automatico e' affidabile.
   */
  furigana: string | null
}

/**
 * In che verso va la domanda su una carta.
 *
 * Sono **due carte di studio distinte** e non due modi di guardare la stessa:
 * riconoscere e produrre si imparano in tempi diversi, come il riconoscimento e la
 * scrittura sui kana.
 */
export type FlashcardDirection =
  /** Si vede il giapponese e si risponde col significato. */
  | 'jp_to_meaning'
  /** Si vede il significato e si scrive il giapponese. */
  | 'meaning_to_jp'

/** Cosa si sta studiando: quale mazzo, e in che verso. */
export interface FlashcardScope {
  deck: string
  direction: FlashcardDirection
}

/**
 * Com'e' andata una risposta, nella scala a quattro gradini di FSRS.
 *
 * `again` non si sceglie: e' quello che vale su una risposta sbagliata, e lo decide il
 * core. Gli altri tre sono i bottoni fra cui sceglie chi ha indovinato, perche' il
 * sistema non puo' sapere se e' costato fatica o niente.
 */
export type Grade = 'again' | 'hard' | 'good' | 'easy'

/**
 * In che modo si sta studiando un mazzo.
 *
 * Non si sceglie: lo decide il core guardando cosa e' dovuto. Sono la risposta a due
 * domande diverse, e una sola delle due ha senso in un dato momento.
 */
export type FlashcardMode =
  /** C'e' qualcosa di dovuto: si ripassa quello, e i voti spostano le scadenze. */
  | 'review'
  /** Non e' dovuto niente: si ripassa tutto il mazzo, e l'algoritmo non se ne accorge. */
  | 'practice'

/** Cosa si troverebbe partendo adesso. */
export interface FlashcardAvailability {
  /** Quante carte sono dovute adesso, mai viste comprese. */
  due: number
  /** Quante carte ha il mazzo in tutto. */
  total: number
}

/** Un giro appena cominciato. La modalita' vale per tutto il giro. */
export interface FlashcardSession {
  mode: FlashcardMode
  step: Step
}

/**
 * Cosa dice un CSV da importare.
 *
 * Lo stesso tipo serve a guardare e a importare: `cards` sono le carte che
 * **verrebbero** nel primo caso e quelle **scritte** nel secondo.
 *
 * **Un rifiuto non e' un errore del ponte**, e' un esito: il file e' arrivato e si e'
 * capito cosa contiene, ed e' che non si puo' usare.
 */
export type ImportReview =
  | { state: 'ready'; cards: number }
  /** Nessuna riga e' stata scritta: o tutto o niente. */
  | { state: 'rejected'; errors: ImportRowError[] }

/** Una riga del file che non va, e perche'. */
export interface ImportRowError {
  /**
   * La riga **del file**, intestazione compresa: e' il numero che si legge nel foglio
   * di calcolo, cioe' il posto in cui chi corregge deve andare.
   */
  line: number
  problem: ImportProblem
}

/**
 * Cosa c'e' che non va in una riga.
 *
 * Arriva come **etichetta da mappare** e non come frase: la lingua dell'interfaccia non
 * entra nel core, come gia' per `asks` e per i motivi di `Gate`.
 */
export type ImportProblem =
  /** La prima riga non porta i nomi delle colonne che servono. */
  | { kind: 'header' }
  /** Un campo obbligatorio e' vuoto. `field` dice quale. */
  | { kind: 'empty_field'; field: string }
  /** Sono arrivati piu' valori di quanti se ne accettino. */
  | { kind: 'too_many_values'; field: string; max: number }
  /** La riga ha piu' colonne dell'intestazione: di solito una virgola non protetta. */
  | { kind: 'too_many_columns'; found: number; expected: number }
  /** Il file non si legge affatto: virgolette non chiuse, o simili. */
  | { kind: 'malformed' }

/** Com'e' andata una correzione, cioe' se c'e' qualcosa da chiedere. */
export interface Edited {
  /**
   * Se il testo e' davvero cambiato, **dopo** la pulizia.
   *
   * Lo dice il core e non la schermata, perche' il confronto va fatto su quello che e'
   * stato scritto davvero: una casella vuota, uno spazio ai bordi o un doppione non
   * sono una modifica, e quella regola vive di la'.
   */
  changed: boolean
  /** Se la carta ha dei progressi, cioe' se c'e' qualcosa da azzerare. */
  studied: boolean
}
