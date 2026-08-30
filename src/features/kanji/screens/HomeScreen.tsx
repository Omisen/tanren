import { useCallback, useEffect, useState, type ReactNode } from 'react'

import {
  kanjiCurrentLevel,
  kanjiGrid,
  kanjiOverview,
  type Gate,
  type KanjiCell,
  type Level,
  type Overview,
  type Standing,
  type StudyMode,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Field } from '@/shared/ui/Field'
import { Note } from '@/shared/ui/Card'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

import { KanjiSheet } from '../KanjiSheet'

/**
 * Il percorso sui kanji: dove sei, cosa c'e' in questo livello, cosa puoi fare.
 *
 * # Perche' non e' piu' un filtro
 *
 * La versione precedente era una query su un dataset: scegli un anno di scuola,
 * scegli un tipo di lettura, parti con «80 readings». Metteva al centro le letture
 * invece del kanji, non mostrava mai un significato, e usava l'anno scolastico
 * giapponese come asse, che non e' un ordine di difficolta' per chi non e'
 * madrelingua. Qui il centro e' il kanji, l'asse e' un percorso, e l'avanzamento e' una
 * cosa che si vede.
 *
 * # I livelli si guardano tutti, si studiano solo quelli aperti
 *
 * Un livello piu' avanti si puo' aprire e sfogliare: sapere cosa arrivera' non e'
 * barare. Ma i tre bottoni restano spenti, perche' il percorso e' sequenziale.
 */

const MODES: { value: StudyMode; label: string; caption: string }[] = [
  { value: 'learning', label: 'Learn', caption: 'Meet new kanji' },
  { value: 'review', label: 'Review', caption: 'What is due' },
  { value: 'drill', label: 'Drill', caption: 'Free practice' },
]

