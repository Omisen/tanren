import { useEffect, useState, type ReactNode } from 'react'

import { kanjiCatalogue, type Family, type Grade, type KanjiSet } from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Card, Note } from '@/shared/ui/Card'
import { Chip } from '@/shared/ui/Chip'
import { Field } from '@/shared/ui/Field'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

/**
 * La scelta dell'ambito sui kanji: un anno di scuola piu' delle famiglie di letture.
 *
 * E' la stessa forma della scelta sui kana, dove si prende un sillabario e delle
 * famiglie di segni. I sette gradi stanno qui e non arrivano dal core, come i due
 * sillabari: sono sette, non cambiano, e il core deve dire quanti item contengono, non
 * come si chiamano.
 */

const GRADES: { value: Grade; label: string; caption: string }[] = [
  { value: 'first', label: '1', caption: 'Grade 1' },
  { value: 'second', label: '2', caption: 'Grade 2' },
  { value: 'third', label: '3', caption: 'Grade 3' },
  { value: 'fourth', label: '4', caption: 'Grade 4' },
  { value: 'fifth', label: '5', caption: 'Grade 5' },
  { value: 'sixth', label: '6', caption: 'Grade 6' },
  { value: 'secondary', label: '中', caption: 'Secondary' },
]

const FAMILY_LABELS: Record<Family, string> = {
  on: 'On',
  kun: 'Kun',
  okurigana: 'Okurigana',
}

export function KanjiHomeScreen({ subjects }: { subjects: ReactNode }) {
  const { kanji: scope, setGrade, toggleFamily, goTo } = useUi()
  // Il catalogo si porta dietro il grado a cui appartiene, come quello dei kana col
  // sillabario: cambiando scelta il risultato vecchio smette di valere da solo.
  const [loaded, setLoaded] = useState<{
    grade: Grade
    /** `null` se la richiesta e' fallita. */
    sets: KanjiSet[] | null
  } | null>(null)

  useEffect(() => {
    let current = true
    const grade = scope.grade

    kanjiCatalogue(grade)
      .then((sets) => current && setLoaded({ grade, sets }))
      .catch(() => current && setLoaded({ grade, sets: null }))

    return () => {
      current = false
    }
  }, [scope.grade])

  const fresh = loaded?.grade === scope.grade ? loaded : undefined
  const sets = fresh?.sets ?? null
  const failed = fresh?.sets === null

  const chosen = sets?.filter((s) => scope.families.includes(s.family)) ?? []
  const total = chosen.reduce((sum, s) => sum + s.size, 0)

  return (
    <Screen
      textured
      title="Tanren"
      action={
        <Button disabled={scope.families.length === 0} onClick={() => goTo('session')}>
          {total > 0 ? `Start with ${total} readings` : 'Start'}
        </Button>
      }
    >
      <div className="flex flex-col gap-7">
        {subjects}

        <Field label="School year">
          <div className="grid grid-cols-4 gap-2">
            {GRADES.map((g) => (
              <Card
                key={g.value}
                pressed={scope.grade === g.value}
                onClick={() => setGrade(g.value)}
              >
                <span className="font-jp text-2xl" lang="ja">
                  {g.label}
                </span>
                <span className="text-muted text-[0.6rem]">{g.caption}</span>
              </Card>
            ))}
          </div>
        </Field>

        <Field label="Readings">
          {failed && <Note>The catalogue is not reachable.</Note>}
          {!failed && !sets && <Note>Loading…</Note>}
          {sets && (
            <div className="flex flex-wrap gap-2">
              {sets.map((s) => (
                <Chip
                  key={s.family}
                  pressed={scope.families.includes(s.family)}
                  onClick={() => toggleFamily(s.family)}
                >
                  {FAMILY_LABELS[s.family]}
                  <span className="text-muted ml-2 text-xs">{s.size}</span>
                </Chip>
              ))}
            </div>
          )}
        </Field>
      </div>
    </Screen>
  )
}
