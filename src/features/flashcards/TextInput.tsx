import { useId } from 'react'

/**
 * Una casella di testo con la sua etichetta.
 *
 * Sta nella feature e non in `shared/ui` perche' per ora la usa solo lei, e il progetto
 * generalizza quando i casi sono due, non prima. Quando una seconda materia avra' da
 * far scrivere qualcosa, questa sale di livello.
 *
 * `japanese` non e' una scelta estetica: porta il font imbarcato e l'attributo `lang`,
 * senza i quali il testo cadrebbe sul ripiego di sistema, cioe' forme diverse fra
 * telefono e desktop e rettangoli vuoti su un Linux senza font CJK. E' la stessa
 * ragione per cui il font e' imbarcato.
 */
export function TextInput({
  label,
  value,
  onChange,
  placeholder,
  japanese = false,
  autoFocus = false,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  placeholder: string
  /** Se ci si scrive giapponese. */
  japanese?: boolean
  autoFocus?: boolean
}) {
  const id = useId()

  return (
    <div className="flex flex-col gap-2">
      <label
        htmlFor={id}
        className="text-muted text-xs font-medium tracking-[0.2em] uppercase"
      >
        {label}
      </label>
      <input
        id={id}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        autoFocus={autoFocus}
        lang={japanese ? 'ja' : undefined}
        className={`border-hairline bg-ink text-paper placeholder:text-inactive focus:border-focus min-h-12 w-full rounded-xl border px-4 outline-none ${
          japanese ? 'font-jp text-xl' : 'text-base'
        }`}
      />
    </div>
  )
}
