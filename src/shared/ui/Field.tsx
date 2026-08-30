import type { ReactNode } from 'react'

/**
 * Una sezione della schermata di scelta: un'etichetta e quello che si sceglie.
 *
 * L'etichetta e' piccola, in peso medio e spaziata: e' la stessa grammatica del titolo
 * di schermata, un gradino piu' in basso. La gerarchia la fanno dimensione e peso, non
 * l'opacita' (sezione 4).
 */
export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">
        {label}
      </h2>
      {children}
    </section>
  )
}
