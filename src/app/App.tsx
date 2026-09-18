import { FlashcardsHomeScreen } from '@/features/flashcards/screens/HomeScreen'
import { KanaHomeScreen } from '@/features/kana/screens/HomeScreen'
import { KanaSessionScreen } from '@/features/kana/screens/SessionScreen'
import { KanjiLevelsScreen } from '@/features/kanji/screens/LevelsScreen'
import { KanjiHomeScreen } from '@/features/kanji/screens/HomeScreen'
import { KanjiStudyScreen } from '@/features/kanji/screens/SessionScreen'
import { useUi } from '@/shared/store/ui'

import { AboutScreen } from './AboutScreen'
import { SettingsScreen } from './SettingsScreen'
import { SubjectPicker } from './SubjectPicker'

/**
 * La radice dell'app: sceglie quale schermata mostrare.
 *
 * Non c'e' un router. Le schermate sono poche, l'app non ha indirizzi da condividere
 * ne' cronologia del browser da rispettare, e dove ci si trova e' gia' stato effimero
 * dell'interfaccia: tenerlo in Zustand basta e non aggiunge dipendenze.
 *
 * # Sezioni, non piu' un interruttore
 *
 * Kana, kanji e flashcard erano una scelta **dentro** la schermata iniziale, e le
 * impostazioni erano un'icona nell'intestazione. Adesso sono quattro **sezioni**, cioe'
 * quattro posti in cui si va: la radice guarda in quale si e' e monta quella.
 *
 * Dentro una sezione ci si muove con `screen`, e i valori non si sovrappongono perche'
 * una sola sezione e' montata per volta: `session` e `levels` sono dei kana e dei
 * kanji, `about` e' delle impostazioni.
 *
 * # Perche' la navigazione la compone la radice
 *
 * Perche' e' l'unico posto che conosce tutte le sezioni, e la regola di dipendenza
 * vieta a una feature di nominarne un'altra: la schermata dei kana non potrebbe
 * offrire di passare ai kanji. La radice la costruisce e la passa giu' **come nodo**,
 * e ogni sezione la mette dove le serve.
 */
export default function App() {
  const screen = useUi((s) => s.screen)
  const section = useUi((s) => s.section)

  // Finche' non c'e' la barra in fondo, questa e' la via per cambiare sezione, ed e'
  // per questo che la ricevono tutte e quattro invece delle sole materie.
  const sections = <SubjectPicker />

  // I kana e i kanji hanno un giro di studio, che e' una schermata a se': ci si entra
  // dalla loro home e se ne esce solo da li'.
  if (screen === 'session' && section === 'kana') return <KanaSessionScreen />
  if (screen === 'session' && section === 'kanji') return <KanjiStudyScreen />

  // I livelli sono solo dei kanji: e' il loro percorso, e i kana un percorso non ce
  // l'hanno.
  if (screen === 'levels' && section === 'kanji') return <KanjiLevelsScreen />

  // Le fonti sono una cosa dell'app e non di una materia, perche' la licenza dei dati
  // obbliga l'app: si raggiungono dalle impostazioni, che sono l'altra cosa che vale
  // per tutte le sezioni.
  if (screen === 'about' && section === 'settings') return <AboutScreen />

  if (section === 'kana') return <KanaHomeScreen sections={sections} />
  if (section === 'kanji') return <KanjiHomeScreen sections={sections} />
  if (section === 'flashcards') return <FlashcardsHomeScreen sections={sections} />
  return <SettingsScreen sections={sections} />
}
