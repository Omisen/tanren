import { useCallback, useEffect, useState } from 'react'

import {
  createFlashcard,
  deleteFlashcard,
  deleteFlashcardDeck,
  flashcardCards,
  renameFlashcardDeck,
  updateFlashcard,
  type Deck,
  type Flashcard,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Note } from '@/shared/ui/Card'
import { Confirm } from '@/shared/ui/Confirm'
import { Screen } from '@/shared/ui/Screen'
import { Sheet } from '@/shared/ui/Sheet'

import { CardForm } from '../CardForm'
import { TextInput } from '../TextInput'

/**
 * Dentro un mazzo: le sue carte, e come si correggono.
 *
 * # Le azioni del mazzo non stanno nella fascia in fondo
 *
 * Quella e' della cosa che si fa ogni volta che si entra, cioe' aggiungere una carta.
 * Rinominare ed eliminare si fanno una volta nella vita del mazzo, quindi stanno
 * dietro il ⋯ in cima, che e' la via secondaria che `Screen` prevede.
 */
export function DeckScreen({ deck, onBack }: { deck: Deck; onBack: () => void }) {
  // Il nome puo' cambiare da qui dentro, quindi non basta quello con cui si e' entrati.
  const [name, setName] = useState(deck.name)
  // `null` vuol dire «non ancora arrivate», che non e' la stessa cosa di «nessuna».
  const [cards, setCards] = useState<Flashcard[] | null>(null)
  const [failed, setFailed] = useState(false)

  // Un pannello alla volta: sono modali, e due aperti insieme non avrebbero senso.
  const [editing, setEditing] = useState<Flashcard | 'new' | null>(null)
  const [options, setOptions] = useState(false)
  const [renaming, setRenaming] = useState(false)
  const [removing, setRemoving] = useState<Flashcard | 'deck' | null>(null)

  const load = useCallback(() => {
    flashcardCards(deck.id)
      .then(setCards)
      .catch(() => setFailed(true))
  }, [deck.id])

  useEffect(load, [load])

  const save = async (japanese: string, meaning: string) => {
    if (editing === 'new') await createFlashcard(deck.id, japanese, meaning)
    else if (editing) await updateFlashcard(editing.id, japanese, meaning)
    setEditing(null)
    load()
  }

  const remove = async () => {
    if (removing === 'deck') {
      await deleteFlashcardDeck(deck.id)
      onBack()
      return
    }
    if (removing) await deleteFlashcard(removing.id)
    setRemoving(null)
    load()
  }

  return (
    <>
      <Screen
        title={name}
        onBack={onBack}
        trailing={
          <button
            type="button"
            onClick={() => setOptions(true)}
            aria-label="Deck options"
            className="text-muted flex size-11 items-center justify-center text-xl active:opacity-60"
          >
            ⋯
          </button>
        }
        action={<Button onClick={() => setEditing('new')}>Add card</Button>}
      >
        <div className="flex flex-col gap-2 pt-2">
          {failed && <Note>Could not read this deck.</Note>}
          {!failed && cards === null && <Note>Loading…</Note>}
          {cards?.length === 0 && <Note>No cards yet. Add the first one.</Note>}

          {cards?.map((card) => (
            <button
              key={card.id}
              type="button"
              onClick={() => setEditing(card)}
              className="border-hairline bg-ink-soft flex min-h-16 flex-col items-start justify-center gap-0.5 rounded-xl border px-4 py-3 text-left active:opacity-70"
            >
              <span className="font-jp text-xl" lang="ja">
                {card.japanese}
              </span>
              <span className="text-muted text-sm">{card.meaning}</span>
            </button>
          ))}
        </div>
      </Screen>

      {editing && (
        <CardForm
          card={editing === 'new' ? null : editing}
          onSave={save}
          onDelete={
            editing === 'new'
              ? undefined
              : () => {
                  const card = editing
                  setEditing(null)
                  setRemoving(card)
                }
          }
          onClose={() => setEditing(null)}
        />
      )}

      {options && (
        <Sheet title="Deck" onClose={() => setOptions(false)}>
          <div className="flex flex-col gap-2 pt-2">
            <Button
              variant="quiet"
              onClick={() => {
                setOptions(false)
                setRenaming(true)
              }}
            >
              Rename deck
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                setOptions(false)
                setRemoving('deck')
              }}
            >
              Delete deck
            </Button>
          </div>
        </Sheet>
      )}

      {renaming && (
        <RenameForm
          name={name}
          onSave={async (value) => {
            await renameFlashcardDeck(deck.id, value)
            setName(value.trim())
            setRenaming(false)
          }}
          onClose={() => setRenaming(false)}
        />
      )}

      {removing && (
        <Confirm
          title={removing === 'deck' ? `Delete ${name}?` : 'Delete this card?'}
          confirmLabel="Delete"
          cancelLabel="Keep it"
          onConfirm={() => void remove()}
          onCancel={() => setRemoving(null)}
        >
          {removing === 'deck'
            ? 'The deck and all its cards go away, and so does what you have learned of them.'
            : 'The card goes away, and so does what you have learned of it.'}
        </Confirm>
      )}
    </>
  )
}

/** Il pannello con cui si cambia nome a un mazzo. */
function RenameForm({
  name,
  onSave,
  onClose,
}: {
  name: string
  onSave: (name: string) => Promise<void>
  onClose: () => void
}) {
  const [value, setValue] = useState(name)
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const ready = value.trim() !== ''

  const save = () => {
    if (!ready || busy) return
    setBusy(true)
    setFailed(false)
    onSave(value).catch(() => {
      setFailed(true)
      setBusy(false)
    })
  }

  return (
    <Sheet
      title="Rename deck"
      onClose={onClose}
      action={
        <Button onClick={save} disabled={!ready || busy}>
          Save
        </Button>
      }
    >
      <div className="flex flex-col gap-4 pt-2">
        <TextInput
          label="Name"
          value={value}
          onChange={setValue}
          placeholder="N5"
          autoFocus
        />
        {failed && <p className="text-accent text-sm">Could not save. Try again.</p>}
      </div>
    </Sheet>
  )
}
