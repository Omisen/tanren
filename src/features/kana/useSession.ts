import { useCallback, useEffect, useRef, useState } from 'react'

import {
  nextQuestion,
  prepareSession,
  sessionProgress,
  submitAnswer,
  type Outcome,
  type Progress,
  type Question,
  type Scope,
} from '@/shared/bridge'

/**
 * Il giro di una sessione, visto dall'interfaccia.
 *
 * Qui non si decide niente di dominio: cosa chiedere, se la risposta e' giusta e
 * quando il segno tornera' lo stabilisce il core. Questo modulo si limita a mettere
 * in fila le chiamate e a ricordare a che punto e' il giro, che e' stato effimero
 * quanto la schermata che lo mostra.
 *
 * Non sta nello store globale proprio per questo: uscire dalla sessione deve
 * dimenticare tutto, e non c'e' niente da salvare che il database non abbia gia'.
 */

/** Dove si trova la sessione in questo momento. */
export type SessionState =
  /** Prima domanda ancora in arrivo. */
  | { phase: 'loading' }
  /** Il core non ha risposto. */
  | { phase: 'failed' }
  /** Per adesso non c'e' altro da ripassare. */
  | { phase: 'done' }
  /** C'e' una domanda aperta. */
  | { phase: 'asking'; question: Question }
  /** La risposta e' stata corretta e si sta guardando l'esito. */
  | {
      phase: 'answered'
      question: Question
      /** Quello che l'utente ha risposto, per poterlo evidenziare. */
      answer: string
      outcome: Outcome
    }

export interface Session {
  state: SessionState
  /** A che punto e' l'ambito, o `null` finche' non lo si sa. */
  progress: Progress | null
  /** Vero mentre una chiamata al core e' in volo. */
  busy: boolean
  /** Manda la risposta scelta. Ignorata se non c'e' una domanda aperta. */
  answer: (value: string) => void
  /** Passa alla domanda successiva. Ignorata se non si e' appena risposto. */
  next: () => void
  /** Ricomincia da capo, dopo un errore. */
  retry: () => void
}

export function useSession(scope: Scope): Session {
  const [state, write] = useState<SessionState>({ phase: 'loading' })
  const [progress, setProgress] = useState<Progress | null>(null)
  const [busy, showBusy] = useState(false)

  // Lo stato serve anche fuori dal render, per rifiutare i tocchi che arrivano
  // quando non e' il loro momento: due tocchi rapidi su due opzioni diverse
  // manderebbero altrimenti due risposte per la stessa domanda.
  const current = useRef(state)
  const setState = useCallback((next: SessionState) => {
    current.current = next
    write(next)
  }, [])

  // `busy` come stato serve al render, ma il render arriva troppo tardi per fermare
  // il secondo tocco: la guardia vera e' il riferimento, aggiornato all'istante.
  const pending = useRef(false)
  const mark = useCallback((value: boolean) => {
    pending.current = value
    showBusy(value)
  }, [])

  // Ogni avvio ha il suo numero. Cambiando ambito, o uscendo dalla schermata, le
  // risposte ancora in volo appartengono a un giro che non esiste piu' e vanno
  // lasciate cadere invece che scritte sopra quello nuovo.
  const run = useRef(0)

  const ask = useCallback(
    async (token: number) => {
      mark(true)
      try {
        const [question, p] = await Promise.all([
          nextQuestion(scope),
          sessionProgress(scope),
        ])
        if (run.current !== token) return
        setProgress(p)
        setState(question ? { phase: 'asking', question } : { phase: 'done' })
      } catch {
        if (run.current === token) setState({ phase: 'failed' })
      } finally {
        if (run.current === token) mark(false)
      }
    },
    [scope, mark, setState],
  )

  const start = useCallback(async () => {
    const token = (run.current += 1)
    pending.current = true

    try {
      // Preparare crea le carte mancanti: un ambito mai studiato non avrebbe
      // altrimenti nessuna domanda da dare. E' anche la prima cosa che accade, prima
      // di qualunque aggiornamento di stato, perche' l'effetto che avvia la sessione
      // non deve far ripartire un render appena montato.
      const p = await prepareSession(scope)
      if (run.current !== token) return
      setProgress(p)
    } catch {
      if (run.current === token) {
        setState({ phase: 'failed' })
        mark(false)
      }
      return
    }

    await ask(token)
  }, [scope, ask, mark, setState])

  useEffect(() => {
    // La sessione vive nel core: montare la schermata e' proprio il momento in cui
    // l'interfaccia si sincronizza con qualcosa che sta fuori da React, che e' il
    // caso per cui gli effetti esistono. Il primo stato lo scrive il core, non il
    // render, quindi non c'e' niente da derivare durante il render.
    // oxlint-disable-next-line react/set-state-in-effect
    void start()

    // Uscendo dalla schermata quello che e' ancora in volo diventa irrilevante.
    return () => {
      run.current += 1
    }
  }, [start])

  const answer = useCallback(
    (value: string) => {
      const now = current.current
      if (now.phase !== 'asking' || pending.current) return

      const token = run.current
      const { question } = now
      mark(true)

      submitAnswer(scope, question.item, value)
        .then((outcome) => {
          if (run.current !== token) return
          setState({ phase: 'answered', question, answer: value, outcome })
        })
        .catch(() => {
          if (run.current === token) setState({ phase: 'failed' })
        })
        .finally(() => {
          if (run.current === token) mark(false)
        })
    },
    [scope, mark, setState],
  )

  const next = useCallback(() => {
    if (current.current.phase !== 'answered' || pending.current) return
    void ask(run.current)
  }, [ask])

  const retry = useCallback(() => {
    setState({ phase: 'loading' })
    void start()
  }, [start, setState])

  return { state, progress, busy, answer, next, retry }
}
