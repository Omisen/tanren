import { useCallback, useEffect, useRef, useState } from 'react'

import {
  nextStep,
  startSession,
  submitAnswer,
  type Queue,
  type Question,
  type Scope,
  type Step,
  type Verdict,
} from '@/shared/bridge'

/**
 * Il giro di una sessione, visto dall'interfaccia.
 *
 * Una sessione e' un giro completo sull'ambito scelto, mescolato, e un segno esce
 * dalla coda solo quando lo si indovina. La coda arriva dal core a ogni passo e
 * questo hook la conserva **senza guardarci dentro**: chi esce, chi rientra e dove lo
 * decide il core, perche' e' la regola dell'esercizio.
 *
 * # Perche' il conteggio vive qui e non nel database
 *
 * Perche' deve morire uscendo. Ogni sessione riparte da zero: quello che si e' fatto
 * l'ultima volta non deve avanzare la barra di oggi ne' togliere segni dal giro. Le
 * risposte finiscono comunque nell'archivio, ma sono storico, non stato della
 * sessione, e nessuna schermata le rilegge.
 */

/** Dove si trova la sessione in questo momento. */
export type SessionState =
  /** Piano o domanda ancora in arrivo. */
  | { phase: 'loading' }
  /** Il core non ha risposto. */
  | { phase: 'failed' }
  /** Il giro e' finito: resta il riepilogo. */
  | { phase: 'done' }
  /** C'e' una domanda aperta. */
  | { phase: 'asking'; question: Question }
  /** La risposta e' stata corretta e si sta guardando l'esito. */
  | {
      phase: 'answered'
      question: Question
      /** Quello che l'utente ha risposto, per poterlo evidenziare. */
      answer: string
      verdict: Verdict
    }

/**
 * Come sta andando il giro. Vale solo per questa sessione.
 *
 * `answered` puo' superare `total`, ed e' il punto: un segno sbagliato torna, quindi
 * si risponde piu' volte di quanti siano i segni. Quello che avanza davvero e'
 * `correct`, perche' un segno esce dalla coda solo quando lo si indovina.
 */
export interface Tally {
  /** Quante risposte sono state date, ritentativi compresi. */
  answered: number
  /** Quanti segni sono stati indovinati, cioe' tolti dalla coda. */
  correct: number
  /** Quanti segni conta il giro. */
  total: number
}

export interface Session {
  state: SessionState
  tally: Tally
  /** Vero mentre una chiamata al core e' in volo. */
  busy: boolean
  /** Vero se uscire adesso butterebbe via qualcosa. */
  dirty: boolean
  /** Manda la risposta scelta. Ignorata se non c'e' una domanda aperta. */
  answer: (value: string) => void
  /** Passa alla domanda successiva. Ignorata se non si e' appena risposto. */
  next: () => void
  /** Ricomincia: un piano nuovo sullo stesso ambito, conteggio azzerato. */
  restart: () => void
}

const EMPTY: Tally = { answered: 0, correct: 0, total: 0 }

export function useSession(scope: Scope): Session {
  const [state, write] = useState<SessionState>({ phase: 'loading' })
  const [tally, setTally] = useState<Tally>(EMPTY)
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

  // Ogni avvio ha il suo numero. Ricominciando, o uscendo dalla schermata, le
  // risposte ancora in volo appartengono a un giro che non esiste piu' e vanno
  // lasciate cadere invece che scritte sopra quello nuovo.
  const run = useRef(0)

  // La coda non si disegna mai: quello che si vede e' la domanda, e quella e' gia'
  // nello stato. Qui si tiene solo per poterla rimandare al core.
  const queue = useRef<Queue>([])

  // Quando la domanda e' comparsa, per sapere quanto ci si e' messi a rispondere.
  //
  // Sta qui e non nel core perche' e' l'unico punto che lo sa davvero: il core
  // costruisce la domanda, ma tra quel momento e il pixel acceso ci sono la risposta
  // del ponte e un render. Si legge con `performance.now()`, che e' monotono e non
  // salta se l'orologio di sistema viene spostato.
  //
  // Il valore e' **grezzo di proposito**: resta acceso anche mentre l'app sta in
  // secondo piano, quindi una domanda lasciata aperta mezz'ora produce mezz'ora. Non
  // viene tagliato, perche' scegliere ora una soglia la cuocerebbe dentro i dati per
  // sempre, mentre un valore intero si potra' sempre filtrare quando quei dati si
  // guarderanno davvero.
  const shownAt = useRef<number | null>(null)

  const install = useCallback(
    (step: Step) => {
      queue.current = step.queue
      shownAt.current = step.question ? performance.now() : null
      setState(step.question ? { phase: 'asking', question: step.question } : { phase: 'done' })
    },
    [setState],
  )

  const start = useCallback(async () => {
    const token = (run.current += 1)
    pending.current = true

    try {
      // Il primo passo arriva prima di qualunque aggiornamento di stato, cosi'
      // l'effetto che avvia la sessione non fa ripartire un render appena montato.
      const step = await startSession(scope)
      if (run.current !== token) return

      setTally({ ...EMPTY, total: step.queue.length })
      install(step)
    } catch {
      if (run.current === token) setState({ phase: 'failed' })
    } finally {
      if (run.current === token) mark(false)
    }
  }, [scope, install, mark, setState])

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

      // Si ferma il cronometro qui, sul tocco, e non quando il core risponde: il
      // ritardo del ponte non e' tempo di chi studia. Consumato subito, cosi' una
      // seconda risposta non puo' riusare un tempo che non e' il suo.
      const started = shownAt.current
      shownAt.current = null
      const elapsed = started === null ? null : Math.round(performance.now() - started)

      submitAnswer(scope, question.item, value, elapsed)
        .then((verdict) => {
          if (run.current !== token) return
          setState({ phase: 'answered', question, answer: value, verdict })
          setTally((t) => ({
            ...t,
            answered: t.answered + 1,
            correct: t.correct + (verdict.outcome === 'correct' ? 1 : 0),
          }))
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
    const now = current.current
    if (now.phase !== 'answered' || pending.current) return

    const token = run.current
    const correct = now.verdict.outcome === 'correct'
    mark(true)

    nextStep(scope, queue.current, correct)
      .then((step) => {
        if (run.current !== token) return
        install(step)
      })
      .catch(() => {
        if (run.current === token) setState({ phase: 'failed' })
      })
      .finally(() => {
        if (run.current === token) mark(false)
      })
  }, [scope, install, mark, setState])

  const restart = useCallback(() => {
    setState({ phase: 'loading' })
    void start()
  }, [start, setState])

  return {
    state,
    tally,
    busy,
    // Finito il giro non c'e' piu' niente da difendere: il riepilogo e' l'uscita.
    dirty: tally.answered > 0 && state.phase !== 'done',
    answer,
    next,
    restart,
  }
}
