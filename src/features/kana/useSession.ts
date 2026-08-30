import { useMemo } from 'react'

import {
  nextKanaStep,
  startKanaSession,
  submitKanaAnswer,
  type KanaScope,
} from '@/shared/bridge'
import { useSession, type Session, type SessionApi } from '@/shared/session/useSession'

/**
 * La sessione sui kana: il giro condiviso, legato ai comandi di questa materia.
 *
 * Tutto quello che c'era qui dentro (le guardie, il conteggio, il cronometro della
 * risposta) e' salito in `shared/session`, perche' non sapeva niente di kana. Qui
 * resta il collegamento fra l'ambito di questa materia e i suoi tre comandi.
 */
export function useKanaSession(scope: KanaScope): Session {
  const api = useMemo<SessionApi<KanaScope>>(
    () => ({
      start: startKanaSession,
      next: nextKanaStep,
      submit: submitKanaAnswer,
    }),
    [],
  )

  return useSession(scope, api)
}
