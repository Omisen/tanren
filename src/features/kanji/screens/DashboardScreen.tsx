import { useEffect, useState } from 'react'

import { kanjiDashboard, type LevelSummary } from '@/shared/bridge'
import { Note } from '@/shared/ui/Card'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

/**
 * Come sta andando tutto il percorso.
 *
 * # Cosa misura, e cosa no
 *
 * Misura **quanto sei consolidato**, che e' quello che dice FSRS, e lo alimentano solo
 * il Learning e il Ripasso. Il Drill non compare qui e non deve: e' pratica in piu', e
 * come e' andata oggi si vede alla fine del giro e finisce li'. Sono due misure
 * diverse, e mescolarle farebbe sembrare progresso quello che e' solo esercizio.
 */
export function KanjiDashboardScreen() {
  const { goTo, setLevel } = useUi()
  const [levels, setLevels] = useState<LevelSummary[] | null | 'failed'>(null)

  useEffect(() => {
    let current = true
    kanjiDashboard()
      .then((rows) => current && setLevels(rows))
      .catch(() => current && setLevels('failed'))
    return () => {
      current = false
    }
  }, [])

  const rows = Array.isArray(levels) ? levels : []
  const mature = rows.reduce((n, l) => n + l.mature, 0)
  const total = rows.reduce((n, l) => n + l.total, 0)
  // Non si mostrano i livelli che non hai ancora sfiorato: sessanta righe tutte a zero
  // non dicono niente e nascondono le poche che parlano.
  const touched = rows.filter((l) => l.unlocked || l.mature > 0 || l.learning > 0)

  return (
    <Screen title="Progress" onBack={() => goTo('home')}>
      {levels === null && <Note>Loading…</Note>}
      {levels === 'failed' && <Note>Progress could not be loaded.</Note>}

      {rows.length > 0 && (
        <div className="flex flex-col gap-6">
          <div className="flex flex-col gap-1">
            <p className="text-4xl tabular-nums">
              {mature}
              <span className="text-muted text-2xl">/{total}</span>
            </p>
            <p className="text-muted text-sm">
              kanji consolidated, meaning every facet of them holds for three weeks or
              more.
            </p>
          </div>

          <div className="flex flex-col gap-3">
            {touched.map((l) => (
              <Row key={l.level} level={l} onOpen={() => {
                setLevel(l.level)
                goTo('home')
              }} />
            ))}
            {touched.length < rows.length && (
              <p className="text-muted pt-2 text-center text-xs">
                {rows.length - touched.length} more levels ahead, still closed.
              </p>
            )}
          </div>

          <p className="text-muted text-xs">
            This is what spaced repetition knows, and only Learn and Review feed it.
            Drill never moves these numbers: how a practice round went is shown at the
            end of that round and stays there.
          </p>
        </div>
      )}
    </Screen>
  )
}

function Row({ level, onOpen }: { level: LevelSummary; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="border-hairline bg-ink-soft flex flex-col gap-2 rounded-xl border p-3 text-left active:opacity-70"
    >
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-base tabular-nums">Level {level.level}</span>
        <State level={level} />
      </div>

      <div
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={level.total}
        aria-valuenow={level.mature}
        aria-label={`Level ${level.level}, ${level.mature} of ${level.total} consolidated`}
        className="bg-ink h-1.5 overflow-hidden rounded-full"
      >
        <div
          className="bg-type-kanji h-full transition-[width] duration-300"
          style={{ width: `${level.ratio * 100}%` }}
        />
      </div>

      <p className="text-muted text-xs tabular-nums">
        {level.mature} consolidated · {level.learning} in progress · {level.new} to meet
        {level.recall !== null && ` · recall ${Math.round(level.recall * 100)}%`}
      </p>
    </button>
  )
}

function State({ level }: { level: LevelSummary }) {
  const [label, tone] = level.complete
    ? ['complete', 'text-ok']
    : !level.unlocked
      ? ['locked', 'text-inactive']
      : ['in progress', 'text-type-kanji']

  return (
    <span className={`text-xs font-medium tracking-[0.2em] uppercase ${tone}`}>{label}</span>
  )
}
