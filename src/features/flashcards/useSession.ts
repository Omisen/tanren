import { useMemo } from 'react'

import {
  nextFlashcardStep,
  startFlashcardSession,
  submitFlashcardAnswer,
  type FlashcardScope,
} from '@/shared/bridge'
import { useSession, type Session, type SessionApi } from '@/shared/session/useSession'

/**
 * Il giro su un mazzo: la sessione condivisa, legata ai comandi di questa materia.
 *
 * Il core delle flashcard non passa da `shared::session` perche' il suo contenuto sta
 * nel database e non nel binario, ma **i tipi che attraversano il confine sono gli
 * stessi**: quindi di qua non cambia niente, e l'hook condiviso funziona senza sapere
 * che dall'altra parte il giro e' scritto altrove.
 *
 * `next` riceve anche se la risposta era giusta e **non lo passa**: qui un giro passa
 * una volta sola su ogni carta.
 */
export function useFlashcardSession(scope: FlashcardScope): Session {
  const api = useMemo<SessionApi<FlashcardScope>>(
    () => ({
      start: startFlashcardSession,
      next: (scope, queue) => nextFlashcardStep(scope, queue),
      submit: (scope, question, answer, responseTimeMs) =>
        submitFlashcardAnswer(scope, question.item, answer, responseTimeMs),
    }),
    [],
  )

  return useSession(scope, api)
}
