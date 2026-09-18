import { useMemo, useRef, useState } from 'react'

import {
  checkFlashcardAnswer,
  nextFlashcardStep,
  startFlashcardSession,
  submitFlashcardAnswer,
  type FlashcardMode,
  type FlashcardScope,
} from '@/shared/bridge'
import { useSession, type Session, type SessionApi } from '@/shared/session/useSession'

/**
 * Il giro su un mazzo: la sessione condivisa, legata ai comandi di questa materia.
 *
 * Il core delle flashcard non passa da `shared::session`, perche' il suo contenuto sta
 * nel database e non nel binario, ma **i tipi che attraversano il confine sono gli
 * stessi**: quindi l'hook condiviso funziona senza sapere che dall'altra parte il giro
 * e' scritto altrove.
 *
 * # Perche' rispondere e registrare sono due chiamate
 *
 * Perche' il voto arriva **dopo**: prima si deve sapere se si ha indovinato, e solo
 * allora si puo' dire quanto e' costato. Correggere non registra niente, e la riga
 * viene scritta quando si va avanti, col voto scelto. La conseguenza da sapere e' che
 * una risposta lasciata a meta' non viene registrata, che e' la stessa cosa che gia'
 * succede al giro intero quando si esce.
 */
export function useFlashcardSession(scope: FlashcardScope): {
  session: Session
  /** In che modalita' e' partito il giro. `null` finche' non e' partito. */
  mode: FlashcardMode | null
} {
  const [mode, setMode] = useState<FlashcardMode | null>(null)

  // La modalita' serve anche fuori dal render, perche' e' quello che si rimanda al
  // core a ogni risposta, e lo stato arriva un render dopo.
  const running = useRef<FlashcardMode>('practice')

  // La risposta in sospeso fra il controllo e il voto.
  const pending = useRef<{
    item: string
    answer: string
    elapsed: number | null
  } | null>(null)

  const api = useMemo<SessionApi<FlashcardScope>>(
    () => ({
      start: async (scope) => {
        pending.current = null
        const session = await startFlashcardSession(scope)
        running.current = session.mode
        setMode(session.mode)
        return session.step
      },

      submit: (scope, question, answer, responseTimeMs) => {
        pending.current = { item: question.item, answer, elapsed: responseTimeMs }
        return checkFlashcardAnswer(scope, question.item, answer)
      },

      next: async (scope, queue, correct, grade) => {
        const answered = pending.current
        pending.current = null

        if (answered) {
          await submitFlashcardAnswer(
            scope,
            running.current,
            answered.item,
            answered.answer,
            grade ?? null,
            answered.elapsed,
          )
        }

        return nextFlashcardStep(scope, queue, correct)
      },
    }),
    [],
  )

  return { session: useSession(scope, api), mode }
}