export function KanjiHomeScreen({ subjects }: { subjects: ReactNode }) {
  const { kanji: scope, setLevel, study } = useUi()
  const [reached, setReached] = useState<Level | null>(null)
  const [overview, setOverview] = useState<{ level: Level; data: Overview | null } | null>(null)
  const [grid, setGrid] = useState<{ level: Level; cells: KanjiCell[] } | null>(null)
  const [opened, setOpened] = useState<KanjiCell | null>(null)

  // Fin dove si e' arrivati lo sa il core: e' il primo livello non ancora consolidato.
  // Si chiede una volta sola, e la schermata ci si porta sopra.
  useEffect(() => {
    let current = true
    kanjiCurrentLevel()
      .then((level) => {
        if (!current) return
        setReached(level)
        setLevel(level)
      })
      .catch(() => current && setReached(1))
    return () => {
      current = false
    }
  }, [setLevel])

  // Il livello guardato porta con se' i propri dati, come il catalogo dei kana col
  // sillabario: cambiando livello il risultato vecchio smette di valere da solo.
  useEffect(() => {
    let current = true
    const level = scope.level

    kanjiOverview({ ...scope, level })
      .then((data) => current && setOverview({ level, data }))
      .catch(() => current && setOverview({ level, data: null }))
    kanjiGrid(level)
      .then((cells) => current && setGrid({ level, cells }))
      .catch(() => current && setGrid({ level, cells: [] }))

    return () => {
      current = false
    }
    // La modalita' non cambia cosa mostrare: cambia solo cosa succede toccando un
    // bottone, e non vale una richiesta in piu'.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, [scope.level])

  const fresh = overview?.level === scope.level ? overview.data : undefined
  const cells = grid?.level === scope.level ? grid.cells : null
  const locked = reached !== null && scope.level > reached

  const start = useCallback((mode: StudyMode) => study(mode), [study])

  return (
    <>
      <Screen
        textured
        title="Tanren"
        action={
          <div className="flex flex-col gap-2">
            {MODES.map((m) => (
              <ModeButton
                key={m.value}
                mode={m}
                locked={locked}
                overview={fresh}
                onStart={start}
              />
            ))}
          </div>
        }
      >
        <div className="flex flex-col gap-7">
          {subjects}

          <Field label="Level">
            <LevelPile
              level={scope.level}
              reached={reached}
              progress={fresh?.progress}
              onPick={setLevel}
            />
          </Field>

          <Field label={locked ? 'Locked, but you can look' : 'Kanji'}>
            {cells === null && <Note>Loading…</Note>}
            {cells !== null && <Grid cells={cells} onOpen={setOpened} />}
          </Field>
        </div>
      </Screen>

      {/* La scheda si apre anche sui livelli chiusi: guardare cosa arrivera' non e'
          barare, ed e' l'unico modo di farsi un'idea del percorso. */}
      {opened && (
        <KanjiSheet
          level={scope.level}
          character={opened.character}
          standing={opened.standing}
          onClose={() => setOpened(null)}
        />
      )}
    </>
  )
}

/**
 * Dove sei nel percorso.
 *
 * Il livello guardato sta in grande, quelli attorno rimpiccioliti: a colpo d'occhio si
 * deve leggere «sei qui, questo viene dopo». La barra dice quanto e' consolidato, che
 * e' la cosa che apre il livello successivo.
 */
function LevelPile({
  level,
  reached,
  progress,
  onPick,
}: {
  level: Level
  reached: Level | null
  progress?: Overview['progress']
  onPick: (level: Level) => void
}) {
  const around = [level - 2, level - 1, level, level + 1, level + 2].filter(
    (l) => l >= 1 && l <= 69,
  )

  return (
    <div className="flex flex-col gap-3">
      <div className="border-hairline bg-ink-soft flex flex-col gap-3 rounded-2xl border p-4">
        <div className="flex items-baseline justify-between">
          <p className="text-3xl tabular-nums">Level {level}</p>
          <State level={level} reached={reached} progress={progress} />
        </div>

        {progress && (
          <>
            <div
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={progress.total}
              aria-valuenow={progress.mature}
              aria-label="Kanji consolidated in this level"
              className="bg-ink h-1.5 overflow-hidden rounded-full"
            >
              <div
                className="bg-type-kanji h-full transition-[width] duration-300"
                style={{ width: `${progress.ratio * 100}%` }}
              />
            </div>
            <p className="text-muted text-xs">
              {progress.mature} consolidated · {progress.learning} in progress ·{' '}
              {progress.new} to meet
            </p>
          </>
        )}
      </div>

      <div className="flex items-end justify-center gap-2">
        {around.map((l) => (
          <button
            key={l}
            type="button"
            aria-pressed={l === level}
            aria-label={`Level ${l}`}
            onClick={() => onPick(l)}
            className={`rounded-lg border tabular-nums transition-colors active:opacity-70 ${
              l === level
                ? 'border-selected bg-selected-wash text-paper px-4 py-2 text-base'
                : 'border-hairline text-muted px-3 py-1 text-sm'
            }`}
          >
            {l}
          </button>
        ))}
      </div>
    </div>
  )
}

function State({
  level,
  reached,
  progress,
}: {
  level: Level
  reached: Level | null
  progress?: Overview['progress']
}) {
  if (reached === null) return null
  if (level > reached) return <Tag>locked</Tag>
  if (progress?.complete) return <Tag tone="done">complete</Tag>
  return <Tag tone="current">in progress</Tag>
}

function Tag({ children, tone }: { children: ReactNode; tone?: 'done' | 'current' }) {
  const styles =
    tone === 'done'
      ? 'text-ok'
      : tone === 'current'
        ? 'text-type-kanji'
        : 'text-inactive'
  return (
    <span className={`text-xs font-medium tracking-[0.2em] uppercase ${styles}`}>{children}</span>
  )
}

/**
 * I kanji del livello, nell'ordine per frequenza.
 *
 * Il colore dice a che punto sei su ciascuno, e non e' un colore nuovo: e' quello
 * della categoria a due intensita'. Piu' lo sai, piu' e' pieno. Fare tre token nuovi
 * per gli stati SRS e' una decisione da prendere apposta, non di straforo.
 */
const STANDING: Record<Standing, string> = {
  new: 'border-hairline text-muted',
  learning: 'border-type-kanji/40 bg-type-kanji/15 text-paper',
  mature: 'border-type-kanji bg-type-kanji text-paper',
}

function Grid({
  cells,
  onOpen,
}: {
  cells: KanjiCell[]
  onOpen: (cell: KanjiCell) => void
}) {
  if (cells.length === 0) return <Note>Nothing here.</Note>

  return (
    <div className="grid grid-cols-6 gap-1.5">
      {cells.map((c) => (
        <button
          key={c.character}
          type="button"
          onClick={() => onOpen(c)}
          aria-label={`${c.character}, ${c.standing}`}
          className={`font-jp flex aspect-square items-center justify-center rounded-lg border text-xl transition-colors active:opacity-70 ${STANDING[c.standing]}`}
          lang="ja"
        >
          {c.character}
        </button>
      ))}
    </div>
  )
}

/** Un bottone che, quando non si puo' premere, dice perche'. */
function ModeButton({
  mode,
  locked,
  overview,
  onStart,
}: {
  mode: { value: StudyMode; label: string; caption: string }
  locked: boolean
  overview?: Overview | null
  onStart: (mode: StudyMode) => void
}) {
  const reason = why(mode.value, locked, overview)

  return (
    <div className="flex flex-col gap-1">
      <Button
        variant={mode.value === 'learning' ? 'primary' : 'quiet'}
        disabled={reason !== null}
        onClick={() => onStart(mode.value)}
      >
        {mode.label}
        {count(mode.value, overview)}
      </Button>
      {reason && <p className="text-muted px-2 text-center text-xs">{reason}</p>}
    </div>
  )
}

function count(mode: StudyMode, overview?: Overview | null): string {
  if (!overview) return ''
  if (mode === 'review') return overview.available.due > 0 ? ` · ${overview.available.due}` : ''
  if (mode === 'drill') return overview.available.practiced > 0 ? ' · ready' : ''
  return overview.available.learning.state === 'open'
    ? ` · ${overview.available.learning.room}`
    : ''
}

/**
 * Perche' un bottone e' spento, detto con parole utili.
 *
 * «Torna domani» non insegna niente. «Consolida quello che hai» e «torna fra tre ore»
 * sono due consigli diversi, e chi studia ha diritto di sapere quale dei due vale.
 */
function why(mode: StudyMode, locked: boolean, overview?: Overview | null): string | null {
  if (locked) return 'Finish the earlier levels first.'
  if (!overview) return 'Loading…'

  if (mode === 'review') {
    return overview.available.due > 0 ? null : 'Nothing is due right now.'
  }
  if (mode === 'drill') {
    return overview.available.practiced > 0 ? null : 'Nothing learned to practise yet.'
  }
  return blocked(overview.available.learning)
}

function blocked(gate: Gate): string | null {
  if (gate.state === 'open') return null
  switch (gate.reason) {
    case 'consolidate':
      return `Consolidate what you have first: recall is at ${Math.round(
        gate.current * 100,
      )}%, and ${Math.round(gate.needed * 100)}% is the mark.`
    case 'too_soon':
      return `You just met new kanji. Come back ${when(gate.until)}.`
    case 'daily_cap':
      return `That is ${gate.done} new kanji today, which is the daily limit.`
    case 'nothing_new':
      return 'You have met every kanji in this level.'
  }
}

/** Fra quanto, detto in ore o minuti invece che con una data. */
function when(iso: string): string {
  const minutes = Math.max(0, Math.round((new Date(iso).getTime() - Date.now()) / 60_000))
  if (minutes < 60) return `in ${minutes} min`
  return `in about ${Math.round(minutes / 60)} h`
}
