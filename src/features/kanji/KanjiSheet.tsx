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

/**
 * I quattro gradi, e da dove comincia ciascuno.
 *
 * Le fasce sono **semiaperte**: un grado vale dalla sua soglia fino a quella dopo,
 * esclusa, e l'ultima si ferma sotto il pieno. Cosi' nessuna percentuale resta senza
 * grado e nessuna ne ha due. Kohai e' la piu' larga di proposito: e' la fascia del
 * «sto ancora faticando», ed e' quella che deve durare.
 */
const RANKS = [
  { from: 0.75, name: 'Shihan', colour: 'text-rank-shihan' },
  { from: 0.5, name: 'Sensei', colour: 'text-rank-sensei' },
  { from: 0.375, name: 'Senpai', colour: 'text-rank-senpai' },
  { from: 0, name: 'Kohai', colour: 'text-rank-kohai' },
] as const

function rank(progress: number) {
  // Il primo che la soglia lascia passare, scorrendo dall'alto.
  return RANKS.find((r) => progress >= r.from) ?? RANKS[RANKS.length - 1]
}

export function KanjiSheet({
  level,
  character,
  standing,
  progress,
  onClose,
}: {
  level: Level
  character: string
  standing: Standing
  /** Quanto e' consolidato, da 0 a 1, e `null` se non e' mai stato incontrato. */
  progress: number | null
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
    <Sheet title={character} onClose={onClose}>
      <div className="flex flex-col gap-5">
        <Status standing={standing} progress={progress} />

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

/**
 * A che punto e' questo kanji.
 *
 * # Perche' e' una riga sua e non piu' il titolo del pannello
 *
 * Perche' adesso porta tre cose invece di una, e il titolo e' un'etichetta piccola,
 * maiuscola e spaziata: tre informazioni li' dentro si leggono male. Il titolo torna a
 * dire di **cosa** parla il pannello, cioe' il kanji, e lo stato si prende la riga che
 * gli serve.
 *
 * # Le tre forme
 *
 * - mai incontrato: **NOT MET YET** e basta, perche' non c'e' nessun numero da dire;
 * - in corso: lo stato, la percentuale e il grado;
 * - finito: **COMPLETED** al posto di tutto, in `muted` come il resto del testo di
 *   servizio. Un kanji finito smette di gridare, ed e' la riga di prima con una parola
 *   diversa.
 *
 * # Il confine fra 99 e 100
 *
 * COMPLETED si decide da `standing`, **non** dal numero uguale a 1: confrontare un
 * decimale per uguaglianza e' il modo di sbagliare prima o poi. E la percentuale si
 * **tronca**, non si arrotonda, perche' 20,99 giorni su 21 arrotondati darebbero
 * «100% · Shihan» su un kanji che maturo non e'.
 */
function Status({ standing, progress }: { standing: Standing; progress: number | null }) {
  if (standing === 'mature') return <p className="text-muted text-sm">COMPLETED</p>

  if (progress === null) return <p className="text-muted text-sm">NOT MET YET</p>

  const percent = Math.min(99, Math.floor(progress * 100))
  const { name, colour } = rank(progress)

  return (
    <p className="text-sm">
      <span className="text-muted">IN PROGRESS</span>
      <span className="text-inactive"> · </span>
      <span className="text-paper tabular-nums">{percent}%</span>
      <span className="text-inactive"> · </span>
      <span className={`${colour} font-medium`}>{name}</span>
    </p>
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
