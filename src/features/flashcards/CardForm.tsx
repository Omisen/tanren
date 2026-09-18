import { useState } from 'react'

import type { Flashcard } from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Sheet } from '@/shared/ui/Sheet'

import { TextInput } from './TextInput'

/**
 * Il pannello con cui si scrive una carta, nuova o gia' esistente.
 *
 * E' **uno solo per i due casi** e non due quasi uguali: quello che si compila per
 * creare una carta e quello che si compila per correggerla sono la stessa cosa, e due
 * copie divergerebbero come sono gia' divergiti `Chip` e `Card`. Cambia il titolo e
 * cosa c'e' dentro le caselle all'apertura.
 *
 * # Chi salva non e' questo componente
 *
 * Il modulo raccoglie il testo e chiama `onSave`; chi lo apre decide cosa vuol dire
 * salvare. Serve perche' salvare una **correzione** non e' come salvare una carta
 * nuova: dopo una correzione c'e' da chiedere se azzerare o no quello che si e'
 * imparato, e quella e' una decisione della schermata, non della casella di testo.
 *
 * # Le risposte in piu' non stanno tutte insieme
 *
 * Il **furigana** sta sotto il giapponese e gli **altri significati** sotto il
 * significato, perche' servono a due domande diverse: il furigana vale quando si
 * risponde in giapponese, gli altri significati quando si risponde col significato.
 * Metterli in un unico elenco di «risposte accettate» inviterebbe a scriverci dentro
 * l'una al posto dell'altra.
 */

/**
 * Se un testo contiene almeno un kanji.
 *
 * Serve solo a decidere **se offrire** la casella del furigana: una parola scritta in
 * soli kana la lettura ce l'ha gia' addosso, e chiederla sarebbe far ricopiare quello
 * che si e' appena scritto. Il campo pero' non sparisce mai se un valore c'e' gia',
 * altrimenti una carta riscritta senza kanji si porterebbe dietro un furigana che non
 * si puo' piu' ne' vedere ne' togliere.
 */
const KANJI = /[々㐀-䶿一-鿿]/u

export function CardForm({
  card,
  max,
  onSave,
  onDelete,
  onClose,
}: {
  /** La carta da correggere, oppure `null` per scriverne una nuova. */
  card: Flashcard | null
  /** Quanti significati si accettano oltre al principale. Lo dice il core. */
  max: number
  onSave: (
    japanese: string,
    meaning: string,
    alternatives: string[],
    furigana: string,
  ) => Promise<void>
  /** Come si butta via questa carta. Su una carta nuova non c'e' niente da buttare. */
  onDelete?: () => void
  onClose: () => void
}) {
  const [japanese, setJapanese] = useState(card?.japanese ?? '')
  const [furigana, setFurigana] = useState(card?.furigana ?? '')
  const [meaning, setMeaning] = useState(card?.meaning ?? '')
  const [alternatives, setAlternatives] = useState<string[]>(card?.alternatives ?? [])
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)

  // Le due facce servono tutte e due, e il core le rifiuterebbe comunque. Spegnere il
  // bottone lo dice prima invece di far scoprire l'errore dopo aver premuto.
  const ready = japanese.trim() !== '' && meaning.trim() !== ''

  const save = () => {
    if (!ready || busy) return
    setBusy(true)
    setFailed(false)
    onSave(japanese, meaning, alternatives, furigana).catch(() => {
      setFailed(true)
      setBusy(false)
    })
  }

  return (
    <Sheet
      title={card ? 'Edit card' : 'New card'}
      onClose={onClose}
      action={
        <Button onClick={save} disabled={!ready || busy}>
          Save
        </Button>
      }
    >
      <div className="flex flex-col gap-5 pt-2">
        <div className="flex flex-col gap-3">
          <TextInput
            label="Japanese"
            value={japanese}
            onChange={setJapanese}
            placeholder="日本語"
            japanese
            autoFocus={card === null}
          />

          {(KANJI.test(japanese) || furigana !== '') && (
            <TextInput
              label="Furigana"
              value={furigana}
              onChange={setFurigana}
              placeholder="にほんご"
              japanese
            />
          )}
        </div>

        <div className="flex flex-col gap-3">
          <TextInput
            label="Meaning"
            value={meaning}
            onChange={setMeaning}
            placeholder="japanese"
          />

          {alternatives.map((value, i) => (
            <TextInput
              key={i}
              value={value}
              onChange={(next) =>
                setAlternatives((list) => list.map((v, j) => (j === i ? next : v)))
              }
              onRemove={() =>
                setAlternatives((list) => list.filter((_, j) => j !== i))
              }
              placeholder="another accepted answer"
            />
          ))}

          {alternatives.length < max && (
            <button
              type="button"
              onClick={() => setAlternatives((list) => [...list, ''])}
              className="text-muted flex min-h-11 items-center gap-2 text-left text-sm active:opacity-60"
            >
              <span aria-hidden="true" className="text-base">
                +
              </span>
              add more answer
            </button>
          )}
        </div>

        {failed && <p className="text-accent text-sm">Could not save. Try again.</p>}

        {onDelete && (
          <button
            type="button"
            onClick={onDelete}
            className="text-accent min-h-11 text-sm active:opacity-60"
          >
            Delete card
          </button>
        )}
      </div>
    </Sheet>
  )
}
