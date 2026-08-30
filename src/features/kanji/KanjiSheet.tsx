import { useEffect, useState } from 'react'

import { kanjiDetails, type Kanji, type Level, type Standing } from '@/shared/bridge'
import { Sheet } from '@/shared/ui/Sheet'

import { KanjiCard } from './KanjiCard'

/**
 * La scheda di un kanji, aperta dalla griglia.
 *
 * Dentro c'e' la **stessa** `KanjiCard` che il Learning mostra per presentarlo: quello
 * che serve a conoscere un kanji e quello che serve a riguardarlo sono la stessa cosa,
 * e due schede diverse divergerebbero come sono gia' divergiti `Chip` e `Card`.
 *
 * Si apre anche sui livelli chiusi: guardare cosa arrivera' non e' barare, e' l'unico
 * modo di farsi un'idea del percorso.
 */

const STANDING_LABELS: Record<Standing, string> = {
  new: 'not met yet',
  learning: 'in progress',
  mature: 'consolidated',
}

export function KanjiSheet({
  level,
  character,
  standing,
  onClose,
}: {
  level: Level
  character: string
  standing: Standing
  onClose: () => void
}) {
  const [tab, setTab] = useState<'info' | 'related'>('info')
  const [kanji, setKanji] = useState<Kanji | null | 'failed'>(null)

  useEffect(() => {
    let current = true
    kanjiDetails(level, [character])
      .then((found) => current && setKanji(found[0] ?? 'failed'))
      .catch(() => current && setKanji('failed'))
    return () => {
      current = false
    }
  }, [level, character])

  return (
    <Sheet title={STANDING_LABELS[standing]} onClose={onClose}>
      <div className="flex flex-col gap-5">
        <div className="flex gap-2">
          <Tab active={tab === 'info'} onClick={() => setTab('info')}>
            Info
          </Tab>
          <Tab active={tab === 'related'} onClick={() => setTab('related')}>
            Related
          </Tab>
        </div>

        {tab === 'info' &&
          (kanji === null ? (
            <p className="text-muted text-sm">Loading…</p>
          ) : kanji === 'failed' ? (
            <p className="text-muted text-sm">This kanji could not be loaded.</p>
          ) : (
            <KanjiCard kanji={kanji} />
          ))}

        {tab === 'related' && (
          /* Segnaposto dichiarato, non una tab vuota per finta: il dato dei componenti
             c'e' gia' in kanjium (`elements.kanji_parts` e `part_of`), quello che manca
             e' decidere se i componenti diventino a loro volta cose da imparare, e con
             quali dipendenze. E' una fase a se'. */
          <div className="flex flex-col gap-2 py-4">
            <p className="text-base">Not here yet.</p>
            <p className="text-muted text-sm">
              This is where the pieces a kanji is built from will go, and the other kanji
              that share them. The data is already in the source; what is missing is
              deciding whether those pieces become things you learn in their own right.
            </p>
          </div>
        )}
      </div>
    </Sheet>
  )
}

function Tab({
  active,
  onClick,
  children,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`min-h-11 rounded-full border px-4 text-sm transition-colors active:opacity-70 ${
        active
          ? 'border-selected bg-selected-wash text-paper'
          : 'border-hairline text-muted'
      }`}
    >
      {children}
    </button>
  )
}
