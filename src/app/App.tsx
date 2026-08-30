import { KanaHomeScreen } from '@/features/kana/screens/HomeScreen'
import { KanaSessionScreen } from '@/features/kana/screens/SessionScreen'
import { KanjiDashboardScreen } from '@/features/kanji/screens/DashboardScreen'
import { KanjiHomeScreen } from '@/features/kanji/screens/HomeScreen'
import { KanjiStudyScreen } from '@/features/kanji/screens/SessionScreen'
import { useUi } from '@/shared/store/ui'

import { AboutScreen } from './AboutScreen'
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
    return subject === 'kana' ? <KanaSessionScreen /> : <KanjiStudyScreen />
  }

  // La dashboard e' solo dei kanji: e' la misura del loro percorso, e i kana un
  // percorso non ce l'hanno.
  if (screen === 'dashboard') return <KanjiDashboardScreen />

  // Le fonti sono una cosa dell'app, non di una materia: la licenza dei dati obbliga
  // ad attribuire dentro il mezzo in cui l'app viaggia, e qui e' l'unico posto che le
  // conosce tutte.
  if (screen === 'about') return <AboutScreen />

  const subjects = <SubjectPicker />
  const about = <AboutButton />
  return subject === 'kana' ? (
    <KanaHomeScreen subjects={subjects} about={about} />
  ) : (
    <KanjiHomeScreen subjects={subjects} about={about} />
  )
}

/**
 * La via per le fonti.
 *
 * In cima e non nella fascia in fondo: quella e' della cosa che si fa decine di volte,
 * e questa si tocca di rado. Ma dev'esserci **in tutte e due le schermate iniziali**,
 * perche' l'obbligo di attribuzione non dipende da quale materia si sta guardando.
 */
function AboutButton() {
  const goTo = useUi((s) => s.goTo)

  return (
    <button
      type="button"
      onClick={() => goTo('about')}
      aria-label="Sources and licences"
      className="text-muted flex size-11 items-center justify-center text-lg active:opacity-60"
    >
      ⓘ
    </button>
  )
}
