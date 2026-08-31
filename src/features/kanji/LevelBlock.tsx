import type { LevelProgress } from '@/shared/bridge'

/**
 * A che punto e' un livello: il numero, lo stato e i conteggi.
 *
 * **Uno solo per due schermate.** Lo usano il banner della home, che parla del livello
 * a cui si e' arrivati, e il blocco della vista dei livelli, che parla di quello che si
 * sta guardando. Sono la stessa cosa detta di due livelli diversi, e tenerne due copie
 * le farebbe divergere come sono gia' divergiti `Chip` e `Card`.
 *
 * `recall` c'e' solo dove il dato c'e': la panoramica del livello corrente non lo
 * porta, la riga della dashboard si'. Assente non si scrive, perche' `null` vuol dire
 * «non misurato» e uno zero direbbe un'altra cosa.
 */
export function LevelBlock({
  level,
  progress,
  recall,
  unlocked = true,
}: {
  level: number
  progress?: LevelProgress
  recall?: number | null
  /** Se il livello si puo' studiare. Serve a distinguere «chiuso» da «in corso». */
  unlocked?: boolean
}) {
  return (
    <div className="border-hairline bg-ink-soft flex flex-col gap-3 rounded-2xl border p-4">
      <div className="flex items-baseline justify-between gap-2">
        <p className="text-3xl tabular-nums">Level {level}</p>
        <Tag complete={progress?.complete} unlocked={unlocked} />
      </div>

      {progress && (
        <>
          <div
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={progress.total}
            aria-valuenow={progress.mature}
            aria-label={`Level ${level}, ${progress.mature} of ${progress.total} consolidated`}
            className="bg-ink h-1.5 overflow-hidden rounded-full"
          >
            <div
              className="bg-type-kanji h-full transition-[width] duration-300"
              style={{ width: `${progress.ratio * 100}%` }}
            />
          </div>

          <p className="text-muted text-xs tabular-nums">
            {progress.mature} consolidated · {progress.learning} in progress ·{' '}
            {progress.new} to meet
            {recall !== null && recall !== undefined && ` · recall ${Math.round(recall * 100)}%`}
          </p>
        </>
      )}
    </div>
  )
}

function Tag({ complete, unlocked }: { complete?: boolean; unlocked: boolean }) {
  const [label, tone] = !unlocked
    ? ['locked', 'text-inactive']
    : complete
      ? ['complete', 'text-ok']
      : ['in progress', 'text-type-kanji']

  return (
    <span className={`text-xs font-medium tracking-[0.2em] uppercase ${tone}`}>{label}</span>
  )
}
