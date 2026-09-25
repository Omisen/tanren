import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { useState } from 'react'

import {
  checkFlashcardImport,
  flashcardImportTemplate,
  importFlashcards,
  type ImportProblem,
  type ImportRowError,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Note } from '@/shared/ui/Card'
import { Sheet } from '@/shared/ui/Sheet'

/**
 * Il pannello con cui si riempie un mazzo da un CSV.
 *
 * Fa due cose: da' il file da compilare, e ne rimanda indietro uno pieno. Sono le due
 * meta' della stessa operazione e stanno insieme, perche' chi importa la prima volta
 * scarica il modello e chi importa la seconda ce l'ha gia'.
 *
 * # Perche' un `Sheet` e non un wizard vero
 *
 * Perche' una primitiva a piu' passi non esiste, e non la si inventa per un caso solo.
 * I passi qui sono comunque due e non tre, scegli e conferma, che in un pannello solo
 * stanno senza stringere.
 *
 * # Il file si legge a byte, mai come testo
 *
 * `readTextFile` non fallisce su un file che non e' UTF-8: restituisce mojibake dove
 * c'e' il giapponese e testo giusto dove c'e' il latino, quindi un import andrebbe a
 * buon fine e le carte avrebbero il lato giapponese distrutto. Misurato sul dispositivo
 * con un CSV in Shift-JIS. Il decodificatore severo invece **lancia**, ed e' l'unico
 * modo di sapere che quel file non si puo' usare.
 */

/** I messaggi che non vengono dal core, cioe' quelli sul file e non sulle righe. */
const MESSAGES = {
  encoding:
    'This file is not saved in UTF-8, so the Japanese in it cannot be read. Save it as UTF-8 and try again.',
  unreadable: 'This file could not be read.',
  unsaved: 'The template could not be saved.',
} as const

/** Quante righe sbagliate si elencano prima di dire solo quante sono. */
const SHOWN = 8

type Stage =
  | { at: 'idle' }
  | { at: 'busy' }
  /** Il file va bene: si tiene il testo, perche' e' quello che si importera'. */
  | { at: 'ready'; csv: string; cards: number }
  | { at: 'rejected'; errors: ImportRowError[] }
  | { at: 'failed'; message: string }
  | { at: 'done'; cards: number }

export function ImportSheet({
  deck,
  name,
  onImported,
  onClose,
}: {
  deck: string
  /** Il nome del mazzo, che finisce nel nome del file del modello. */
  name: string
  /** Si sono aggiunte delle carte: chi sta sopra deve rileggere il mazzo. */
  onImported: () => void
  onClose: () => void
}) {
  const [stage, setStage] = useState<Stage>({ at: 'idle' })

  const template = async () => {
    setStage({ at: 'busy' })
    const csv = await flashcardImportTemplate()

    const path = await pick(() =>
      save({ defaultPath: filename(name), filters: [CSV] }),
    )
    if (!path) {
      setStage({ at: 'idle' })
      return
    }

    try {
      await writeTextFile(path, csv)
      setStage({ at: 'idle' })
    } catch {
      setStage({ at: 'failed', message: MESSAGES.unsaved })
    }
  }

  const choose = async () => {
    setStage({ at: 'busy' })

    const path = await pick(() => open({ multiple: false, filters: [CSV] }))
    if (typeof path !== 'string') {
      setStage({ at: 'idle' })
      return
    }

    let bytes: Uint8Array
    try {
      bytes = await readFile(path)
    } catch {
      setStage({ at: 'failed', message: MESSAGES.unreadable })
      return
    }

    let csv: string
    try {
      // Severo apposta: e' qui che si risponde alla domanda «questo file e' UTF-8?»,
      // e piu' in la' nessuno potra' piu' porsela. Il BOM lo toglie da se'.
      csv = new TextDecoder('utf-8', { fatal: true }).decode(bytes)
    } catch {
      setStage({ at: 'failed', message: MESSAGES.encoding })
      return
    }

    const review = await checkFlashcardImport(csv)
    setStage(
      review.state === 'ready'
        ? { at: 'ready', csv, cards: review.cards }
        : { at: 'rejected', errors: review.errors },
    )
  }

  const confirm = async (csv: string) => {
    setStage({ at: 'busy' })
    // Il core ricontrolla il file, quindi puo' ancora rifiutarlo: succederebbe solo se
    // le due chiamate non vedessero lo stesso testo, ma fingere che non possa
    // accadere vorrebbe dire scrivere un ramo che mente.
    const review = await importFlashcards(deck, csv)
    if (review.state === 'rejected') {
      setStage({ at: 'rejected', errors: review.errors })
      return
    }

    setStage({ at: 'done', cards: review.cards })
    onImported()
  }

  return (
    <Sheet
      title="Import cards"
      onClose={onClose}
      action={
        stage.at === 'ready' ? (
          <Button onClick={() => void confirm(stage.csv)}>
            {stage.cards === 1 ? 'Add 1 card' : `Add ${stage.cards} cards`}
          </Button>
        ) : undefined
      }
    >
      <div className="flex flex-col gap-4 pt-2">
        {stage.at === 'busy' && <Note>Working…</Note>}

        {stage.at === 'failed' && <p className="text-accent text-sm">{stage.message}</p>}

        {(stage.at === 'idle' || stage.at === 'failed') && (
          <>
            <p className="text-muted text-sm">
              Fill the template with one card per row, then bring it back here. The
              first two columns are the card, the rest are optional.
            </p>
            <div className="flex flex-col gap-2">
              {/* Il bordo serve: `quiet` e' il colore della superficie sollevata, che
                  e' anche quella del pannello, quindi da solo sotto un bottone pieno
                  si legge come una scritta e non come una cosa che si preme. E' lo
                  stesso rimedio che la conferma a due strade usa gia'. */}
              <Button
                variant="quiet"
                className="border-hairline border"
                onClick={() => void template()}
              >
                Download the template
              </Button>
              <Button onClick={() => void choose()}>Choose a CSV file</Button>
            </div>
          </>
        )}

        {stage.at === 'ready' && (
          <p className="text-sm">
            {stage.cards === 1
              ? 'One card is ready to be added.'
              : `${stage.cards} cards are ready to be added.`}
          </p>
        )}

        {stage.at === 'rejected' && <Rejected errors={stage.errors} onBack={() => setStage({ at: 'idle' })} />}

        {stage.at === 'done' && (
          <p className="text-sm">
            {stage.cards === 1 ? 'One card was added.' : `${stage.cards} cards were added.`}
          </p>
        )}
      </div>
    </Sheet>
  )
}

