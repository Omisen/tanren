import { useEffect, useState } from 'react'

import type { Note, Prompt, Question, Verdict } from '@/shared/bridge'
import { AnswerField } from '@/shared/ui/AnswerField'
import { Button } from '@/shared/ui/Button'
import { Confirm } from '@/shared/ui/Confirm'
import { Screen } from '@/shared/ui/Screen'

import type { Session, SessionState, Tally } from './useSession'

/**
 * La schermata di un giro di studio, uguale per tutte le materie.
 *
 * Tutto quello che conta lo decide il core: quale item tocca, se la risposta e' giusta
 * e cosa registrare. Qui si mostra e si raccoglie il tocco.
 *
 * Le opzioni stanno in fondo, nella fascia delle azioni, perche' sono la cosa che si
 * tocca decine di volte di fila: il pollice deve trovarle senza spostare la mano. Lo
 * stimolo resta in mezzo, dove cade lo sguardo.
 *
 * # Cosa cambia da materia a materia, e come arriva
 *
 * Non la struttura, che e' identica: barra, stimolo, esito, azioni. Cambiano il colore
 * della categoria, come si chiama quello che si conta, e le due righe di testo che
 * dipendono da cosa si sta chiedendo. Arrivano tutte come proprieta', cosi' questo file
 * non deve sapere cosa sia un kana o un kanji.
 */

/** Quanto resta visibile l'esito di una risposta giusta, prima di andare avanti. */
const ADVANCE_MS = 700

/** Come si introduce la risposta giusta dopo un errore. */
export interface Reveal {
  /** La riga sopra, per esempio «Read as». */
  label: string
  /** In che alfabeto e' la risposta: decide font e corpo. */
  script: 'japanese' | 'latin'
}

export interface StudyProps {
  /** Il titolo della fascia in alto: di norma la modalita'. */
  title: string
  /**
   * La classe di sfondo del blocco dello stimolo, cioe' il **colore della
   * categoria**: `bg-type-kana`, `bg-type-kanji`. Il colore porta informazione e dice
   * a colpo d'occhio cosa si sta allenando (sezione 4, regole 1 e 2).
   */
  accent: string
  /** Come si chiama al plurale quello che si conta: «characters», «readings». */
  unit: string
  session: Session
  onHome: () => void
  /** La riga sopra lo stimolo, quando la domanda da sola non basta. */
  hint?: (question: Question) => string | null
  /** Come si presenta la risposta giusta dopo un errore. */
  reveal: (question: Question) => Reveal
  /**
   * Come si dice un rilievo su una risposta **giusta**.
   *
   * Il core manda un'etichetta (`on_in_hiragana`), non una frase: la frase la scrive
   * la materia, che e' l'unica a sapere di cosa si sta parlando.
   */
  remark?: (note: Note) => string
  /** Serve solo alle modalita' in cui si scrive. */
  input?: {
    placeholder: string
    normalize: (value: string) => Promise<string>
  }
}

