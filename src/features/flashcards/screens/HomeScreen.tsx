import { useCallback, useEffect, useState, type ReactNode } from 'react'

import {
  createFlashcardDeck,
  flashcardDecks,
  type Deck,
  type DeckSummary,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Note } from '@/shared/ui/Card'
import { Field } from '@/shared/ui/Field'
import { LogoMark } from '@/shared/ui/LogoMark'
import { Screen } from '@/shared/ui/Screen'
import { Sheet } from '@/shared/ui/Sheet'

import { TextInput } from '../TextInput'
import { DeckScreen } from './DeckScreen'

/**
 * La home delle flashcard: i mazzi che ci sono, e come se ne fa un altro.
 *
 * # Perche' il mazzo aperto non sta nello store
 *
 * Perche' e' uno sguardo dentro la materia e non una scelta di materia: chiudendo
 * l'app va perso e va bene cosi', e nessun'altra schermata deve saperlo. E' la stessa
 * ragione per cui il livello che si consulta vive nella vista dei livelli. Lo store
 * tiene quale materia si sta guardando, non dove si e' arrivati a guardare dentro.
 *
 * # Perche' l'elenco si ricarica tornando indietro
 *
 * Perche' dentro un mazzo si aggiungono e si tolgono carte, quindi il conteggio che si
 * era letto entrando non vale piu'. Ricaricare al ritorno costa una query e dice la
 * verita'; tenere un conteggio aggiornato a mano vorrebbe dire avere due copie dello
 * stesso numero.
 */
export function FlashcardsHomeScreen({
  sections,
}: {
  sections: ReactNode
}) {
  const [open, setOpen] = useState<Deck | null>(null)
  // `null` vuol dire «non ancora arrivati», che non e' la stessa cosa di «nessuno».
  const [decks, setDecks] = useState<DeckSummary[] | null>(null)
  const [failed, setFailed] = useState(false)
  const [creating, setCreating] = useState(false)

  const load = useCallback(() => {
    flashcardDecks()
      .then(setDecks)
      .catch(() => setFailed(true))
  }, [])

  // Si ricarica anche quando si chiude un mazzo, perche' li' dentro le carte possono
  // essere cambiate.
  useEffect(() => {
    if (!open) load()
  }, [load, open])

  if (open) return <DeckScreen deck={open} onBack={() => setOpen(null)} />

  return (
    <>
      <Screen
        title="Tanren"
        mark={<LogoMark />}
        textured
        action={<Button onClick={() => setCreating(true)}>New deck</Button>}
      >
        <div className="flex flex-col gap-6 pt-2">
          {sections}

          <Field label="Decks">
            <div className="flex flex-col gap-2">
              {failed && <Note>Could not read your decks.</Note>}
              {!failed && decks === null && <Note>Loading…</Note>}
              {decks?.length === 0 && (
                <Note>No decks yet. Make one and fill it with your own words.</Note>
              )}

              {decks?.map((deck) => (
                <button
                  key={deck.id}
                  type="button"
                  onClick={() => setOpen({ id: deck.id, name: deck.name })}
                  className="border-hairline bg-ink-soft flex min-h-16 items-center justify-between gap-3 rounded-xl border px-4 py-3 text-left active:opacity-70"
                >
                  <span className="text-base">{deck.name}</span>
                  <span className="text-muted shrink-0 text-sm">
                    {deck.cards === 1 ? '1 card' : `${deck.cards} cards`}
                  </span>
                </button>
              ))}
            </div>
          </Field>
        </div>
      </Screen>

      {creating && (
        <NewDeckForm
          onSave={async (name) => {
            const deck = await createFlashcardDeck(name)
            setCreating(false)
            // Si entra subito nel mazzo appena fatto: un mazzo vuoto esiste per
            // riempirlo, e chiederlo con un secondo tocco sarebbe un tocco in piu'
            // per una cosa gia' decisa.
            setOpen(deck)
          }}
          onClose={() => setCreating(false)}
        />
      )}
    </>
  )
}

/** Il pannello con cui si crea un mazzo. Serve solo il nome. */
function NewDeckForm({
  onSave,
  onClose,
}: {
  onSave: (name: string) => Promise<void>
  onClose: () => void
}) {
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const ready = name.trim() !== ''

  const save = () => {
    if (!ready || busy) return
    setBusy(true)
    setFailed(false)
    onSave(name).catch(() => {
      setFailed(true)
      setBusy(false)
    })
  }

  return (
    <Sheet
      title="New deck"
      onClose={onClose}
      action={
        <Button onClick={save} disabled={!ready || busy}>
          Create
        </Button>
      }
    >
      <div className="flex flex-col gap-4 pt-2">
        <TextInput
          label="Name"
          value={name}
          onChange={setName}
          placeholder="N5"
          autoFocus
        />
        {failed && <p className="text-accent text-sm">Could not save. Try again.</p>}
      </div>
    </Sheet>
  )
}
