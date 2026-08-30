import type { KanjiMode, Question } from '@/shared/bridge'
import { SessionScreen, type Reveal } from '@/shared/session/SessionScreen'
import { useUi } from '@/shared/store/ui'

import { useKanjiSession } from '../useSession'

/**
 * La sessione sui kanji: la schermata condivisa, con quello che sanno solo i kanji.
 *
 * La differenza vera rispetto ai kana e' la riga sopra lo stimolo. 生 da solo non e'
 * una domanda, perche' ha letture on e letture kun: e' quella riga a renderlo una
 * domanda con una risposta. Le forme con l'okurigana non ne hanno bisogno, perche'
 * 生きる dice gia' da se' cosa si vuole, e infatti il core manda `asks` a `null`.
 */

const MODE_LABELS: Record<KanjiMode, string> = {
  recognition: 'Recognition',
}

/** L'etichetta arriva dal core come chiave (`on`), non come testo da mostrare. */
const ASKS_LABELS: Record<string, string> = {
  on: 'on reading',
  kun: 'kun reading',
}

export function KanjiSessionScreen() {
  const { kanji, goTo } = useUi()
  const session = useKanjiSession(kanji)

  return (
    <SessionScreen
      title={MODE_LABELS[kanji.mode]}
      accent="bg-type-kanji"
      unit="readings"
      session={session}
      onHome={() => goTo('home')}
      hint={(q) => (q.asks ? (ASKS_LABELS[q.asks] ?? q.asks) : null)}
      reveal={reveal}
    />
  )
}

/** Sui kanji si chiede sempre una lettura, e una lettura si scrive in kana. */
function reveal(_question: Question): Reveal {
  return { label: 'Read as', script: 'japanese' }
}