export function SessionScreen({
  title,
  accent,
  unit,
  session,
  onHome,
  hint,
  reveal,
  remark,
  input,
}: StudyProps) {
  const { state, tally, busy, dirty, answer, next, restart } = session
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
        title={title}
        onBack={() => (dirty ? setLeaving(true) : onHome())}
        action={
          <Actions
            state={state}
            tally={tally}
            busy={busy}
            unit={unit}
            reveal={reveal}
            input={input}
            onAnswer={answer}
            onNext={next}
            onRestart={restart}
            onHome={onHome}
          />
        }
      >
        {state.phase === 'loading' && <Centered>Setting up the round…</Centered>}

        {state.phase === 'failed' && (
          <Centered>The core did not answer: the session did not start.</Centered>
        )}

        {state.phase === 'done' && <Summary tally={tally} />}

        {(state.phase === 'asking' || state.phase === 'answered') && (
          /* `min-h-full` e non `h-full`: con la tastiera aperta lo spazio verticale
             si dimezza, e un contenitore alto esattamente quanto `main` non potrebbe
             crescere. Il contenuto traboccherebbe da un `justify-center`, cioe'
             verrebbe tagliato sopra e sotto, senza niente da scorrere per
             recuperarlo. Potendo crescere, `main` scorre davvero. */
          <div className="flex min-h-full flex-col">
            <Meter tally={tally} unit={unit} />

            <div className="flex flex-1 flex-col items-center justify-center gap-6">
              <div className="flex flex-col items-center gap-3">
                {/* Cosa si chiede, quando lo stimolo da solo non lo dice: in quale
                    sillabario scrivere, o quale lettura di un kanji si vuole. */}
                {hint?.(state.question) && (
                  <p className="text-muted text-xs font-medium tracking-[0.2em] uppercase">
                    {hint(state.question)}
                  </p>
                )}
                <Stage prompt={state.question.prompt} accent={accent} />
              </div>

              {/* Lo spazio dell'esito e' sempre occupato, anche quando non c'e'
                  niente da dire: lo stimolo non deve saltare quando si risponde. */}
              <div className="flex min-h-16 flex-col items-center gap-1 text-center">
                {state.phase === 'answered' && (
                  <Feedback
                    verdict={state.verdict}
                    reveal={reveal(state.question)}
                    remark={remark}
                  />
                )}
              </div>
            </div>
          </div>
        )}
      </Screen>

      {leaving && (
        <Confirm
          title="Back to the choice?"
          confirmLabel="Leave anyway"
          cancelLabel="Keep going"
          onConfirm={onHome}
          onCancel={() => setLeaving(false)}
        >
          The {tally.correct} {unit} you got right in this round are lost: every session
          starts from zero.
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
  unit,
  reveal,
  input,
  onAnswer,
  onNext,
  onRestart,
  onHome,
}: {
  state: SessionState
  tally: Tally
  busy: boolean
  unit: string
  reveal: (question: Question) => Reveal
  input?: StudyProps['input']
  onAnswer: (value: string) => void
  onNext: () => void
  onRestart: () => void
  onHome: () => void
}) {
  if (state.phase === 'failed') {
    return <Button onClick={onRestart}>Try again</Button>
  }

  if (state.phase === 'done') {
    return (
      <div className="flex flex-col gap-2">
        <Button onClick={onRestart}>
          {tally.total > 0 ? `Redo the ${tally.total} ${unit}` : 'Redo the round'}
        </Button>
        <Button variant="quiet" onClick={onHome}>
          Change scope
        </Button>
      </div>
    )
  }

  if (state.phase !== 'asking' && state.phase !== 'answered') return null

  const { format } = state.question
  const answered = state.phase === 'answered' ? state : null

  if (format.mode === 'input') {
    // Una domanda a risposta libera senza il modo di normalizzarla non e' uno stato in
    // cui l'utente puo' finire: e' una schermata montata male.
    if (!input) return <Button onClick={onHome}>Change scope</Button>

    return (
      <div className="flex flex-col gap-2">
        <AnswerField
          // Il campo riparte pulito a ogni domanda. Serve anche il conteggio delle
          // risposte, non solo l'item: l'ultimo item rimasto in coda puo' essere
          // chiesto due volte di fila, e con la sola chiave dell'item il campo si
          // ritroverebbe dentro il tentativo precedente.
          key={`${state.question.item}:${tally.answered}`}
          disabled={answered !== null || busy}
          given={answered?.answer ?? null}
          placeholder={input.placeholder}
          normalize={input.normalize}
          onSubmit={onAnswer}
        />

        <div className="min-h-12">
          {answered && (
            <Button variant="quiet" disabled={busy} onClick={onNext}>
              Next
            </Button>
          )}
        </div>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="grid grid-cols-2 gap-2">
        {format.options.map((option) => (
          <Option
            key={option}
            value={option}
            script={reveal(state.question).script}
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
            Next
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
  open: 'border-hairline bg-ink-soft text-paper',
  right: 'border-ok bg-ok-wash text-ok',
  wrong: 'border-accent bg-accent-wash text-accent',
  other: 'border-hairline-soft text-inactive',
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
 *
 * Il corpo e' piu' grande quando l'opzione e' giapponese, perche' una lettura in kana
 * a `text-xl` si legge peggio della trascrizione latina della stessa misura.
 */
function Option({
  value,
  script,
  state,
  disabled,
  onClick,
}: {
  value: string
  /** In che alfabeto e' l'opzione: sui kana e' una trascrizione, sui kanji una lettura. */
  script: Reveal['script']
  state: OptionState
  disabled: boolean
  onClick: () => void
}) {
  const japanese = script === 'japanese'

  return (
    <button
      type="button"
      disabled={disabled}
      // Il giapponese vuole il font imbarcato e l'attributo `lang`: col ripiego di
      // sistema si vedrebbero forme diverse fra telefono e desktop, e su un Linux
      // senza font CJK dei rettangoli vuoti. E' la stessa ragione per cui il font
      // e' imbarcato invece che chiesto alla rete.
      lang={japanese ? 'ja' : undefined}
      onClick={onClick}
      className={`flex min-h-16 items-center justify-center gap-2 rounded-xl border transition-colors active:opacity-70 ${
        japanese ? 'font-jp text-2xl' : 'text-xl'
      } ${OPTION_STYLES[state]}`}
    >
      {value}
      {OPTION_MARKS[state] && <span aria-hidden="true">{OPTION_MARKS[state]}</span>}
    </button>
  )
}

function Feedback({
  verdict,
  reveal,
  remark,
}: {
  verdict: Verdict
  reveal: Reveal
  remark?: (note: Note) => string
}) {
  if (verdict.outcome === 'correct') {
    // Giusta, ma scritta contro la convenzione: si dice, e non si toglie niente. Chi
    // ha ricordato la lettura ha ricordato, e il rilievo insegna l'ortografia senza
    // trasformarla in un errore.
    const note = verdict.note && remark?.(verdict.note)
    return (
      <>
        <p className="text-ok text-base">Right</p>
        {note && <p className="text-muted text-sm">{note}</p>}
      </>
    )
  }

  const japanese = reveal.script === 'japanese'

  return (
    <>
      <p className="text-muted text-sm">{reveal.label}</p>
      <p
        className={`text-accent ${japanese ? 'font-jp text-4xl leading-none' : 'text-xl'}`}
        lang={japanese ? 'ja' : undefined}
      >
        {verdict.accepted.join(' · ')}
      </p>
    </>
  )
}

/** Com'è andato il giro appena finito. Non viene salvato da nessuna parte. */
function Summary({ tally }: { tally: Tally }) {
  const mistakes = tally.answered - tally.correct

  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
      <p className="text-muted text-xs font-medium tracking-[0.2em] uppercase">Round done</p>
      <p className="text-5xl tabular-nums">
        <span className={tally.correct === tally.answered ? 'text-ok' : 'text-paper'}>
          {tally.correct}
        </span>
        <span className="text-muted">/{tally.answered}</span>
      </p>
      <p className="text-muted mt-2 text-sm">
        {mistakes === 0
          ? 'No mistakes.'
          : mistakes === 1
            ? 'One mistake, recovered.'
            : `${mistakes} mistakes, all recovered.`}
      </p>
    </div>
  )
}

/**
 * A che punto è il giro. Vale solo per questa sessione.
 *
 * Avanza sugli item indovinati, non sulle risposte date: sbagliando si risponde di
 * più senza avvicinarsi alla fine, ed è giusto che la barra lo dica.
 */
function Meter({ tally, unit }: { tally: Tally; unit: string }) {
  const ratio = tally.total > 0 ? tally.correct / tally.total : 0

  return (
    <div className="flex items-center gap-3 pt-1">
      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={tally.total}
        aria-valuenow={tally.correct}
        aria-label={`${unit} guessed in this round`}
        className="bg-ink-soft h-1 flex-1 overflow-hidden rounded-full"
      >
        <div
          className="bg-type-kana h-full transition-[width] duration-300"
          style={{ width: `${ratio * 100}%` }}
        />
      </div>
      <span className="text-muted text-xs tabular-nums">
        {tally.correct}/{tally.total}
      </span>
    </div>
  )
}

/**
 * Il palco dello stimolo: un blocco a tinta piena del colore della categoria, col
 * segno grande e bianco in mezzo.
 *
 * # Perche' un blocco
 *
 * Il carattere e' l'elemento primario di ogni schermata di studio, e il colore del
 * blocco dice a quale categoria appartiene quello che si sta allenando (sezione 4,
 * regole 1 e 2). Il colore porta informazione: non e' decorazione.
 *
 * Il blocco c'e' **in entrambe le modalita'**, anche scrivendo, dove lo stimolo puo'
 * non essere un carattere giapponese. Il colore dice quale materia si sta allenando e
 * questo vale comunque; farlo comparire e sparire a seconda della modalita' farebbe
 * sembrare due app diverse invece di due modi di esercitarsi sulla stessa materia.
 *
 * # Perche' le misure sono in `vh`
 *
 * Quando si apre la tastiera lo spazio verticale si dimezza, ed e' proprio il momento
 * in cui lo stimolo deve restare visibile (regola 5, che vince su qualsiasi scelta
 * estetica). Un blocco di dimensione fissa lo spingerebbe fuori dallo schermo.
 * Legando blocco e segno alla stessa unita' si rimpiccioliscono insieme e restano in
 * proporzione: con la tastiera aperta il blocco scende a circa 128 px e il segno a
 * 64, sopra i 209 e 105 di quando la tastiera non c'e'. I limiti di `clamp` evitano i
 * due eccessi, illeggibile sugli schermi bassi e smisurato sul desktop.
 *
 * Perche' `vh` segua davvero lo spazio utile serve che la WebView si restringa con la
 * tastiera: lo fa `MainActivity.kt`, vedi la nota sugli insets.
 */
function Stage({ prompt, accent }: { prompt: Prompt; accent: string }) {
  const japanese = prompt.script === 'japanese'

  return (
    <div
      className={`${accent} flex aspect-square h-[clamp(5.5rem,24vh,17rem)] items-center justify-center rounded-3xl px-3`}
    >
      {japanese ? (
        <p
          className="font-jp text-[clamp(2.75rem,12vh,8.5rem)] leading-none text-paper"
          lang="ja"
        >
          {prompt.text}
        </p>
      ) : (
        <p className="text-[clamp(1.75rem,10.5vh,6rem)] leading-none tracking-wide text-paper">
          {prompt.text}
        </p>
      )}
    </div>
  )
}

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-muted flex h-full flex-col items-center justify-center text-center text-sm">
      {children}
    </div>
  )
}
