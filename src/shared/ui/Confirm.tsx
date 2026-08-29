import { useEffect, useId, type ReactNode } from 'react'

import { Button } from './Button'

/**
 * Una domanda a cui bisogna rispondere prima di andare avanti.
 *
 * Sale dal basso invece di comparire al centro: e' dove arriva il pollice, ed e' la
 * stessa zona in cui stanno le azioni di ogni schermata, quindi non sposta l'abitudine.
 *
 * La conferma porta l'accento, perche' e' l'azione di cui si sta avvisando e deve
 * essere riconoscibile a colpo d'occhio; sta in alto, cosi' non e' quella che il
 * pollice trova per prima. Restare e' la scelta neutra, in fondo.
 */
export function Confirm({
  title,
  children,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
}: {
  title: string
  /** Cosa succede se si conferma. */
  children: ReactNode
  confirmLabel: string
  cancelLabel: string
  onConfirm: () => void
  onCancel: () => void
}) {
  const titleId = useId()

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onCancel()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onCancel])

  return (
    <div className="bg-ink/80 fixed inset-0 z-50 flex flex-col justify-end p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="border-muted/20 bg-ink-soft flex flex-col gap-2 rounded-2xl border p-5"
      >
        <h2 id={titleId} className="text-base font-medium">
          {title}
        </h2>
        <p className="text-muted text-sm">{children}</p>

        <div className="mt-4 flex flex-col gap-2">
          <Button onClick={onConfirm}>{confirmLabel}</Button>
          <Button variant="quiet" autoFocus onClick={onCancel}>
            {cancelLabel}
          </Button>
        </div>
      </div>
    </div>
  )
}
