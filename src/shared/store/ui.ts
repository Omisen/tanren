import { create } from 'zustand'

import type {
  Family,
  Grade,
  KanaGroup,
  KanaMode,
  KanaScope,
  KanjiScope,
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
 * Perche' non sono la stessa cosa: uno parla di sillabari, l'altro di anni di scuola.
 * Tenendoli separati, cambiare materia e tornare indietro ritrova la scelta di prima
 * invece di azzerarla, e nessuno dei due deve avere campi che non lo riguardano.
 */

export type ScreenName = 'home' | 'session'

/** Quale materia si sta guardando. */
export type Subject = 'kana' | 'kanji'

interface UiState {
  screen: ScreenName
  subject: Subject
  kana: KanaScope
  kanji: KanjiScope

  goTo: (screen: ScreenName) => void
  setSubject: (subject: Subject) => void

  setSyllabary: (syllabary: Syllabary) => void
  setKanaMode: (mode: KanaMode) => void
  toggleGroup: (group: KanaGroup) => void

  setGrade: (grade: Grade) => void
  toggleFamily: (family: Family) => void
}

const initialKana: KanaScope = {
  syllabary: 'hiragana',
  groups: ['base'],
  mode: 'recognition',
}

const initialKanji: KanjiScope = {
  grade: 'first',
  families: ['on'],
  mode: 'recognition',
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

  setGrade: (grade) => set((s) => ({ kanji: { ...s.kanji, grade } })),
  toggleFamily: (family) =>
    set((s) => ({ kanji: { ...s.kanji, families: toggle(s.kanji.families, family) } })),
}))
