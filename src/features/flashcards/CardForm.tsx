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
 * nuova: dopo una correzione ci sara' da chiedere se azzerare o no quello che si e'
 * imparato, e quella e' una decisione della schermata, non della casella di testo.
 */
export function CardForm({
  card,
  onSave,
  onDelete,
  onClose,
}: {
  /** La carta da correggere, oppure `null` per scriverne una nuova. */
  card: Flashcard | null
  onSave: (japanese: string, meaning: string) => Promise<void>
  /** Come si butta via questa carta. Su una carta nuova non c'e' niente da buttare. */
  onDelete?: () => void
  onClose: () => void
}) {
  const [japanese, setJapanese] = useState(card?.japanese ?? '')
  const [meaning, setMeaning] = useState(card?.meaning ?? '')
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)

  // Le due facce servono tutte e due, e il core lo rifiuterebbe comunque. Spegnere il
  // bottone lo dice prima invece di far scoprire l'errore dopo aver premuto.
  const ready = japanese.trim() !== '' && meaning.trim() !== ''

  const save = () => {
    if (!ready || busy) return
    setBusy(true)
    setFailed(false)
    onSave(japanese, meaning)
      .catch(() => {
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
      <div className="flex flex-col gap-4 pt-2">
        <TextInput
          label="Japanese"
          value={japanese}
          onChange={setJapanese}
          placeholder="ねこ"
          japanese
          autoFocus={card === null}
        />
        <TextInput
          label="Meaning"
          value={meaning}
          onChange={setMeaning}
          placeholder="cat"
        />
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
