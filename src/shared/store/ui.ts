import { create } from 'zustand'

import type {
  KanaGroup,
  KanaMode,
  KanaScope,
  Level,
  StudyMode,
  StudyScope,
  Syllabary,
} from '@/shared/bridge'

/**
 * Lo stato effimero dell'interfaccia.
 *
 * Qui dentro sta solo quello che si puo' perdere chiudendo l'app: quale schermata e'
 * aperta, quale materia si sta guardando e cosa l'utente ha selezionato adesso. I
 * progressi e tutto cio' che deve sopravvivere vivono nel core Rust e nel suo
 * database, non qui.
 *
 * Se un domani si volesse ricordare l'ultima materia o l'ultimo ambito tra un avvio e
 * l'altro, quella memoria andrebbe nel core, non in questo store.
 *
 * # Perche' i due ambiti stanno accanto e non uno solo
 *
 * Perche' non sono la stessa cosa: uno parla di sillabari e famiglie di segni, l'altro
 * di un livello del percorso e di quale delle tre modalita' si sta facendo. Tenendoli
 * separati, cambiare materia e tornare indietro ritrova la scelta di prima invece di
 * azzerarla, e nessuno dei due deve avere campi che non lo riguardano.
 */

export type ScreenName = 'home' | 'session' | 'levels' | 'about'

/**
 * In quale sezione dell'app si e'.
 *
 * Sono le quattro destinazioni di primo livello. Le tre materie erano un interruttore
 * **dentro** la schermata iniziale, e non lo sono piu': una sezione e' un posto in cui
 * si va, non un'opzione che si sceglie da qualche altra parte. Le impostazioni stanno
 * nell'elenco per la stessa ragione, e non perche' siano una materia: erano dietro
 * un'icona in un'intestazione, cioe' in un posto che cambiava da schermata a schermata.
 *
 * Quale mazzo o quale livello si sta guardando **non** sta qui: sono sguardi dentro una
 * sezione, e vivono nelle loro schermate.
 */
export type Section = 'kana' | 'kanji' | 'flashcards' | 'settings'

interface UiState {
  screen: ScreenName
  section: Section
  kana: KanaScope
  kanji: StudyScope

  goTo: (screen: ScreenName) => void
  /**
   * Cambia sezione, e riporta quella che si apre alla sua porta d'ingresso.
   *
   * **Cosa si conserva e cosa no**, ed e' una regola sola: si conserva quello che si e'
   * **scelto**, cioe' il sillabario, le famiglie, la modalita' e il livello, che stanno
   * qui accanto; non si conserva dove si era **guardato**, cioe' la griglia dei livelli
   * o un mazzo aperto. E' la stessa distinzione che il progetto fa gia' per la vista
   * dei livelli: una scelta e' una decisione, uno sguardo no.
   */
  goToSection: (section: Section) => void

  setSyllabary: (syllabary: Syllabary) => void
  setKanaMode: (mode: KanaMode) => void
  toggleGroup: (group: KanaGroup) => void

  /** Il livello che si sta guardando, che non e' per forza quello a cui si e' arrivati. */
  setLevel: (level: Level) => void
  /** Sceglie la modalita' e apre il giro. */
  study: (mode: StudyMode) => void
}

const initialKana: KanaScope = {
  syllabary: 'hiragana',
  groups: ['base'],
  mode: 'recognition',
}

const initialKanji: StudyScope = {
  level: 1,
  mode: 'learning',
}

/** Toglie o aggiunge una voce, che e' quello che fa una pastiglia premuta. */
function toggle<T>(list: T[], value: T): T[] {
  return list.includes(value) ? list.filter((v) => v !== value) : [...list, value]
}

export const useUi = create<UiState>((set) => ({
  screen: 'home',
  section: 'kana',
  kana: initialKana,
  kanji: initialKanji,

  goTo: (screen) => set({ screen }),
  goToSection: (section) => set({ section, screen: 'home' }),

  setSyllabary: (syllabary) => set((s) => ({ kana: { ...s.kana, syllabary } })),
  setKanaMode: (mode) => set((s) => ({ kana: { ...s.kana, mode } })),
  toggleGroup: (group) =>
    set((s) => ({ kana: { ...s.kana, groups: toggle(s.kana.groups, group) } })),

  setLevel: (level) => set((s) => ({ kanji: { ...s.kanji, level } })),
  study: (mode) => set((s) => ({ kanji: { ...s.kanji, mode }, screen: 'session' })),
}))
