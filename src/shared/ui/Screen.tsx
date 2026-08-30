import type { ReactNode } from 'react'

/**
 * L'impianto di ogni schermata.
 *
 * Tre fasce: un'intestazione bassa, il contenuto che scorre, e in fondo la zona delle
 * azioni. Le azioni stanno in basso perche' il caso d'uso primario e' il telefono
 * tenuto in una mano: il pollice arriva comodamente al bordo inferiore, molto meno a
 * quello superiore.
 */
export function Screen({
  title,
  onBack,
  action,
  trailing,
  textured = false,
  children,
}: {
  title: string
  /** Se presente, compare la freccia per tornare indietro. */
  onBack?: () => void
  /** L'azione principale, ancorata in fondo. */
  action?: ReactNode
  /**
   * Una via secondaria, in cima a destra.
   *
   * Sta li' e non nella fascia in fondo perche' quella e' della cosa che si fa
   * decine di volte; questa si tocca di rado e non deve rubarle il posto.
   */
  trailing?: ReactNode
  /** Il reticolo di sfondo. Solo dove non c'e' uno stimolo da proteggere. */
  textured?: boolean
  children: ReactNode
}) {
  return (
    <div className={`flex h-full flex-col ${textured ? 'paper-grid' : ''}`}>
      <header className="flex items-center gap-1 px-4 pt-4 pb-2">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            aria-label="Go back"
            className="text-muted -ml-2 flex size-11 items-center justify-center text-2xl active:opacity-60"
          >
            ←
          </button>
        )}
        <h1 className="text-muted text-sm font-medium tracking-[0.2em] uppercase">
          {title}
        </h1>
        {trailing && <div className="ml-auto">{trailing}</div>}
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">{children}</main>

      {action && <footer className="px-4 pt-2 pb-4">{action}</footer>}
    </div>
  )
}
