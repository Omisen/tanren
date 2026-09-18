import { useCallback, useEffect, useState } from 'react'

import {
  createFlashcard,
  deleteFlashcard,
  deleteFlashcardDeck,
  flashcardAvailability,
  flashcardCards,
  renameFlashcardDeck,
  updateFlashcard,
  type Deck,
  type Flashcard,
  type FlashcardAvailability,
  type FlashcardDirection,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Note } from '@/shared/ui/Card'
import { Chip } from '@/shared/ui/Chip'
import { Confirm } from '@/shared/ui/Confirm'
import { Screen } from '@/shared/ui/Screen'
import { Sheet } from '@/shared/ui/Sheet'

import { CardForm } from '../CardForm'
import { TextInput } from '../TextInput'
import { FlashcardSessionScreen } from './SessionScreen'

/**
 * I due versi in cui si puo' studiare un mazzo.
 *
 * L'etichetta dice **cosa si vede per primo**, non cosa si deve fare: e' la cosa che
 * si riconosce a colpo d'occhio guardando la schermata di studio, e la riga sopra lo
 * stimolo dira' comunque cosa si vuole.
 */
const DIRECTIONS: { value: FlashcardDirection; label: string }[] = [
  { value: 'jp_to_meaning', label: 'Japanese first' },
  { value: 'meaning_to_jp', label: 'Meaning first' },
]

/**
 * Dentro un mazzo: le sue carte, e come si studia.
 *
 * # Cosa sta nella fascia in fondo, e perche' e' cambiato
 *
 * Adesso ci stanno il verso e il tasto di avvio. Prima ci stava «Add card», e andava
 * bene finche' un mazzo si poteva solo riempire: ora la cosa che si fa ogni volta che
 * si entra e' studiarlo, e quella e' la fascia di chi si fa trovare dal pollice.
 * Aggiungere una carta scende in fondo all'elenco, dove si guarda quando si sta
 * costruendo il mazzo; rinominare ed eliminare restano dietro il ⋯ in cima, perche' si
 * fanno una volta nella vita di un mazzo.
 *
 * # Il verso si sceglie qui e non dentro il giro
 *
 * Perche' decide cosa sara' l'intera sessione, e cambiarlo a meta' vorrebbe dire
 * cambiare esercizio mentre lo si sta facendo. E' la stessa collocazione che ha la
 * scelta fra riconoscimento e scrittura sui kana.
 */
export function DeckScreen({ deck, onBack }: { deck: Deck; onBack: () => void }) {
  // Il nome puo' cambiare da qui dentro, quindi non basta quello con cui si e' entrati.
  const [name, setName] = useState(deck.name)
  // `null` vuol dire «non ancora arrivate», che non e' la stessa cosa di «nessuna».
  const [cards, setCards] = useState<Flashcard[] | null>(null)
  // Cosa si troverebbe partendo adesso. Cambia col verso, perche' le due direzioni
  // sono due carte di studio con due scadenze.
  const [available, setAvailable] = useState<FlashcardAvailability | null>(null)
  const [failed, setFailed] = useState(false)

  // Un pannello alla volta: sono modali, e due aperti insieme non avrebbero senso.
  const [editing, setEditing] = useState<Flashcard | 'new' | null>(null)
  const [options, setOptions] = useState(false)
  const [renaming, setRenaming] = useState(false)
  const [removing, setRemoving] = useState<Flashcard | 'deck' | null>(null)

  // Il verso e la sessione vivono qui, come il mazzo aperto: sono scelte che muoiono
  // uscendo, e nessun'altra schermata deve conoscerle.
  const [direction, setDirection] = useState<FlashcardDirection>('jp_to_meaning')
  const [studying, setStudying] = useState(false)

  const load = useCallback(() => {
    flashcardCards(deck.id)
      .then(setCards)
      .catch(() => setFailed(true))
    flashcardAvailability({ deck: deck.id, direction })
      .then(setAvailable)
      .catch(() => setFailed(true))
  }, [deck.id, direction])

  // Si ricarica anche tornando da un giro, perche' li' dentro le scadenze si sono
  // mosse e quello che si e' letto entrando non vale piu'.
  useEffect(() => {
    if (!studying) load()
  }, [load, studying])

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

  if (studying) {
    return (
      <FlashcardSessionScreen
        deck={name}
        scope={{ deck: deck.id, direction }}
        onBack={() => setStudying(false)}
      />
    )
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
        action={
          <div className="flex flex-col gap-2">
            <div className="flex gap-2">
              {DIRECTIONS.map((d) => (
                <Chip
                  key={d.value}
                  pressed={direction === d.value}
                  onClick={() => setDirection(d.value)}
                >
                  {d.label}
                </Chip>
              ))}
            </div>
            <Button
              onClick={() => setStudying(true)}
              disabled={!available || available.total === 0}
            >
              {start(available)}
            </Button>
          </div>
        }
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

          {cards && (
            <Button variant="quiet" className="mt-2" onClick={() => setEditing('new')}>
              Add card
            </Button>
          )}
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

/**
 * Cosa dice il tasto di avvio.
 *
 * **Ripassare tre carte e rifare tutto il mazzo sono due cose diverse**, e chi studia
 * ha diritto di saperlo prima di premere, non dopo. Quale delle due sia lo decide il
 * core guardando cosa e' dovuto: qui si legge e basta.
 */
function start(available: FlashcardAvailability | null): string {
  if (!available || available.total === 0) return 'Start'

  const quante = available.due > 0 ? available.due : available.total
  const carte = quante === 1 ? 'card' : 'cards'
  return available.due > 0 ? `Review ${quante} ${carte}` : `Practice ${quante} ${carte}`
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
