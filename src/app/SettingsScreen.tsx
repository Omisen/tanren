import { useEffect, useState, type ReactNode } from 'react'

import {
  setFlashcardAgain,
  setFlashcardGood,
  setKanjiDailyNew,
  settings as loadSettings,
  type Settings,
} from '@/shared/bridge'
import { ExternalLink } from '@/shared/ui/ExternalLink'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

/** Dove vive il progetto, per chi vuole leggerlo o segnalare qualcosa. */
const REPO = 'https://github.com/Omisen/tanren'

/**
 * Le impostazioni, e le vie secondarie dell'app.
 *
 * Sta nella radice e non in una feature perche' qui dentro convivono le preferenze di
 * due materie diverse, le fonti (che sono di tutta l'app, perche' la licenza obbliga
 * l'app e non una materia) e il rimando alla repository. Nessuna feature potrebbe
 * tenerle insieme senza nominarne un'altra.
 *
 * # Perche' e' una schermata e non piu' una tendina
 *
 * Perche' e' diventata una **destinazione**. La tendina entrava da destra, e quel
 * movimento diceva una cosa vera finche' le impostazioni erano un altro posto in cui si
 * andava **da dentro** una schermata, richiamate da un'icona nell'intestazione: un
 * pannello che si apre sopra quello che stavi facendo, e da cui si torna indietro.
 *
 * Adesso non si torna indietro da nessuna parte: si passa a un'altra sezione, come si
 * passa dai kana ai kanji. Un pannello che copre la sezione di prima, mentre la barra
 * in fondo dice che sei in questa, racconterebbe due cose diverse nello stesso momento.
 */
export function SettingsScreen({ sections }: { sections: ReactNode }) {
  const goTo = useUi((s) => s.goTo)
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
  function move(
    field: 'dailyNew' | 'flashcardAgain' | 'flashcardGood',
    delta: number,
    min: number,
    max: number,
    write: (value: number) => Promise<void>,
  ) {
    if (!current) return
    const value = current[field] + delta
    if (value < min || value > max) return

    const prima = current
    setCurrent({ ...current, [field]: value })
    write(value).catch(() => {
      setCurrent(prima)
      setFailed(true)
    })
  }

  return (
    <Screen title="Settings">
      <div className="flex flex-col gap-7 pt-2">
        {sections}

        <section className="flex flex-col gap-2">
          <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">Kanji</h3>

          <Preference
            title="New kanji per lesson"
            caption="Each one brings its meaning and its readings together."
            value={current?.dailyNew}
            min={current?.dailyNewMin}
            max={current?.dailyNewMax}
            onChange={(delta) =>
              move(
                'dailyNew',
                delta,
                current?.dailyNewMin ?? 0,
                current?.dailyNewMax ?? 0,
                setKanjiDailyNew,
              )
            }
          />
        </section>

        <section className="flex flex-col gap-2">
          <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">
            Flashcards
          </h3>

          {/* I due passi brevi con cui una carta entra in circolo. Non li produce
              l'algoritmo, e non e' un difetto suo: una carta nuova sbagliata gli
              tornerebbe dopo cinque ore, che e' giusto per consolidare e sbagliato per
              imparare adesso. Dopo questi due passi decide lui. */}
          <Preference
            title="Retry after a slip"
            caption="How soon a card you missed comes back."
            value={current?.flashcardAgain}
            min={current?.flashcardAgainMin}
            max={current?.flashcardAgainMax}
            suffix="min"
            onChange={(delta) =>
              move(
                'flashcardAgain',
                delta,
                current?.flashcardAgainMin ?? 0,
                current?.flashcardAgainMax ?? 0,
                setFlashcardAgain,
              )
            }
          />

          <Preference
            title="First repeat"
            caption="How soon a new card you got right comes back."
            value={current?.flashcardGood}
            min={current?.flashcardGoodMin}
            max={current?.flashcardGoodMax}
            suffix="min"
            onChange={(delta) =>
              move(
                'flashcardGood',
                delta,
                current?.flashcardGoodMin ?? 0,
                current?.flashcardGoodMax ?? 0,
                setFlashcardGood,
              )
            }
          />

          {failed && <p className="text-accent text-xs">The setting could not be saved.</p>}
        </section>

        <section className="flex flex-col gap-2">
          <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">About</h3>

          <Row onClick={() => goTo('about')}>Sources and licences</Row>
          <Row href={REPO}>Source code on GitHub</Row>
        </section>
      </div>
    </Screen>
  )
}

/**
 * Una preferenza: cosa decide, e il numero che la muove.
 *
 * E' una sola per tutte e tre perche' fanno la stessa cosa, e tre copie divergerebbero
 * come sono gia' divergiti `Chip` e `Card`. Il numero compare solo quando e' arrivato:
 * finche' non c'e' restano il titolo e la spiegazione, che non si muovono.
 */
function Preference({
  title,
  caption,
  value,
  min,
  max,
  suffix,
  onChange,
}: {
  title: string
  caption: string
  value?: number
  min?: number
  max?: number
  /** L'unita', dove il numero da solo non direbbe di cosa parla. */
  suffix?: string
  onChange: (delta: number) => void
}) {
  return (
    <div className="border-hairline bg-ink flex items-center justify-between gap-3 rounded-2xl border p-3">
      <div className="flex flex-col">
        <span className="text-paper">{title}</span>
        <span className="text-muted text-xs">{caption}</span>
      </div>

      {value !== undefined && (
        <div className="flex shrink-0 items-center gap-1">
          <Step
            label="One fewer"
            glyph="−"
            disabled={min === undefined || value <= min}
            onClick={() => onChange(-1)}
          />
          <span className="min-w-14 text-center text-lg tabular-nums">
            {value}
            {suffix && <span className="text-muted ml-1 text-xs">{suffix}</span>}
          </span>
          <Step
            label="One more"
            glyph="+"
            disabled={max === undefined || value >= max}
            onClick={() => onChange(1)}
          />
        </div>
      )}
    </div>
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
      <ExternalLink href={href} className={stile}>
        {children}
        <span className="text-muted" aria-hidden="true">
          ↗
        </span>
      </ExternalLink>
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
