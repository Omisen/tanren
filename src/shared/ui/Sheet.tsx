import { useEffect, useId, type ReactNode } from 'react'

import { Button } from './Button'

/**
 * Un pannello che sale dal basso e si puo' scorrere.
 *
 * Sale dal basso come la conferma, per la stessa ragione: e' dove arriva il pollice, ed
 * e' la zona in cui stanno le azioni di ogni schermata, quindi non sposta l'abitudine.
 * A differenza della conferma pero' **contiene qualcosa da leggere**, quindi occupa
 * quasi tutto lo schermo e il corpo scorre.
 *
 * Non e' `Confirm` con altri bottoni: quella pone una domanda e aspetta una scelta,
 * questo mostra e basta. Tenerli separati evita un componente con due mestieri.
 */
export function Sheet({
  title,
  onClose,
  children,
}: {
  title: string
  onClose: () => void
  children: ReactNode
}) {
  const titleId = useId()

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return (
    <div className="bg-scrim fixed inset-0 z-50 flex flex-col justify-end">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        // Alto quasi quanto lo schermo ma non tutto: la striscia di velo che resta
        // sopra dice che sotto c'e' ancora la schermata, e che questo si chiude.
        className="border-hairline bg-ink-soft flex max-h-[88vh] flex-col rounded-t-3xl border-t"
      >
        <header className="flex items-center justify-between gap-2 px-5 pt-4 pb-2">
          <h2 id={titleId} className="text-muted text-sm font-medium tracking-[0.2em] uppercase">
            {title}
          </h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-muted -mr-2 flex size-11 items-center justify-center text-xl active:opacity-60"
          >
            ✕
          </button>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-5 pb-4">{children}</div>

        <footer className="px-5 pt-2 pb-5">
          <Button variant="quiet" onClick={onClose}>
            Close
          </Button>
        </footer>
      </div>
    </div>
  )
}
