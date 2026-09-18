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
  kind = 'warning',
}: {
  title: string
  /** Cosa succede se si conferma. */
  children: ReactNode
  confirmLabel: string
  cancelLabel: string
  onConfirm: () => void
  onCancel: () => void
  /** Se si sta avvisando di qualcosa, o si stanno offrendo due strade pari. */
  kind?: 'warning' | 'choice'
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
    <div className="bg-scrim fixed inset-0 z-50 flex flex-col justify-end p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="border-hairline bg-ink-soft flex flex-col gap-2 rounded-2xl border p-5"
      >
        <h2 id={titleId} className="text-base font-medium">
          {title}
        </h2>
        <p className="text-muted text-sm">{children}</p>

        {kind === 'choice' ? (
          /* Il bordo c'e' solo qui, e serve. Il bottone neutro ha il colore della
             superficie sollevata, che e' anche quella del pannello: nell'avviso non e'
             un problema, perche' l'accento sopra dice gia' che quelle due cose sono
             bottoni, ma due neutri affiancati e basta si leggerebbero come due scritte.
             Dare l'accento a una delle due direbbe che una avvisa di qualcosa, che e'
             proprio cio' che qui non vale. */
          <div className="mt-4 flex gap-2">
            <Button variant="quiet" className="border-hairline border" onClick={onConfirm}>
              {confirmLabel}
            </Button>
            <Button variant="quiet" className="border-hairline border" onClick={onCancel}>
              {cancelLabel}
            </Button>
          </div>
        ) : (
          <div className="mt-4 flex flex-col gap-2">
            <Button variant="danger" onClick={onConfirm}>
              {confirmLabel}
            </Button>
            <Button variant="quiet" autoFocus onClick={onCancel}>
              {cancelLabel}
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
