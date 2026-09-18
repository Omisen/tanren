import { useUi, type Section } from '@/shared/store/ui'

/**
 * La barra in fondo: le quattro sezioni, sempre le stesse e sempre li'.
 *
 * # Perche' in fondo
 *
 * Perche' il caso d'uso primario e' il telefono tenuto in una mano, e il pollice arriva
 * comodo al bordo inferiore e molto meno a quello superiore. E' la stessa ragione per
 * cui `Screen` ancora la fascia delle azioni in basso, e il motivo per cui questa barra
 * sostituisce un'icona che stava in alto a destra.
 *
 * # Perche' i simboli sono kana e kanji veri
 *
 * Perche' l'app un'icona per ogni sezione non ce l'ha, e non e' un caso: aggiungerne
 * una serie vorrebbe dire una dipendenza per quattro disegni. I tre sillabari si dicono
 * gia' da soli, con la scrittura che si sta studiando e col font che l'app imbarca;
 * l'unica sezione che non e' una materia e' anche l'unica che porta un simbolo.
 *
 * # Come si vede quale e' attiva
 *
 * Il testo pieno contro il testo secondario, che e' la gerarchia gia' in uso in tutta
 * l'app (sezione 4: la fanno dimensione e peso, non l'opacita'). **Niente colore di
 * categoria**: il colore qui dice di che tipo e' un item, e una sezione non e' un item;
 * darlo a tre tab su quattro vorrebbe poi dire inventarne un quarto per le impostazioni,
 * cioe' una categoria che non esiste.
 *
 * `aria-current` dice la stessa cosa a chi la schermata non la guarda.
 */

const TABS: { value: Section; glyph: string; label: string; jp?: true }[] = [
  { value: 'kana', glyph: 'かな', label: 'Kana', jp: true },
  { value: 'kanji', glyph: '漢字', label: 'Kanji', jp: true },
  { value: 'flashcards', glyph: 'カード', label: 'Flashcards', jp: true },
  { value: 'settings', glyph: '⚙', label: 'Settings' },
]

export function TabBar() {
  const section = useUi((s) => s.section)
  const goToSection = useUi((s) => s.goToSection)

  return (
    <nav
      aria-label="Sections"
      className="border-hairline bg-ink-soft flex shrink-0 border-t"
    >
      {TABS.map((tab) => {
        const active = section === tab.value

        return (
          <button
            key={tab.value}
            type="button"
            aria-current={active ? 'page' : undefined}
            onClick={() => goToSection(tab.value)}
            // Alta abbastanza da prendersi il pollice senza mirare: quattro bersagli in
            // fila sono stretti, e l'altezza e' l'unica misura che resta generosa.
            className={`flex min-h-16 flex-1 flex-col items-center justify-center gap-1 px-1 transition-opacity active:opacity-60 ${
              active ? 'text-paper' : 'text-muted'
            }`}
          >
            <span
              className={`text-lg leading-none ${tab.jp ? 'font-jp' : ''}`}
              lang={tab.jp ? 'ja' : undefined}
              aria-hidden="true"
            >
              {tab.glyph}
            </span>
            <span className={`text-[11px] leading-none ${active ? 'font-medium' : ''}`}>
              {tab.label}
            </span>
          </button>
        )
      })}
    </nav>
  )
}
