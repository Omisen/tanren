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
  children,
}: {
  title: string
  /** Se presente, compare la freccia per tornare indietro. */
  onBack?: () => void
  /** L'azione principale, ancorata in fondo. */
  action?: ReactNode
  children: ReactNode
}) {
  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-1 px-4 pt-4 pb-2">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            aria-label="Torna indietro"
            className="text-muted -ml-2 flex size-11 items-center justify-center text-2xl active:opacity-60"
          >
            ←
          </button>
        )}
        <h1 className="text-muted text-sm font-medium tracking-[0.2em] uppercase">
          {title}
        </h1>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">{children}</main>

      {action && <footer className="px-4 pt-2 pb-4">{action}</footer>}
    </div>
  )
}
