import type { ReactNode } from 'react'

/**
 * Una scelta a pastiglia, premuta o no.
 *
 * Usa `aria-pressed` invece di una classe soltanto, cosi' lo stato e' leggibile anche
 * da chi non vede il colore.
 */
export function Chip({
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
      className={`min-h-11 rounded-full border px-4 text-sm transition-colors active:opacity-70 ${
        pressed
          ? 'border-accent bg-accent-wash text-paper'
          : 'border-hairline text-muted'
      }`}
    >
      {children}
    </button>
  )
}
