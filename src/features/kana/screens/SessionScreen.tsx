import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

/**
 * Il guscio della sessione di studio.
 *
 * Le domande arrivano nei passaggi successivi: qui c'e' solo l'impianto della
 * schermata e il ritorno indietro.
 */
export function SessionScreen() {
  const { scope, goTo } = useUi()

  return (
    <Screen title="Sessione" onBack={() => goTo('home')}>
      <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
        <p className="font-jp text-5xl" lang="ja">
          鍛
        </p>
        <p className="text-muted text-sm">
          {scope.syllabary}, {scope.groups.join(' + ') || 'nessuna famiglia'},{' '}
          {scope.mode}
        </p>
        <p className="text-muted/60 text-xs">Le domande arrivano al prossimo passo.</p>
      </div>
    </Screen>
  )
}
