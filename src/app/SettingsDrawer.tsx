import { useEffect, useState } from 'react'

import { setKanjiDailyNew, settings as loadSettings, type Settings } from '@/shared/bridge'
import { Drawer } from '@/shared/ui/Drawer'

/** Dove vive il progetto, per chi vuole leggerlo o segnalare qualcosa. */
const REPO = 'https://github.com/Omisen/tanren'

/**
 * Le impostazioni, e le vie secondarie dell'app.
 *
 * Sta nella radice e non in una feature per la stessa ragione della scelta della
 * materia: qui dentro convivono una preferenza dei kanji, le fonti (che sono di tutta
 * l'app, perche' la licenza obbliga l'app e non una materia) e il rimando alla
 * repository. Nessuna feature potrebbe tenerle insieme senza nominarne un'altra.
 */
export function SettingsDrawer({ onClose, onSources }: { onClose: () => void; onSources: () => void }) {
  const [current, setCurrent] = useState<Settings | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let alive = true
    loadSettings()
      .then((s) => alive && setCurrent(s))
      .catch(() => alive && setFailed(true))
    return () => {
      alive = false
    }
  }, [])

  /**
   * Si sposta subito quello che si vede e si scrive dopo.
   *
   * Il giro fino al database e ritorno e' breve ma non istantaneo, e un numero che si
   * muove mezzo secondo dopo il tocco si legge come un tocco non registrato, quindi si
   * tocca di nuovo. Se la scrittura fallisce si torna indietro: meglio un numero che
   * rimbalza di uno che dice il falso.
   */
  function change(delta: number) {
    if (!current) return
    const value = current.dailyNew + delta
    if (value < current.dailyNewMin || value > current.dailyNewMax) return

    const prima = current
    setCurrent({ ...current, dailyNew: value })
    setKanjiDailyNew(value).catch(() => {
      setCurrent(prima)
      setFailed(true)
    })
  }

  return (
    <Drawer title="Settings" onClose={onClose}>
      <div className="flex flex-col gap-7">
        <section className="flex flex-col gap-2">
          <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">Learning</h3>

          <div className="border-hairline bg-ink flex items-center justify-between gap-3 rounded-2xl border p-3">
            <div className="flex flex-col">
              <span className="text-paper">New kanji per lesson</span>
              <span className="text-muted text-xs">
                Each one brings its meaning and its readings together.
              </span>
            </div>

            {current && (
              <div className="flex shrink-0 items-center gap-1">
                <Step
                  label="One fewer"
                  glyph="−"
                  disabled={current.dailyNew <= current.dailyNewMin}
                  onClick={() => change(-1)}
                />
                <span className="w-6 text-center text-lg tabular-nums">{current.dailyNew}</span>
                <Step
                  label="One more"
                  glyph="+"
                  disabled={current.dailyNew >= current.dailyNewMax}
                  onClick={() => change(1)}
                />
              </div>
            )}
          </div>

          {failed && (
            <p className="text-accent text-xs">The setting could not be saved.</p>
          )}
        </section>

        <section className="flex flex-col gap-2">
          <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">About</h3>

          <Row onClick={onSources}>Sources and licences</Row>
          <Row href={REPO}>Source code on GitHub</Row>
        </section>
      </div>
    </Drawer>
  )
}

/** Un tondo per muovere di uno, con il bersaglio tattile intero. */
function Step({
  label,
  glyph,
  disabled,
  onClick,
}: {
  label: string
  glyph: string
  disabled: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className="border-hairline text-paper flex size-11 items-center justify-center rounded-full border text-lg active:opacity-60 disabled:text-inactive"
    >
      {glyph}
    </button>
  )
}

/**
 * Una voce dell'elenco, che porta dentro l'app o fuori.
 *
 * Fuori si va con un'ancora normale, che e' quello che gia' fa la schermata delle
 * fonti coi suoi link alle licenze: l'app non ha un plugin per aprire l'esterno, e
 * aggiungerne uno per una riga sarebbe una dipendenza in piu' presa senza averne
 * misurato il bisogno.
 */
function Row({
  href,
  onClick,
  children,
}: {
  href?: string
  onClick?: () => void
  children: string
}) {
  const stile =
    'border-hairline bg-ink flex min-h-14 items-center justify-between gap-3 rounded-2xl border px-3 text-left text-paper active:opacity-60'

  if (href) {
    return (
      <a href={href} target="_blank" rel="noreferrer" className={stile}>
        {children}
        <span className="text-muted" aria-hidden="true">
          ↗
        </span>
      </a>
    )
  }

  return (
    <button type="button" onClick={onClick} className={stile}>
      {children}
      <span className="text-muted" aria-hidden="true">
        ›
      </span>
    </button>
  )
}
