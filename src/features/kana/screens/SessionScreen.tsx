import { useEffect, useState } from 'react'

import type { Mode, Prompt, Verdict } from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Confirm } from '@/shared/ui/Confirm'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

import { useSession, type SessionState, type Tally } from '../useSession'

/**
 * La sessione di studio: un giro completo sull'ambito scelto, in ordine casuale, dove
 * un segno esce dalla coda solo quando lo si indovina.
 *
 * Tutto quello che conta lo decide il core: quale segno tocca, se la risposta e'
 * giusta e cosa registrare. Qui si mostra e si raccoglie il tocco.
 *
 * Le opzioni stanno in fondo, nella fascia delle azioni, perche' sono la cosa che si
 * tocca decine di volte di fila: il pollice deve trovarle senza spostare la mano. Il
 * segno da leggere resta in mezzo, dove cade lo sguardo.
 */

/** Quanto resta visibile l'esito di una risposta giusta, prima di andare avanti. */
const ADVANCE_MS = 700

const MODE_LABELS: Record<Mode, string> = {
  recognition: 'Riconoscimento',
  input: 'Scrittura',
}

export function SessionScreen() {
  const { scope, goTo } = useUi()
  const { state, tally, busy, dirty, answer, next, restart } = useSession(scope)
  const [leaving, setLeaving] = useState(false)

  // Una risposta giusta non ha niente da leggere: si va avanti da soli dopo un
  // istante, quel tanto che basta a vedere il verde. Una sbagliata invece aspetta,
  // perche' la lettura corretta va letta.
  // Con la conferma di uscita aperta il giro sta fermo: il conteggio di cui parla la
  // domanda non deve cambiare mentre la si legge.
  const correct = state.phase === 'answered' && state.verdict.outcome === 'correct'
  useEffect(() => {
    if (!correct || leaving) return
    const timer = setTimeout(next, ADVANCE_MS)
    return () => clearTimeout(timer)
  }, [correct, leaving, next])

  return (
    <>
      <Screen
        title={MODE_LABELS[scope.mode]}
        onBack={() => (dirty ? setLeaving(true) : goTo('home'))}
        action={
          <Actions
            state={state}
            tally={tally}
            busy={busy}
            onAnswer={answer}
            onNext={next}
            onRestart={restart}
            onHome={() => goTo('home')}
          />
        }
      >
        {state.phase === 'loading' && <Centered>Preparo il giro…</Centered>}

        {state.phase === 'failed' && (
          <Centered>Il core non ha risposto: la sessione non è partita.</Centered>
        )}

        {state.phase === 'done' && <Summary tally={tally} />}

        {(state.phase === 'asking' || state.phase === 'answered') && (
          <div className="flex h-full flex-col">
            <Meter tally={tally} />

            <div className="flex flex-1 flex-col items-center justify-center gap-6">
              <PromptView prompt={state.question.prompt} />

              {/* Lo spazio dell'esito e' sempre occupato, anche quando non c'e'
                  niente da dire: il segno non deve saltare quando si risponde. */}
              <div className="flex min-h-14 flex-col items-center gap-1 text-center">
                {state.phase === 'answered' && <Feedback verdict={state.verdict} />}
              </div>
            </div>
          </div>
        )}
      </Screen>

      {leaving && (
        <Confirm
          title="Torni alla scelta?"
          confirmLabel="Esci comunque"
          cancelLabel="Continua la sessione"
          onConfirm={() => goTo('home')}
          onCancel={() => setLeaving(false)}
        >
          I {tally.correct} segni indovinati in questo giro vanno persi: ogni sessione
          riparte da zero.
        </Confirm>
      )}
    </>
  )
}

/** La fascia in fondo: cambia con la fase, ma tiene sempre la stessa altezza. */
function Actions({
  state,
  tally,
  busy,
  onAnswer,
  onNext,
  onRestart,
  onHome,
}: {
  state: SessionState
  tally: Tally
  busy: boolean
  onAnswer: (value: string) => void
  onNext: () => void
  onRestart: () => void
  onHome: () => void
}) {
  if (state.phase === 'failed') {
    return <Button onClick={onRestart}>Riprova</Button>
  }

  if (state.phase === 'done') {
    return (
      <div className="flex flex-col gap-2">
        <Button onClick={onRestart}>
          {tally.total > 0 ? `Rifai i ${tally.total} segni` : 'Rifai il giro'}
        </Button>
        <Button variant="quiet" onClick={onHome}>
          Cambia ambito
        </Button>
      </div>
    )
  }

  if (state.phase !== 'asking' && state.phase !== 'answered') return null

  const { format } = state.question
  if (format.mode !== 'choice') {
    // La scrittura con l'IME arriva al passo successivo. Fino ad allora l'esercizio
    // esiste nel core ma non ha una superficie con cui rispondere.
    return (
      <p className="text-muted/60 py-3 text-center text-xs">
        La scrittura con l’IME arriva al prossimo passo.
      </p>
    )
  }

  const answered = state.phase === 'answered' ? state : null

  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-2 gap-2">
        {format.options.map((option) => (
          <Option
            key={option}
            value={option}
            state={answered ? judge(option, answered.answer, answered.verdict) : 'open'}
            disabled={answered !== null || busy}
            onClick={() => onAnswer(option)}
          />
        ))}
      </div>

      {/* Il posto del bottone e' riservato anche mentre si sceglie, cosi' le opzioni
          non si spostano sotto il pollice nel momento in cui si risponde. */}
      <div className="min-h-12">
        {answered && (
          <Button variant="quiet" disabled={busy} onClick={onNext}>
            Avanti
          </Button>
        )}
      </div>
    </div>
  )
}

