import { useMemo } from 'react'

import {
  nextKanjiStep,
  startKanjiSession,
  submitKanjiAnswer,
  type KanjiScope,
} from '@/shared/bridge'
import { useSession, type Session, type SessionApi } from '@/shared/session/useSession'

/** La sessione sui kanji: il giro condiviso, legato ai comandi di questa materia. */
export function useKanjiSession(scope: KanjiScope): Session {
  const api = useMemo<SessionApi<KanjiScope>>(
    () => ({
      start: startKanjiSession,
      next: nextKanjiStep,
      submit: submitKanjiAnswer,
    }),
    [],
  )

  return useSession(scope, api)
}
