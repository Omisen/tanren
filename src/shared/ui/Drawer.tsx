import { useCallback, useEffect, useId, useState, type ReactNode } from 'react'

/**
 * Una tendina che entra da destra e copre quasi tutta la larghezza.
 *
 * # Perche' da destra e non dal basso come gli altri due pannelli
 *
 * `Confirm` e `Sheet` salgono dal basso perche' sono la risposta a qualcosa che si sta
 * facendo li' in quel momento: una domanda da confermare, una scheda da guardare. La
 * fascia in fondo e' la zona delle azioni, e loro ci appartengono.
 *
 * Le impostazioni no: non sono un passo dentro quello che si sta facendo, sono un
 * altro posto in cui si va e da cui si torna. Entrare di lato lo dice, ed e' anche il
 * gesto che il pollice si aspetta da un menu, visto che il richiamo sta in alto a
 * destra e la tendina arriva da li'.
 *
 * # Entra scorrendo, e esce allo stesso modo
 *
 * Comparire di colpo sopra la schermata non dice da dove si viene ne' dove si torna: il
 * movimento da destra e' l'informazione, ed e' la ragione per cui la tendina sta a
 * destra e non altrove. L'uscita e' la stessa animazione **al contrario**, cosi' il
 * movimento e' descritto una volta sola e le due meta' non possono sfasarsi; e' pero'
 * un filo piu' rapida, perche' chi chiude ha gia' deciso e non vuole aspettare.
 *
 * **Chiudere passa da qui e non da chi ci sta sopra.** Se la richiesta arrivasse
 * direttamente a `onClose`, il pannello verrebbe smontato subito e l'uscita non
 * avrebbe niente da animare: la tendina incassa la richiesta, si toglie di scena e
 * **poi** avvisa. Vale per tutte e tre le vie d'uscita, la ✕, il velo e Esc.
 *
 * # Perche' quasi tutta la larghezza
 *
 * Su un telefono in verticale una tendina stretta lascerebbe righe di due parole. La
 * striscia di velo che resta a sinistra basta a dire che sotto c'e' ancora la
 * schermata e che questa si chiude, che e' lo stesso patto di `Sheet`.
 */
export function Drawer({
  title,
  onClose,
  children,
}: {
  title: string
  onClose: () => void
  children: ReactNode
}) {
  const titleId = useId()
  /**
   * Se la tendina sta uscendo di scena.
   *
   * Serve perche' chiudere non e' istantaneo: chi ci sta sopra ci smonterebbe subito,
   * e l'animazione d'uscita non avrebbe niente da animare. Quindi la richiesta di
   * chiusura la incassa la tendina, che si toglie da sola e **poi** avvisa.
   */
  const [uscita, setUscita] = useState(false)

  const chiudi = useCallback(() => setUscita(true), [])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') chiudi()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [chiudi])

  return (
    <div className={`bg-scrim fixed inset-0 z-50 flex justify-end ${uscita ? 'scrim-out' : 'scrim-in'}`}>
      {/* Il velo si tocca per chiudere, come ci si aspetta da una tendina. Non e' la
          sola via: c'e' la ✕ e c'e' Esc, perche' un bersaglio invisibile non puo'
          essere l'unico modo di uscire. */}
      <button
        type="button"
        aria-label="Close"
        tabIndex={-1}
        onClick={chiudi}
        className="flex-1 cursor-default"
      />

      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className={`border-hairline bg-ink-soft flex w-[88%] max-w-sm flex-col border-l ${
          uscita ? 'drawer-out' : 'drawer-in'
        }`}
        // Solo la fine dell'animazione **di questo** elemento conta: quelle dei figli
        // gorgogliano fin qui e chiuderebbero la tendina mentre sta ancora entrando.
        onAnimationEnd={(event) => {
          if (uscita && event.target === event.currentTarget) onClose()
        }}
      >
        <header className="flex items-center justify-between gap-2 px-5 pt-4 pb-2">
          <h2 id={titleId} className="text-muted text-sm font-medium tracking-[0.2em] uppercase">
            {title}
          </h2>
          <button
            type="button"
            onClick={chiudi}
            aria-label="Close"
            className="text-muted -mr-2 flex size-11 items-center justify-center text-xl active:opacity-60"
          >
            ✕
          </button>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto px-5 pt-2 pb-5">{children}</div>
      </div>
    </div>
  )
}
