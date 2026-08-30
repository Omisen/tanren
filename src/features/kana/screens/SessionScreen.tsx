import { normalizeInput, type KanaMode, type Question, type Syllabary } from '@/shared/bridge'
import { SessionScreen, type Reveal } from '@/shared/session/SessionScreen'
import { useUi } from '@/shared/store/ui'

import { useKanaSession } from '../useSession'

/**
 * La sessione sui kana: la schermata condivisa, con quello che sanno solo i kana.
 *
 * Sono quattro cose: come si chiama la modalita', di che colore e' il blocco dello
 * stimolo, come si chiama quello che si conta, e le due righe di testo che dipendono
 * da cosa si sta chiedendo.
 */

const MODE_LABELS: Record<KanaMode, string> = {
  recognition: 'Recognition',
  input: 'Writing',
}

const SYLLABARY_LABELS: Record<Syllabary, string> = {
  hiragana: 'hiragana',
  katakana: 'katakana',
}

export function KanaSessionScreen() {
  const { kana, goTo } = useUi()
  const session = useKanaSession(kana)

  return (
    <SessionScreen
      title={MODE_LABELS[kana.mode]}
      accent="bg-type-kana"
      unit="characters"
      session={session}
      onHome={() => goTo('home')}
      // Scrivendo, il prompt e' una trascrizione, e `ka` vale sia per か sia per カ:
      // senza dire quale sillabario la domanda avrebbe due risposte, e il core ne
      // accetta una sola.
      hint={(q) => (q.format.mode === 'input' ? `in ${SYLLABARY_LABELS[kana.syllabary]}` : null)}
      reveal={reveal}
      input={{ placeholder: 'Type the character', normalize: normalizeInput }}
    />
  )
}

/**
 * La risposta attesa e' nell'alfabeto opposto a quello della domanda: si mostra il
 * segno se si chiedeva la trascrizione, e viceversa.
 */
function reveal(question: Question): Reveal {
  return question.prompt.script === 'japanese'
    ? { label: 'Read as', script: 'latin' }
    : { label: 'Written as', script: 'japanese' }
}
