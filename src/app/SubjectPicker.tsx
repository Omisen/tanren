import { Card } from '@/shared/ui/Card'
import { Field } from '@/shared/ui/Field'
import { useUi, type Section } from '@/shared/store/ui'

/**
 * Come si passa da una sezione all'altra, **finche' non c'e' la barra in fondo**.
 *
 * Sta nella radice e non in una feature perche' e' l'unico pezzo di interfaccia che le
 * conosce tutte, e la regola di dipendenza vieta a una feature di nominarne un'altra:
 * ogni sezione lo riceve gia' fatto, come nodo.
 *
 * **E' provvisorio.** Le sezioni adesso sono quattro e si raggiungono da qui in tre,
 * il che e' proprio il difetto che la barra viene a togliere: una navigazione che vive
 * dentro il contenuto, cambia da schermata a schermata e non arriva dappertutto.
 */

const SECTIONS: { value: Section; label: string; caption: string; jp?: true }[] = [
  { value: 'kana', label: 'かな', caption: 'Kana', jp: true },
  { value: 'kanji', label: '漢字', caption: 'Kanji', jp: true },
  { value: 'flashcards', label: 'カード', caption: 'Flashcards', jp: true },
  { value: 'settings', label: '⚙', caption: 'Settings' },
]

export function SubjectPicker() {
  const { section, goToSection } = useUi()

  return (
    <Field label="Section">
      {/* Due per riga e non quattro: con quattro colonne «Flashcards» andrebbe a capo
          su un telefono stretto. E' comunque una disposizione provvisoria, e la barra
          in fondo la sostituira' del tutto. */}
      <div className="grid grid-cols-2 gap-2">
        {SECTIONS.map((s) => (
          <Card
            key={s.value}
            pressed={section === s.value}
            onClick={() => goToSection(s.value)}
          >
            <span className={`text-2xl ${s.jp ? 'font-jp' : ''}`} lang={s.jp ? 'ja' : undefined}>
              {s.label}
            </span>
            <span className="text-muted text-xs">{s.caption}</span>
          </Card>
        ))}
      </div>
    </Field>
  )
}
