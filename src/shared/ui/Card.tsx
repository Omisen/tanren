import type { ReactNode } from 'react'

/**
 * Una scelta grande, in una griglia.
 *
 * Usa `aria-pressed` e non solo una classe, cosi' lo stato e' leggibile anche da chi
 * non vede il colore. Premuta porta `selected`, che e' un token di **interfaccia** e
 * non di categoria: questo componente non puo' sapere cosa si sta scegliendo.
 */
export function Card({
  pressed,
  onClick,
  children,
}: {
  pressed: boolean
  onClick: () => void
  children: ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={`flex min-h-20 flex-col items-center justify-center gap-1 rounded-xl border transition-colors active:opacity-70 ${
        pressed ? 'border-selected bg-selected-wash' : 'border-hairline bg-ink-soft'
      }`}
    >
      {children}
    </button>
  )
}

/** Una riga di servizio dentro una sezione: sto caricando, non ci sono riuscito. */
export function Note({ children }: { children: ReactNode }) {
  return <p className="text-muted text-sm">{children}</p>
}
