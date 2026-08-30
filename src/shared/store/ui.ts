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

export type ScreenName = 'home' | 'session'

/** Quale materia si sta guardando. */
export type Subject = 'kana' | 'kanji'

interface UiState {
  screen: ScreenName
  subject: Subject
  kana: KanaScope
  kanji: StudyScope

  goTo: (screen: ScreenName) => void
  setSubject: (subject: Subject) => void

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
  subject: 'kana',
  kana: initialKana,
  kanji: initialKanji,

  goTo: (screen) => set({ screen }),
  setSubject: (subject) => set({ subject }),

  setSyllabary: (syllabary) => set((s) => ({ kana: { ...s.kana, syllabary } })),
  setKanaMode: (mode) => set((s) => ({ kana: { ...s.kana, mode } })),
  toggleGroup: (group) =>
    set((s) => ({ kana: { ...s.kana, groups: toggle(s.kana.groups, group) } })),

  setLevel: (level) => set((s) => ({ kanji: { ...s.kanji, level } })),
  study: (mode) => set((s) => ({ kanji: { ...s.kanji, mode }, screen: 'session' })),
}))
