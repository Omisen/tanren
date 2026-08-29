import { create } from 'zustand'

import type { KanaGroup, Mode, Scope, Syllabary } from '@/shared/bridge'

/**
 * Lo stato effimero dell'interfaccia.
 *
 * Qui dentro sta solo quello che si puo' perdere chiudendo l'app: quale schermata e'
 * aperta e cosa l'utente ha selezionato adesso. I progressi, le scadenze e tutto cio'
 * che deve sopravvivere vivono nel core Rust e nel suo database, non qui.
 *
 * Se un domani si volesse ricordare l'ultimo ambito scelto tra un avvio e l'altro,
 * quella memoria andrebbe nel core, non in questo store.
 */

export type ScreenName = 'home' | 'session'

interface UiState {
  screen: ScreenName
  /** Cosa si sta per allenare, o si sta allenando. */
  scope: Scope

  goTo: (screen: ScreenName) => void
  setSyllabary: (syllabary: Syllabary) => void
  setMode: (mode: Mode) => void
  toggleGroup: (group: KanaGroup) => void
}

const initialScope: Scope = {
  syllabary: 'hiragana',
  groups: ['base'],
  mode: 'recognition',
}

export const useUi = create<UiState>((set) => ({
  screen: 'home',
  scope: initialScope,

  goTo: (screen) => set({ screen }),

  setSyllabary: (syllabary) =>
    set((s) => ({ scope: { ...s.scope, syllabary } })),

  setMode: (mode) => set((s) => ({ scope: { ...s.scope, mode } })),

  toggleGroup: (group) =>
    set((s) => {
      const selected = s.scope.groups.includes(group)
      const groups = selected
        ? s.scope.groups.filter((g) => g !== group)
        : [...s.scope.groups, group]
      return { scope: { ...s.scope, groups } }
    }),
}))
