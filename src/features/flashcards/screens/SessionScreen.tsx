import {
  normalizeInput,
  type FlashcardScope,
  type Grade,
  type Question,
} from '@/shared/bridge'
import { SessionScreen, type Reveal } from '@/shared/session/SessionScreen'

import { useFlashcardSession } from '../useSession'

/**
 * Il giro su un mazzo: la schermata condivisa, con quello che sanno solo le flashcard.
 *
 * Il titolo e' il **nome del mazzo** e non la modalita', come invece fanno kana e
 * kanji: li' la modalita' e' l'unica cosa che distingue un giro da un altro, qui a
 * distinguerlo e' su cosa si sta lavorando, e il verso si legge gia' dalla riga sopra
 * lo stimolo.
 */

/** Cosa si vuole sapere. Il core manda l'etichetta, la frase la scrive la materia. */
const ASKS_LABELS: Record<string, string> = {
  meaning: 'Meaning',
  japanese: 'Japanese',
}

/**
 * I tre voti fra cui sceglie chi ha indovinato, dal piu' faticoso al piu' facile.
 *
 * `again` non c'e', ed e' la decisione: su una risposta sbagliata il voto e' uno solo,
 * e chiederlo sarebbe un tocco per una cosa gia' decisa. Lo mette il core.
 */
const GRADES: { value: Grade; label: string }[] = [
  { value: 'hard', label: 'Hard' },
  { value: 'good', label: 'Good' },
  { value: 'easy', label: 'Easy' },
]

export function FlashcardSessionScreen({
  deck,
  scope,
  onBack,
}: {
  /** Il nome del mazzo, per il titolo. */
  deck: string
  scope: FlashcardScope
  onBack: () => void
}) {
  const { session, mode } = useFlashcardSession(scope)
  const japanese = scope.direction === 'meaning_to_jp'

  return (
    <SessionScreen
      title={deck}
      accent="bg-type-flashcard"
      unit={{ one: 'card', many: 'cards' }}
      session={session}
      onHome={onBack}
      // Rifare il giro da qui non si puo', ed e' la stessa scelta presa sui kanji: un
      // giro di flashcard nutrira' le scadenze, e ripeterlo subito conterebbe due
      // volte le stesse carte. Il riepilogo resta.
      repeatable={false}
      exitLabel="Back to the deck"
      hint={(q) => (q.asks ? ASKS_LABELS[q.asks] : null)}
      reveal={reveal}
      // Si vota solo dove il voto conta. In un ripasso libero i tre bottoni non
      // sposterebbero niente, e mostrarli sarebbe chiedere una cosa per buttarla via.
      grades={mode === 'review' ? GRADES : undefined}
      input={{
        placeholder: japanese ? 'Type in Japanese' : 'Type the meaning',
        // L'anteprima sotto il campo esiste per un problema solo: un IME puo'
        // restituire katakana a mezza larghezza o segni di sonorizzazione staccati.
        // Scrivendo il significato quel problema non c'e', e mostrare la forma
        // normalizzata farebbe comparire «Irun every day» senza gli spazi, cioe'
        // sembrerebbe che l'app abbia storpiato la risposta. Il giudizio gli spazi li
        // ignora comunque: e' una tolleranza, non una trasformazione da far vedere.
        normalize: japanese ? normalizeInput : unchanged,
        script: japanese ? 'japanese' : 'latin',
        // Una carta porta una parola o una frase, non un segno: il corpo grande e
        // centrato dei kana qui taglierebbe la risposta a meta' invece di renderla
        // leggibile. Vale per tutti e due i versi, perche' anche il lato giapponese
        // di una carta puo' essere una frase intera.
        phrase: true,
      }}
    />
  )
}

function unchanged(value: string): Promise<string> {
  return Promise.resolve(value)
}

/** La risposta attesa sta dall'altra parte della carta rispetto a quello che si vede. */
function reveal(question: Question): Reveal {
  return question.prompt.script === 'japanese'
    ? { label: 'It means', script: 'latin' }
    : { label: 'Written as', script: 'japanese' }
}
