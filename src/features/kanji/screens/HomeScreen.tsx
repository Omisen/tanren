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
      {reason && <p className="text-muted px-2 text-center text-xs">{reason}</p>}
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
function why(mode: StudyMode, overview?: Overview | null): string | null {
  if (!overview) return 'Loading…'

  if (mode === 'review') {
    return overview.available.due > 0 ? null : 'Nothing is due right now.'
  }
  if (mode === 'drill') {
    return overview.available.practiced > 0 ? null : 'Nothing learned to practise yet.'
  }
  return blocked(overview.available.learning)
}

function blocked(gate: Gate): string | null {
  if (gate.state === 'open') return null
  switch (gate.reason) {
    case 'consolidate':
      return `Consolidate what you have first: recall is at ${Math.round(
        gate.current * 100,
      )}%, and ${Math.round(gate.needed * 100)}% is the mark.`
    case 'wait':
      return `Next learning ${when(gate.until)}.`
    case 'nothing_new':
      return 'You have met every kanji in this level.'
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
function when(iso: string): string {
  const minutes = Math.max(0, (new Date(iso).getTime() - Date.now()) / 60_000)
  const hours = Math.max(1, Math.ceil(minutes / 60))
  return `in ~${hours}h`
}
