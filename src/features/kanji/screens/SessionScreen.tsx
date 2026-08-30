import { useState } from 'react'

import {
  kanjiDetails,
  normalizeReading,
  type Kanji,
  type Note,
  type Question,
  type StudyMode,
} from '@/shared/bridge'
import { SessionScreen, type Reveal } from '@/shared/session/SessionScreen'
import { Button } from '@/shared/ui/Button'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

import { KanjiCard } from '../KanjiCard'
import { useKanjiSession } from '../useSession'

/**
 * Un giro di studio sui kanji.
 *
 * # Il Learning comincia presentando, non interrogando
 *
 * Un kanji mai visto non si puo' indovinare: prima lo si conosce come blocco (forma,
 * significato, letture, esempi), poi lo si esercita. Le altre due modalita' non hanno
 * niente da presentare, perche' si e' gia' visto tutto.
 */

const MODE_LABELS: Record<StudyMode, string> = {
  learning: 'Learn',
  review: 'Review',
  drill: 'Drill',
}

/** L'etichetta arriva dal core come chiave, non come testo da mostrare. */
const ASKS_LABELS: Record<string, string> = {
  meaning: 'meaning',
  on: 'on reading',
  kun: 'kun reading',
}

export function KanjiStudyScreen() {
  const { kanji: scope, goTo } = useUi()
  const [introducing, setIntroducing] = useState<string[] | null>(null)
  const session = useKanjiSession(scope, setIntroducing)

  // Prima si conoscono, poi si esercitano. Finita la presentazione la lista si
  // svuota e non torna: il giro sotto e' gia' pronto da un pezzo.
  if (introducing && introducing.length > 0) {
    return (
      <Introduction
        level={scope.level}
        characters={introducing}
        onDone={() => setIntroducing([])}
        onHome={() => goTo('home')}
      />
    )
  }

  return (
    <SessionScreen
      title={MODE_LABELS[scope.mode]}
      accent="bg-type-kanji"
      unit="questions"
      session={session}
      onHome={() => goTo('home')}
      hint={(q) => (q.asks ? (ASKS_LABELS[q.asks] ?? q.asks) : null)}
      reveal={reveal}
      remark={remark}
      input={{ placeholder: 'Type the reading', normalize: normalizeReading }}
    />
  )
}

/** Il significato si risponde in inglese, le letture in kana. */
function reveal(question: Question): Reveal {
  return question.exerciseType === 'kanji.meaning'
    ? { label: 'Means', script: 'latin' }
    : { label: 'Read as', script: 'japanese' }
}

/**
 * Il rilievo sulla grafia, detto per esteso.
 *
 * Il core manda `on_in_hiragana`; la frase la scrive qui la materia, che e' l'unica a
 * sapere che si sta parlando di letture on e katakana.
 */
function remark(note: Note): string {
  switch (note.kind) {
    case 'on_in_hiragana':
      return `Right, and on readings are written in katakana: ${note.expected}`
    case 'kun_in_katakana':
      return `Right, and kun readings are written in hiragana: ${note.expected}`
    default:
      return note.expected
  }
}

/** I kanji nuovi, uno alla volta, prima di cominciare a interrogare. */
function Introduction({
  level,
  characters,
  onDone,
  onHome,
}: {
  level: number
  characters: string[]
  onDone: () => void
  onHome: () => void
}) {
  const [at, setAt] = useState(0)
  const [kanji, setKanji] = useState<Kanji[] | null>(null)

  if (kanji === null) {
    void kanjiDetails(level, characters).then(setKanji).catch(() => setKanji([]))
  }

  const current = kanji?.[at]
  const last = kanji !== null && at >= kanji.length - 1

  return (
    <Screen
      title={`New kanji · ${at + 1} of ${characters.length}`}
      onBack={onHome}
      action={
        <Button
          disabled={kanji === null}
          onClick={() => (last ? onDone() : setAt((i) => i + 1))}
        >
          {last ? 'Start practising' : 'Next'}
        </Button>
      }
    >
      {current ? (
        <KanjiCard kanji={current} />
      ) : (
        <p className="text-muted flex h-full items-center justify-center text-sm">Loading…</p>
      )}
    </Screen>
  )
}
