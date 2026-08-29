import { useEffect, useState } from 'react'

import {
  kanaCatalogue,
  type KanaGroup,
  type KanaSet,
  type Syllabary,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Chip } from '@/shared/ui/Chip'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

const SYLLABARIES = [
  { value: 'hiragana', label: 'ひらがな', caption: 'Hiragana' },
  { value: 'katakana', label: 'カタカナ', caption: 'Katakana' },
] as const

const MODES = [
  { value: 'recognition', label: 'Riconoscimento', caption: 'Scegli la lettura' },
  { value: 'input', label: 'Scrittura', caption: 'Digita con l’IME' },
] as const

const GROUP_LABELS: Record<KanaGroup, string> = {
  base: 'Base',
  dakuten: 'Sonori',
  handakuten: 'Semisonori',
  yoon: 'Combinazioni',
}

export function HomeScreen() {
  const { scope, setSyllabary, setMode, toggleGroup, goTo } = useUi()
  // Il catalogo si porta dietro il sillabario a cui appartiene. Cosi' cambiando
  // scelta il risultato vecchio smette di valere da solo, senza doverlo azzerare a
  // mano dentro l'effetto e far ripartire un altro render.
  const [loaded, setLoaded] = useState<{
    syllabary: Syllabary
    /** `null` se la richiesta e' fallita. */
    sets: KanaSet[] | null
  } | null>(null)

  useEffect(() => {
    let current = true
    const syllabary = scope.syllabary

    kanaCatalogue(syllabary)
      .then((sets) => current && setLoaded({ syllabary, sets }))
      .catch(() => current && setLoaded({ syllabary, sets: null }))

    // Cambiando sillabario mentre la richiesta e' in volo, la risposta vecchia non
    // deve sovrascrivere quella nuova.
    return () => {
      current = false
    }
  }, [scope.syllabary])

  const fresh = loaded?.syllabary === scope.syllabary ? loaded : undefined
  const sets = fresh?.sets ?? null
  const failed = fresh?.sets === null

  const chosen = sets?.filter((s) => scope.groups.includes(s.group)) ?? []
  const total = chosen.reduce((sum, s) => sum + s.size, 0)

  return (
    <Screen
      title="Tanren"
      action={
        <Button disabled={scope.groups.length === 0} onClick={() => goTo('session')}>
          {total > 0 ? `Inizia con ${total} segni` : 'Inizia'}
        </Button>
      }
    >
      <div className="flex flex-col gap-7">
        <Field label="Sillabario">
          <div className="grid grid-cols-2 gap-2">
            {SYLLABARIES.map((s) => (
              <Card
                key={s.value}
                pressed={scope.syllabary === s.value}
                onClick={() => setSyllabary(s.value)}
              >
                <span className="font-jp text-2xl" lang="ja">
                  {s.label}
                </span>
                <span className="text-muted text-xs">{s.caption}</span>
              </Card>
            ))}
          </div>
        </Field>

        <Field label="Famiglie">
          {failed && <Note>Il catalogo non è raggiungibile.</Note>}
          {!failed && !sets && <Note>Carico…</Note>}
          {sets && (
            <div className="flex flex-wrap gap-2">
              {sets.map((s) => (
                <Chip
                  key={s.group}
                  pressed={scope.groups.includes(s.group)}
                  onClick={() => toggleGroup(s.group)}
                >
                  {GROUP_LABELS[s.group]}
                  <span className="text-muted ml-2 text-xs">{s.size}</span>
                </Chip>
              ))}
            </div>
          )}
        </Field>

        <Field label="Esercizio">
          <div className="grid grid-cols-2 gap-2">
            {MODES.map((m) => (
              <Card
                key={m.value}
                pressed={scope.mode === m.value}
                onClick={() => setMode(m.value)}
              >
                <span className="text-base">{m.label}</span>
                <span className="text-muted text-xs">{m.caption}</span>
              </Card>
            ))}
          </div>
        </Field>
      </div>
    </Screen>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">
        {label}
      </h2>
      {children}
    </section>
  )
}

function Card({
  pressed,
  onClick,
  children,
}: {
  pressed: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={`flex min-h-20 flex-col items-center justify-center gap-1 rounded-xl border transition-colors active:opacity-70 ${
        // Deriva da riconciliare: l'altra scelta premuta del progetto (`Chip`) usa
        // `bg-accent-wash` (15%), questa e' al 10%. Lasciata com'era per la stessa
        // ragione detta in `Chip`.
        pressed ? 'border-accent bg-accent/10' : 'border-hairline bg-ink-soft'
      }`}
    >
      {children}
    </button>
  )
}

function Note({ children }: { children: React.ReactNode }) {
  return <p className="text-muted text-sm">{children}</p>
}
