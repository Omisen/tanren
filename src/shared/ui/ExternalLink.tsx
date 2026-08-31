import type { ReactNode } from 'react'

import { openExternal } from '@/shared/bridge'

/**
 * Un link che porta **fuori** dall'app.
 *
 * # Perche' non basta un'ancora normale
 *
 * Perche' dentro una WebView un'ancora non fa quello che fa in un browser. Verificato
 * sul dispositivo: wry non abilita le finestre multiple su Android, quindi
 * `target="_blank"` non apre una seconda pagina, e Tauri senza un `on_navigation`
 * nostro autorizza la navigazione. Il risultato era che la WebView **andava sul posto**
 * al sito: l'app diventava un browser, col solo tasto indietro per uscirne, e sembrava
 * rotta.
 *
 * Qui il clic viene fermato e l'indirizzo passato al sistema, che lo apre dove va
 * aperto. **Resta un'ancora** e non diventa un bottone perche' un link deve leggersi
 * come un link: si copia, si tiene premuto, e chi usa la lettura assistita sente che
 * porta fuori.
 *
 * Non e' una raffinatezza: quasi tutti i link dell'app sono **attribuzione
 * obbligatoria**, e una licenza che si raggiunge solo rompendo l'app non e'
 * raggiungibile.
 */
export function ExternalLink({
  href,
  className,
  children,
}: {
  href: string
  className?: string
  children: ReactNode
}) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      onClick={(event) => {
        event.preventDefault()
        void openExternal(href)
      }}
      className={className}
    >
      {children}
    </a>
  )
}
