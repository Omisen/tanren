import { HomeScreen } from '@/features/kana/screens/HomeScreen'
import { SessionScreen } from '@/features/kana/screens/SessionScreen'
import { useUi } from '@/shared/store/ui'

/**
 * La radice dell'app: sceglie quale schermata mostrare.
 *
 * Non c'e' un router. Le schermate sono poche, l'app non ha indirizzi da condividere
 * ne' cronologia del browser da rispettare, e la schermata corrente e' gia' stato
 * effimero dell'interfaccia: tenerla nello store basta e non aggiunge dipendenze.
 */
export default function App() {
  const screen = useUi((s) => s.screen)

  switch (screen) {
    case 'home':
      return <HomeScreen />
    case 'session':
      return <SessionScreen />
  }
}
