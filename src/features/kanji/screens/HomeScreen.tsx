import { useCallback, useEffect, useState, type ReactNode } from 'react'

import {
  kanjiCurrentLevel,
  kanjiOverview,
  type Gate,
  type Level,
  type Overview,
  type StudyMode,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { LogoMark } from '@/shared/ui/LogoMark'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

import { LevelBlock } from '../LevelBlock'

/**
 * La home dei kanji: dove sei, e cosa puoi fare adesso.
 *
 * # Perche' non e' piu' un filtro
 *
 * La versione precedente era una query su un dataset: scegli un anno di scuola,
 * scegli un tipo di lettura, parti con «80 readings». Metteva al centro le letture
 * invece del kanji, non mostrava mai un significato, e usava l'anno scolastico
 * giapponese come asse, che non e' un ordine di difficolta' per chi non e'
 * madrelingua. Qui il centro e' il kanji, l'asse e' un percorso, e l'avanzamento e' una
 * cosa che si vede.
 *
 * # Perche' qui e' rimasto poco
 *
 * Faceva tre mestieri: diceva dove sei, mostrava cosa c'e' nel livello e offriva cosa
 * fare. Il terzo e' l'unico che si fa ogni giorno, e stava schiacciato in fondo sotto
 * gli altri due. Ora restano il banner del livello a cui si e' arrivati e i **tre
 * bottoni**, che diventano quello che sono sempre stati, cioe' il motivo per cui si
 * apre l'app; il selettore dei livelli e la griglia sono passati alla vista dei
 * livelli, che si consulta di rado.
 *
 * # Il livello qui e' uno solo, ed e' quello vero
 *
 * Non si sceglie piu' quale guardare: il livello e' quello a cui si e' arrivati, letto
 * dal core. Da qui sparisce quindi anche l'idea di un livello **chiuso**, perche' su
 * quello raggiunto non si e' mai chiusi: guardare piu' avanti si fa nella vista dei
 * livelli, che infatti non studia niente.
 */

const MODES: { value: StudyMode; label: string; caption: string }[] = [
  { value: 'learning', label: 'Learn', caption: 'Meet new kanji' },
  { value: 'review', label: 'Review', caption: 'What is due' },
  { value: 'drill', label: 'Drill', caption: 'Free practice' },
]

export function KanjiHomeScreen({
  subjects,
  about,
}: {
  subjects: ReactNode
  /** La via per le fonti, messa dalla radice: e' cosa dell'app, non della materia. */
  about: ReactNode
}) {
  const { kanji: scope, setLevel, study, goTo } = useUi()
  const [overview, setOverview] = useState<{ level: Level; data: Overview | null } | null>(null)

  // Fin dove si e' arrivati lo sa il core: e' il primo livello non ancora consolidato.
  // Si chiede una volta sola, e la schermata ci si porta sopra.
  useEffect(() => {
    let current = true
    kanjiCurrentLevel()
      .then((level) => current && setLevel(level))
      .catch(() => {})
    return () => {
      current = false
    }
  }, [setLevel])

  // Il livello guardato porta con se' i propri dati, come il catalogo dei kana col
  // sillabario: cambiando livello il risultato vecchio smette di valere da solo.
  useEffect(() => {
    let current = true
    const level = scope.level

    kanjiOverview({ ...scope, level })
      .then((data) => current && setOverview({ level, data }))
      .catch(() => current && setOverview({ level, data: null }))

    return () => {
      current = false
    }
    // La modalita' non cambia cosa mostrare: cambia solo cosa succede toccando un
    // bottone, e non vale una richiesta in piu'.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, [scope.level])

  const fresh = overview?.level === scope.level ? overview.data : undefined

  const start = useCallback((mode: StudyMode) => study(mode), [study])

  return (
    <Screen
      textured
      title="Tanren"
      mark={<LogoMark />}
      trailing={about}
      action={
        <div className="flex flex-col gap-2">
          {MODES.map((m) => (
            <ModeButton key={m.value} mode={m} overview={fresh} onStart={start} />
          ))}
        </div>
      }
    >
      <div className="flex flex-col gap-7">
        {subjects}

        <LevelBlock level={scope.level} progress={fresh?.progress} />

        <Button variant="quiet" onClick={() => goTo('levels')}>
          Explore the kanji
        </Button>
      </div>
    </Screen>
  )
}

/** Un bottone che, quando non si puo' premere, dice perche'. */
function ModeButton({
  mode,
  overview,
  onStart,
}: {
  mode: { value: StudyMode; label: string; caption: string }
  overview?: Overview | null
  onStart: (mode: StudyMode) => void
}) {
  const reason = why(mode.value, overview)

  return (
    <div className="flex flex-col gap-1">
      <Button
        variant={mode.value === 'learning' ? 'primary' : 'quiet'}
        disabled={reason !== null}
        onClick={() => onStart(mode.value)}
      >
        {mode.label}
        {count(mode.value, overview)}
      </Button>
      {/* Lo spazio del motivo e' sempre occupato, e la misura non e' un numero di
          righe scelto a mano: e' il **messaggio piu' lungo che questo bottone possa
          dire**, disegnato invisibile nella stessa cella. Cosi' l'altezza la decide
          come quel testo va a capo alla larghezza vera dello schermo, quindi resta
          giusta su un telefono stretto come su uno largo, e resta giusta anche se un
          domani si riscrive una frase.
          Serve perche' tornando da una sessione i dati non ci sono ancora: qui compare
          «Loading…» e un istante dopo lascia il posto al motivo definitivo. Comparendo
          e cambiando spostava i tre bottoni proprio mentre li si sta per toccare, e
          premere Drill al posto di Learn non e' un difetto estetico. */}
      <div className="grid px-2 text-center text-xs">
        <p className="invisible col-start-1 row-start-1" aria-hidden="true">
          {longest(mode.value)}
        </p>
        <p className="text-muted col-start-1 row-start-1">{reason}</p>
      </div>
    </div>
  )
}

function count(mode: StudyMode, overview?: Overview | null): string {
  if (!overview) return ''
  if (mode === 'review') return overview.available.due > 0 ? ` · ${overview.available.due}` : ''
  if (mode === 'drill') return overview.available.practiced > 0 ? ' · ready' : ''
  return overview.available.learning.state === 'open'
    ? ` · ${overview.available.learning.room}`
    : ''
}

/**
 * Perche' un bottone e' spento, detto con parole utili.
 *
 * «Torna domani» non insegna niente. «Consolida quello che hai» e «torna fra tre ore»
 * sono due consigli diversi, e chi studia ha diritto di sapere quale dei due vale.
 */
/**
 * Tutto quello che un bottone spento puo' dire, scritto in un posto solo.
 *
 * Sta qui e non sparso fra le funzioni perche' serve due volte: per **mostrare** il
 * motivo, e per **riservargli lo spazio** prima di conoscerlo. Due elenchi separati si
 * sganciarebbero al primo ritocco a una frase, e lo spazio riservato smetterebbe di
 * bastare senza che nessuno se ne accorga.
 */
const MESSAGES = {
  loading: 'Loading…',
  nothingDue: 'Nothing is due right now.',
  nothingLearned: 'Nothing learned to practise yet.',
  levelDone: 'You have met every kanji in this level.',
  consolidate: (current: number, needed: number) =>
    `Consolidate what you have first: recall is at ${current}%, and ${needed}% is the mark.`,
  wait: (hours: number) => `Next learning in ~${hours}h.`,
}

/**
 * Il messaggio piu' lungo che **questo** bottone possa mostrare.
 *
 * Per bottone e non uno per tutti: il motivo lungo, quello del consolidamento, puo'
 * comparire solo sotto Learn, quindi riservare a tutti e tre lo spazio del piu' lungo
 * lascerebbe due righe di vuoto sotto Review e Drill per sempre.
 *
 * Gli argomenti sono i casi peggiori: cento per cento occupa piu' di sessantadue, e
 * ventiquattro ore piu' di quattro.
 */
function longest(mode: StudyMode): string {
  const possibili =
    mode === 'review'
      ? [MESSAGES.loading, MESSAGES.nothingDue]
      : mode === 'drill'
        ? [MESSAGES.loading, MESSAGES.nothingLearned]
        : [
            MESSAGES.loading,
            MESSAGES.levelDone,
            MESSAGES.consolidate(100, 100),
            MESSAGES.wait(24),
          ]

  return possibili.reduce((a, b) => (b.length > a.length ? b : a))
}

function why(mode: StudyMode, overview?: Overview | null): string | null {
  if (!overview) return MESSAGES.loading

  if (mode === 'review') {
    return overview.available.due > 0 ? null : MESSAGES.nothingDue
  }
  if (mode === 'drill') {
    return overview.available.practiced > 0 ? null : MESSAGES.nothingLearned
  }
  return blocked(overview.available.learning)
}

function blocked(gate: Gate): string | null {
  if (gate.state === 'open') return null
  switch (gate.reason) {
    case 'consolidate':
      return MESSAGES.consolidate(
        Math.round(gate.current * 100),
        Math.round(gate.needed * 100),
      )
    case 'wait':
      return MESSAGES.wait(hoursUntil(gate.until))
    case 'nothing_new':
      return MESSAGES.levelDone
  }
}

/**
 * Fra quanto si riapre, in ore e **arrotondate per eccesso**.
 *
 * Per eccesso perche' i due errori non costano uguale: chi torna sull'ora annunciata e
 * trova il bottone ancora spento ha ricevuto una promessa non mantenuta, mentre chi lo
 * trova gia' acceso ha avuto una sorpresa buona. Quindi 3h50 si dice «~4h», mai «3h».
 *
 * E' un valore **statico**, calcolato quando la schermata si disegna: la granularita'
 * e' l'ora, quindi un timer che scorre al secondo non direbbe niente di piu'.
 */
function hoursUntil(iso: string): number {
  const minutes = Math.max(0, (new Date(iso).getTime() - Date.now()) / 60_000)
  return Math.max(1, Math.ceil(minutes / 60))
}