/** Il filtro del selettore di sistema. */
const CSV = { name: 'CSV', extensions: ['csv'] }

/**
 * Apre un selettore, e tratta qualunque fallimento come «non hai scelto niente».
 *
 * Serve perche' su Android **annullare arriva come un errore** e non come un `null`:
 * lo fa il plugin, che su `RESULT_CANCELED` rifiuta la chiamata, e niente lo converte
 * per strada. Distinguere un annullamento da un guasto vorrebbe dire riconoscere il
 * testo del messaggio, che e' una stringa inglese dentro il plugin e cambierebbe sotto
 * i piedi alla prima riscrittura.
 *
 * Fra i due modi di sbagliare si sceglie il piu' silenzioso: annullare e' il caso
 * normale e deve riportare indietro senza dire niente, mentre un guasto vero, che qui
 * vorrebbe dire nessun gestore file sul dispositivo, lascia il pannello com'era.
 */
async function pick<T>(apri: () => Promise<T | null>): Promise<T | null> {
  return apri().catch(() => null)
}

/**
 * Come si chiama il file del modello.
 *
 * Porta il nome del mazzo perche' chi ne importa due di fila si ritroverebbe altrimenti
 * due file uguali nella cartella dei download. I caratteri che un nome di file non puo'
 * contenere si sostituiscono: il nome di un mazzo e' testo libero, e ci puo' stare una
 * barra.
 */
function filename(deck: string) {
  const pulito = deck.replace(/[/\\:*?"<>|]/g, '-').trim()
  return `tanren-${pulito || 'deck'}-template.csv`
}

/** L'elenco delle righe da correggere, e come si torna indietro. */
function Rejected({ errors, onBack }: { errors: ImportRowError[]; onBack: () => void }) {
  // Il file sbagliato e non una riga sbagliata: dirlo per riga sarebbe fuorviante.
  const solaIntestazione = errors.length === 1 && errors[0].problem.kind === 'header'

  return (
    <div className="flex flex-col gap-3">
      <p className="text-accent text-sm">
        {solaIntestazione
          ? describe(errors[0].problem)
          : 'Nothing was imported. Fix these rows and try again.'}
      </p>

      {!solaIntestazione && (
        <ul className="flex flex-col gap-1">
          {errors.slice(0, SHOWN).map((e, i) => (
            <li key={i} className="text-muted text-sm">
              <span className="text-paper">Row {e.line}</span>: {describe(e.problem)}
            </li>
          ))}
          {errors.length > SHOWN && (
            <li className="text-muted text-sm">
              and {errors.length - SHOWN} more
            </li>
          )}
        </ul>
      )}

      <Button variant="quiet" className="border-hairline border" onClick={onBack}>
        Choose another file
      </Button>
    </div>
  )
}

/**
 * Cosa c'e' che non va, detto in inglese.
 *
 * Il core manda un'etichetta e non una frase, come gia' per `asks` e per i motivi per
 * cui una lezione e' chiusa: la lingua dell'interfaccia sta qui.
 */
function describe(problem: ImportProblem): string {
  switch (problem.kind) {
    case 'header':
      return 'The first row of this file does not hold the column names. Download the template and write your cards under its first row.'
    case 'empty_field':
      return `the ${problem.field} column cannot be empty`
    case 'too_many_values':
      return `more than ${problem.max} extra meanings`
    case 'too_many_columns':
      return `${problem.found} columns instead of ${problem.expected}, which usually means a comma inside a field that is not between quotes`
    case 'malformed':
      return 'this row cannot be read, which usually means a quote that was opened and never closed'
  }
}
