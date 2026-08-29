import type { ButtonHTMLAttributes } from 'react'

type Variant = 'primary' | 'quiet'

const base =
  'flex min-h-12 w-full items-center justify-center rounded-xl px-4 text-base font-medium transition-opacity active:opacity-70 disabled:opacity-40'

const variants: Record<Variant, string> = {
  primary: 'bg-accent text-white',
  quiet: 'bg-ink-soft text-paper',
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
