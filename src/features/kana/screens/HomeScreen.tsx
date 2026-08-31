import { useEffect, useState, type ReactNode } from 'react'

import {
  kanaCatalogue,
  type KanaGroup,
  type KanaSet,
  type Syllabary,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Card, Note } from '@/shared/ui/Card'
import { Chip } from '@/shared/ui/Chip'
import { Field } from '@/shared/ui/Field'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

const SYLLABARIES = [
  { value: 'hiragana', label: 'ひらがな', caption: 'Hiragana' },
  { value: 'katakana', label: 'カタカナ', caption: 'Katakana' },
] as const

const MODES = [
  { value: 'recognition', label: 'Recognition', caption: 'Pick the reading' },
  { value: 'input', label: 'Writing', caption: 'Type with the IME' },
] as const

/**
 * Come si chiamano le famiglie, coi termini giapponesi e senza glossa.
 *
 * Sono i nomi che usa chi studia, e tradurli («Voiced», «Combinations») dava una parola
 * che non si incontra da nessun'altra parte: qui l'immersione costa niente, perche' i
 * segni della famiglia sono li' accanto a spiegarla meglio di un'etichetta.
 *
 * `japanese` dice se il nome e' scritto in giapponese, e serve a due cose che vanno
 * insieme: il font imbarcato e l'attributo di lingua. Senza, 外来音 finirebbe nel
 * ripiego di sistema, cioe' rettangoli vuoti su un Linux senza font CJK, che e'
 * esattamente il caso per cui il font e' imbarcato.
 */
const GROUP_LABELS: Record<KanaGroup, { text: string; japanese?: boolean }> = {
  base: { text: 'Base' },
  dakuten: { text: 'Dakuten' },
  handakuten: { text: 'Handakuten' },
  yoon: { text: 'Yōon' },
  gairaion: { text: '外来音', japanese: true },
}

/**
 * La scelta dell'ambito sui kana.
 *
 * La scelta della materia arriva dall'alto come nodo gia' fatto: comporla e' compito
 * della radice, che e' l'unica a conoscerle tutte. Questa schermata sa solo di kana, e
 * con la regola di non incrocio fra feature non potrebbe nemmeno nominare i kanji.
 */
export function KanaHomeScreen({
  subjects,
  about,
}: {
  subjects: ReactNode
  /** La via per le fonti, messa dalla radice: e' cosa dell'app, non della materia. */
  about: ReactNode
}) {
  const { kana: scope, setSyllabary, setKanaMode, toggleGroup, goTo } = useUi()
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

  // Si guarda cosa il **catalogo** offre e non cosa e' rimasto selezionato: i 外来音
  // esistono solo in katakana, quindi tornando all'hiragana quella scelta non ha piu'
  // un insieme dietro. Filtrando qui, la selezione sopravvive al giro di andata e
  // ritorno fra i due sillabari senza che si possa partire su una coda vuota.
  const chosen = sets?.filter((s) => scope.groups.includes(s.group)) ?? []
  const total = chosen.reduce((sum, s) => sum + s.size, 0)

  return (
    <Screen
      textured
      title="Tanren"
      trailing={about}
      action={
        <Button disabled={chosen.length === 0} onClick={() => goTo('session')}>
          {total > 0 ? `Start with ${total} characters` : 'Start'}
        </Button>
      }
    >
      <div className="flex flex-col gap-7">
        {subjects}

        <Field label="Syllabary">
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

        <Field label="Families">
          {failed && <Note>The catalogue is not reachable.</Note>}
          {!failed && !sets && <Note>Loading…</Note>}
          {sets && (
            <div className="flex flex-wrap gap-2">
              {sets.map((s) => (
                <Chip
                  key={s.group}
                  pressed={scope.groups.includes(s.group)}
                  onClick={() => toggleGroup(s.group)}
                >
                  <span
                    className={GROUP_LABELS[s.group].japanese ? 'font-jp' : undefined}
                    lang={GROUP_LABELS[s.group].japanese ? 'ja' : undefined}
                  >
                    {GROUP_LABELS[s.group].text}
                  </span>
                  <span className="text-muted ml-2 text-xs">{s.size}</span>
                </Chip>
              ))}
            </div>
          )}
        </Field>

        <Field label="Exercise">
          <div className="grid grid-cols-2 gap-2">
            {MODES.map((m) => (
              <Card
                key={m.value}
                pressed={scope.mode === m.value}
                onClick={() => setKanaMode(m.value)}
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
