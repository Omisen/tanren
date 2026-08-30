import { useMemo } from 'react'

import {
  nextKanjiStudyStep,
  startKanjiStudy,
  submitKanjiStudyAnswer,
  type StudyScope,
} from '@/shared/bridge'
import { useSession, type Session, type SessionApi } from '@/shared/session/useSession'

/**
 * Un giro di studio sui kanji: il giro condiviso, legato ai comandi del percorso.
 *
 * # Perche' l'avvio consegna anche i kanji da presentare
 *
 * Perche' sono la stessa chiamata. Il Learning sceglie **quali** kanji conoscere e li
 * mette in coda in un colpo solo: chiederli due volte vorrebbe dire pianificare due
 * volte, e nulla garantisce che la seconda scelga gli stessi. `onIntroducing` li passa
 * alla schermata, che li presenta prima di cominciare a interrogare.
 */
export function useKanjiSession(
  scope: StudyScope,
  onIntroducing: (kanji: string[]) => void,
): Session {
  const api = useMemo<SessionApi<StudyScope>>(
    () => ({
      start: async (s) => {
        const session = await startKanjiStudy(s)
        onIntroducing(session.introducing)
        return session.step
      },
      next: (s, queue, correct) => nextKanjiStudyStep(s.mode, queue, correct),
      submit: (s, question, answer, responseTimeMs) =>
        // La domanda porta con se' quale faccetta si sta chiedendo, e senza quella la
        // risposta finirebbe sulla carta sbagliata: dello stesso 生 si chiede il
        // significato, la lettura on e la lettura kun.
        submitKanjiStudyAnswer(
          s.mode,
          { item: question.item, exercise: question.exerciseType },
          answer,
          responseTimeMs,
        ),
    }),
    [onIntroducing],
  )

  return useSession(scope, api)
}
