import type { ButtonHTMLAttributes } from 'react'

type Variant = 'primary' | 'quiet' | 'danger'

const base =
  'flex min-h-12 w-full items-center justify-center rounded-full px-5 text-base font-medium transition-opacity hover:opacity-90 active:opacity-70 disabled:opacity-40'

const variants: Record<Variant, string> = {
  // Crema con testo scuro, non piu' rosso con testo bianco: il rosso dice
  // «sbagliato» a due centimetri di distanza, e con il bianco sopra stava anche
  // sotto la soglia di contrasto (4,35x contro 4,50x). Cosi' sono 12,7x.
  primary: 'bg-action text-ink',
  quiet: 'bg-ink-soft text-paper',
  // L'accento resta qui, ed e' l'unico posto in cui fa da fondo: dice
  // «attenzione», la stessa cosa che dice sull'opzione sbagliata. Nella versione
  // scurita, perche' sotto testo bianco quello pieno non passava il contrasto.
  danger: 'bg-accent-strong text-on-accent',
}

/**
 * Il bottone dell'app.
 *
 * L'altezza minima e' di 48 pixel, che e' la misura sotto la quale un bersaglio
 * diventa scomodo da centrare con il pollice.
 */
export function Button({
  variant = 'primary',
  className = '',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  return (
    <button
      type="button"
      className={`${base} ${variants[variant]} ${className}`}
      {...props}
    />
  )
}
