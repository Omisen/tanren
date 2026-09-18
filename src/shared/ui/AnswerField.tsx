import { useCallback, useEffect, useRef, useState } from 'react'

import { Button } from '@/shared/ui/Button'

/**
 * Il campo con cui si risponde scrivendo, usando l'IME del dispositivo.
 *
 * Non c'e' nessuna tastiera custom in-app: si scrive con l'IME vero, quello che si
 * userebbe per scrivere un messaggio. E' una scelta presa all'inizio del progetto e
 * questo componente e' il primo posto in cui si vede davvero.
 *
 * # I due problemi dell'IME
 *
 * **Invio mentre si converte non e' una risposta.** Digitando `ka` l'IME propone
 * `か`, e il primo Invio serve a confermare quella conversione. Se lo prendessimo per
 * una risposta, mandare `k` a meta' conversione diventerebbe un errore. Da qui la
 * doppia guardia: `isComposing` sull'evento, e lo stato di composizione tenuto a mano,
 * perche' i vari IME non si comportano tutti allo stesso modo.
 *
 * **Quello che si vede non e' sempre quello che verra' giudicato.** Un IME puo'
 * restituire katakana a mezza larghezza, o segni di sonorizzazione staccati. Sotto al
 * campo si mostra la forma normalizzata, ma solo quando differisce da quella scritta:
 * altrimenti sarebbe una riga che ripete il campo e non insegna niente. La
 * normalizzazione la fa il core, non questo file.
 *
 * # Perche' `normalize` arriva da fuori
 *
 * Perche' non ce n'e' una sola. Sui kana conta la **grafia**, quindi si usa la pulizia
 * che non ripiega sull'hiragana: rispondere か a una domanda su カ e' sbagliato. Dove
 * conta la **lettura**, come sara' per i kanji, ne serve un'altra. Il campo mostra
 * quello che la materia gli dice, e non sceglie al posto suo.
 */
export function AnswerField({
  disabled,
  given,
  placeholder,
  normalize,
  onSubmit,
  script = 'japanese',
  phrase = false,
}: {
  disabled: boolean
  /** La risposta gia' data, se si sta guardando l'esito. */
  given: string | null
  placeholder: string
  /** Come il core ridurra' la risposta prima di giudicarla. */
  normalize: (input: string) => Promise<string>
  onSubmit: (value: string) => void
  /**
   * In che alfabeto si risponde.
   *
   * Decide `lang` e il font. Su `latin` il campo **non** dichiara `ja` e non usa il
   * font giapponese: sarebbe una bugia sul contenuto, e chi usa la lettura assistita
   * si sentirebbe leggere l'inglese con la voce sbagliata.
   */
  script?: 'japanese' | 'latin'
  /**
   * Se la risposta e' una **frase** e non un segno.
   *
   * Il corpo grande e centrato nasce per i kana e i kanji, dove si risponde con uno o
   * due segni e la leggibilita' dei tratti e' tutto. Su una frase e' il contrario:
   * misurato su una finestra piu' larga di un telefono, «the library is closed today»
   * si taglia a «toda», e non poter rileggere quello che si e' scritto e' un difetto,
   * non una scelta estetica.
   */
  phrase?: boolean
}) {
  const [value, setValue] = useState('')
  // L'anteprima si porta dietro il testo da cui e' stata ricavata. Cosi' quando il
  // testo cambia il risultato vecchio smette di valere da solo, senza doverlo
  // azzerare a mano e far ripartire un altro render.
  const [preview, setPreview] = useState<{ source: string; text: string } | null>(null)

  // Alcuni IME non riempiono `isComposing` su tutti gli eventi: questa e' la rete di
  // sicurezza.
  const composing = useRef(false)

  // L'anteprima arriva dal core, quindi puo' tornare fuori ordine: vale solo l'ultima
  // richiesta partita.
  const run = useRef(0)

  const shown = given ?? value

  useEffect(() => {
    if (!shown) return

    const token = (run.current += 1)
    normalize(shown)
      .then((text) => {
        if (run.current === token) setPreview({ source: shown, text })
      })
      .catch(() => {
        // Senza anteprima si risponde lo stesso: e' un aiuto, non un requisito.
      })
  }, [shown, normalize])

  const normalized = preview?.source === shown ? preview.text : ''

  const submit = useCallback(() => {
    const answer = shown.trim()
    if (disabled || answer === '') return
    onSubmit(answer)
  }, [disabled, shown, onSubmit])

  return (
    <div className="flex flex-col gap-2">
      <input
        type="text"
        lang={script === 'japanese' ? 'ja' : undefined}
        value={shown}
        disabled={disabled}
        autoFocus
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
        aria-label="Your answer"
        placeholder={placeholder}
        onChange={(e) => setValue(e.target.value)}
        onCompositionStart={() => {
          composing.current = true
        }}
        onCompositionEnd={() => {
          composing.current = false
        }}
        onKeyDown={(e) => {
          if (e.key !== 'Enter') return
          // Qui Invio sta confermando la conversione dell'IME, non rispondendo.
          if (composing.current || e.nativeEvent.isComposing) return
          e.preventDefault()
          submit()
        }}
        className={`border-hairline bg-ink-soft text-paper placeholder:text-inactive disabled:text-muted focus:border-focus min-h-14 w-full rounded-xl border px-4 outline-none ${
          script === 'japanese' ? 'font-jp' : ''
        } ${phrase ? 'text-left text-base' : 'text-center text-3xl'}`}
      />

      {/* Lo spazio dell'anteprima e' sempre occupato, cosi' il campo non si sposta
          quando l'IME produce qualcosa da ripulire. */}
      <p className={`text-muted min-h-5 text-xs ${phrase ? 'text-left' : 'text-center'}`}>
        {normalized && normalized !== shown && (
          <>
            counts as{' '}
            <span className="font-jp text-muted" lang="ja">
              {normalized}
            </span>
          </>
        )}
      </p>

      {!given && (
        <Button disabled={disabled || shown.trim() === ''} onClick={submit}>
          Check
        </Button>
      )}
    </div>
  )
}
