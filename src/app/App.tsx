import { KanaHomeScreen } from '@/features/kana/screens/HomeScreen'
import { KanaSessionScreen } from '@/features/kana/screens/SessionScreen'
import { KanjiHomeScreen } from '@/features/kanji/screens/HomeScreen'
import { KanjiSessionScreen } from '@/features/kanji/screens/SessionScreen'
import { useUi } from '@/shared/store/ui'

import { SubjectPicker } from './SubjectPicker'

/**
 * La radice dell'app: sceglie quale schermata mostrare.
 *
 * Non c'e' un router. Le schermate sono poche, l'app non ha indirizzi da condividere
 * ne' cronologia del browser da rispettare, e la schermata corrente e' gia' stato
 * effimero dell'interfaccia: tenerla nello store basta e non aggiunge dipendenze.
 *
 * # Perche' la scelta della materia sta qui
 *
 * Perche' questo e' l'unico posto che le conosce tutte. Una feature non puo' nominarne
 * un'altra, quindi la schermata dei kana non potrebbe offrire di passare ai kanji: la
 * pastiglia la compone la radice e la passa giu' come nodo, e ogni materia la mette in
 * cima al proprio modulo di scelta.
 */
export default function App() {
  const screen = useUi((s) => s.screen)
  const subject = useUi((s) => s.subject)

  if (screen === 'session') {
    return subject === 'kana' ? <KanaSessionScreen /> : <KanjiSessionScreen />
  }

  const subjects = <SubjectPicker />
  return subject === 'kana' ? (
    <KanaHomeScreen subjects={subjects} />
  ) : (
    <KanjiHomeScreen subjects={subjects} />
  )
}
