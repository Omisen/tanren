import { useEffect, useRef, useState } from 'react'

import {
  kanjiCurrentLevel,
  kanjiDashboard,
  kanjiGrid,
  type KanjiCell,
  type Level,
  type LevelSummary,
  type Standing,
} from '@/shared/bridge'
import { Note } from '@/shared/ui/Card'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

import { KanjiSheet } from '../KanjiSheet'
import { LevelBlock } from '../LevelBlock'

/**
 * I kanji del percorso, da consultare.
 *
 * # Perche' si chiama coi kanji e non coi livelli
 *
 * Il bottone che porta qui diceva «Browse the levels», e puntava alla cosa sbagliata:
 * il livello e' gia' scritto nel banner sopra, e chi comincia **non sa cosa contenga**
 * un livello, quindi la parola non gli promette niente. Quello che incuriosisce sono i
 * kanji, cioe' cosa c'e' dentro; che siano divisi in livelli lo si scopre entrando.
 *
 * # Cosa misura, e cosa no
 *
 * Misura **quanto sei consolidato**, che e' quello che dice FSRS, e lo alimentano solo
 * il Learning e il Ripasso. Il Drill non compare qui e non deve: e' pratica in piu', e
 * come e' andata oggi si vede alla fine del giro e finisce li'. Sono due misure
 * diverse, e mescolarle farebbe sembrare progresso quello che e' solo esercizio.
 *
 * # Si guarda, non si studia
 *
 * Scegliere un livello qui **non** cambia il livello attivo e non sblocca niente: si
 * puo' guardare il livello 40 senza poterci andare, ed e' coerente con un percorso
 * sequenziale. Lo studio parte sempre dalla home, sul livello a cui si e' arrivati
 * davvero. Per questo il livello scelto vive qui dentro e non nello store: uscendo si
 * dimentica, perche' non e' una decisione, e' uno sguardo.
 *
 * # Una richiesta sola per ottantasei livelli
 *
 * La lista arriva tutta insieme da `kanjiDashboard`, che e' la stessa interrogazione
 * unica che alimentava la vista del progresso: serve sia a riempire il selettore sia a
 * dire i numeri del livello scelto, senza una chiamata per livello.
 */
export function KanjiLevelsScreen() {
  const goTo = useUi((s) => s.goTo)
  const [levels, setLevels] = useState<LevelSummary[] | null | 'failed'>(null)
  const [selected, setSelected] = useState<Level | null>(null)
  const [grid, setGrid] = useState<{ level: Level; cells: KanjiCell[] } | null>(null)
  const [opened, setOpened] = useState<KanjiCell | null>(null)

  useEffect(() => {
    let current = true
    kanjiDashboard()
      .then((rows) => current && setLevels(rows))
      .catch(() => current && setLevels('failed'))
    // Si apre su dove sei, non sul livello 1: cercarsi scorrendo ottantasei numeri
    // sarebbe il primo gesto di ogni visita.
    kanjiCurrentLevel()
      .then((level) => current && setSelected(level))
      .catch(() => current && setSelected(1))
    return () => {
      current = false
    }
  }, [])

  // La griglia segue il livello scelto, e si porta dietro di quale livello e': cambiando
  // scelta il risultato vecchio smette di valere da solo, senza azzeramenti a mano.
  useEffect(() => {
    if (selected === null) return
    let current = true
    const level = selected
    kanjiGrid(level)
      .then((cells) => current && setGrid({ level, cells }))
      .catch(() => current && setGrid({ level, cells: [] }))
    return () => {
      current = false
    }
  }, [selected])

  const rows = Array.isArray(levels) ? levels : []
  const mature = rows.reduce((n, l) => n + l.mature, 0)
  const total = rows.reduce((n, l) => n + l.total, 0)
  const shown = rows.find((l) => l.level === selected)
  const cells = grid?.level === selected ? grid.cells : null

  return (
    <>
      <Screen title="Kanji" onBack={() => goTo('home')}>
        {levels === null && <Note>Loading…</Note>}
        {levels === 'failed' && <Note>The path could not be loaded.</Note>}

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

            <Strip levels={rows} selected={selected} onPick={setSelected} />

            {shown && (
              <LevelBlock
                level={shown.level}
                progress={shown}
                recall={shown.recall}
                unlocked={shown.unlocked}
              />
            )}

            {shown &&
              (cells === null ? (
                <GridSkeleton count={shown.total} />
              ) : (
                <Grid cells={cells} onOpen={setOpened} />
              ))}

            <p className="text-muted text-xs">
              This is what spaced repetition knows, and only Learn and Review feed it.
              Drill never moves these numbers: how a practice round went is shown at the
              end of that round and stays there.
            </p>
          </div>
        )}
      </Screen>

      {/* La scheda si apre anche sui livelli chiusi: guardare cosa arrivera' non e'
          barare, ed e' l'unico modo di farsi un'idea del percorso. */}
      {opened && selected !== null && (
        <KanjiSheet
          level={selected}
          character={opened.character}
          standing={opened.standing}
          onClose={() => setOpened(null)}
        />
      )}
    </>
  )
}