type OptionState = 'open' | 'right' | 'wrong' | 'other'

/** Come si presenta un'opzione una volta che si e' risposto. */
function judge(option: string, answer: string, verdict: Verdict): OptionState {
  const accepted = verdict.outcome === 'correct' ? [answer] : verdict.accepted
  if (accepted.includes(option)) return 'right'
  if (option === answer) return 'wrong'
  return 'other'
}

const OPTION_STYLES: Record<OptionState, string> = {
  open: 'border-muted/20 bg-ink-soft text-paper',
  right: 'border-ok bg-ok/15 text-ok',
  wrong: 'border-accent bg-accent/15 text-accent',
  other: 'border-muted/10 text-muted/40',
}

const OPTION_MARKS: Record<OptionState, string> = {
  open: '',
  right: '✓',
  wrong: '✕',
  other: '',
}

/**
 * Una delle scelte.
 *
 * L'esito non e' affidato al solo colore: il segno di spunta o la croce dicono la
 * stessa cosa a chi i colori non li distingue.
 */
function Option({
  value,
  state,
  disabled,
  onClick,
}: {
  value: string
  state: OptionState
  disabled: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`flex min-h-16 items-center justify-center gap-2 rounded-xl border text-xl transition-colors active:opacity-70 ${OPTION_STYLES[state]}`}
    >
      {value}
      {OPTION_MARKS[state] && <span aria-hidden="true">{OPTION_MARKS[state]}</span>}
    </button>
  )
}

function Feedback({ verdict }: { verdict: Verdict }) {
  if (verdict.outcome === 'correct') {
    return <p className="text-ok text-base">Giusto</p>
  }

  return (
    <>
      <p className="text-muted text-sm">Si legge</p>
      <p className="text-accent text-xl">{verdict.accepted.join(' · ')}</p>
    </>
  )
}

/** Com'è andato il giro appena finito. Non viene salvato da nessuna parte. */
function Summary({ tally }: { tally: Tally }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
      <p className="text-muted text-xs tracking-[0.2em] uppercase">Giro finito</p>
      <p className="text-5xl tabular-nums">
        <span className={tally.correct === tally.answered ? 'text-ok' : 'text-paper'}>
          {tally.correct}
        </span>
        <span className="text-muted/40">/{tally.answered}</span>
      </p>
      <p className="text-muted mt-2 text-sm">
        {tally.correct === tally.answered
          ? 'Nessun errore.'
          : `${tally.answered - tally.correct} errori, tutti recuperati.`}
      </p>
    </div>
  )
}

/**
 * A che punto è il giro. Vale solo per questa sessione.
 *
 * Avanza sui segni indovinati, non sulle risposte date: sbagliando si risponde di più
 * senza avvicinarsi alla fine, ed è giusto che la barra lo dica.
 */
function Meter({ tally }: { tally: Tally }) {
  const ratio = tally.total > 0 ? tally.correct / tally.total : 0

  return (
    <div className="flex items-center gap-3 pt-1">
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={tally.total}
        aria-valuenow={tally.correct}
        aria-label="Segni indovinati in questo giro"
        className="bg-ink-soft h-1 flex-1 overflow-hidden rounded-full"
      >
        <div
          className="bg-accent h-full transition-[width] duration-300"
          style={{ width: `${ratio * 100}%` }}
        />
      </div>
      <span className="text-muted/60 text-xs tabular-nums">
        {tally.correct}/{tally.total}
      </span>
    </div>
  )
}

function PromptView({ prompt }: { prompt: Prompt }) {
  if (prompt.script === 'japanese') {
    return (
      <p className="font-jp text-8xl leading-none" lang="ja">
        {prompt.text}
      </p>
    )
  }

  return <p className="text-6xl leading-none tracking-wide">{prompt.text}</p>
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-muted flex h-full flex-col items-center justify-center text-center text-sm">
      {children}
    </div>
  )
}