/**
 * Tutti i livelli, in una striscia che scorre di lato.
 *
 * **Di lato e non in colonna**: una lista verticale di ottantasei righe spingerebbe la
 * griglia fuori dallo schermo, e la griglia e' meta' del motivo per cui si e' qui. Di
 * lato si scorre col pollice e si vede subito che i livelli non sono tre.
 *
 * Si porta da sola sul livello scelto la prima volta che si disegna, altrimenti aprire
 * sul livello corrente non si vedrebbe: il numero sarebbe selezionato ma fuori campo.
 */
function Strip({
  levels,
  selected,
  onPick,
}: {
  levels: LevelSummary[]
  selected: Level | null
  onPick: (level: Level) => void
}) {
  const box = useRef<HTMLDivElement>(null)
  const portato = useRef(false)

  useEffect(() => {
    if (portato.current || selected === null || !box.current) return
    const bersaglio = box.current.querySelector(`[data-level="${selected}"]`)
    bersaglio?.scrollIntoView({ block: 'nearest', inline: 'center' })
    portato.current = true
  }, [selected])

  return (
    <div ref={box} className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-1">
      {levels.map((l) => (
        <button
          key={l.level}
          type="button"
          data-level={l.level}
          aria-pressed={l.level === selected}
          aria-label={`Level ${l.level}`}
          onClick={() => onPick(l.level)}
          className={`min-w-11 shrink-0 rounded-lg border px-3 py-2 tabular-nums transition-colors active:opacity-70 ${
            l.level === selected
              ? 'border-selected bg-selected-wash text-paper'
              : l.unlocked
                ? 'border-hairline text-paper'
                : 'border-hairline-soft text-inactive'
          }`}
        >
          {l.level}
        </button>
      ))}
    </div>
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

/**
 * La geometria della griglia, scritta una volta sola.
 *
 * La usano la griglia vera e il suo segnaposto, e devono essere **identiche**: e' tutto
 * il punto del segnaposto, e due copie di queste classi si sganciarebbero al primo
 * ritocco a una delle due.
 */
const GRIGLIA = 'grid grid-cols-6 gap-1.5'

/**
 * Lo spazio della griglia mentre i kanji arrivano.
 *
 * # Perche' non basta scrivere «Loading…»
 *
 * Perche' quella riga e' alta una riga, e la griglia e' alta cinque: cambiando livello
 * il blocco si accartocciava da circa trecento pixel a venti e si riapriva un istante
 * dopo, trascinandosi dietro tutto quello che sta sotto. Sono due salti in una frazione
 * di secondo, ed e' quello che si vede: non sono i kanji di prima che restano, perche'
 * quelli spariscono dal primo disegno utile (la griglia si mostra solo se e' del
 * livello che si sta guardando). E' il vuoto che si apre e si richiude.
 *
 * # Perche' il conteggio e' esatto e non stimato
 *
 * Quante caselle servano lo dice gia' il dato che la schermata ha in mano: la riga del
 * livello porta il suo `total`, arrivato con l'interrogazione unica. Quindi il
 * segnaposto ha **la misura giusta**, non una probabile, e il riquadro non si muove di
 * un pixel fra il vuoto e il pieno.
 */
function GridSkeleton({ count }: { count: number }) {
  return (
    <div className={GRIGLIA} aria-hidden="true">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className="border-hairline-soft aspect-square rounded-lg border" />
      ))}
    </div>
  )
}

function Grid({ cells, onOpen }: { cells: KanjiCell[]; onOpen: (cell: KanjiCell) => void }) {
  if (cells.length === 0) return <Note>Nothing here.</Note>

  return (
    <div className={GRIGLIA}>
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
