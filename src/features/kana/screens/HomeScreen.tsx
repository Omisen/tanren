import { useEffect, useState, type ReactNode } from 'react'

import {
  kanaCatalogue,
  kanaPatterns,
  type KanaCell,
  type KanaGroup,
  type KanaPattern,
  type KanaSet,
  type PatternCell,
  type PatternGroup,
  type Syllabary,
  type Vowel,
} from '@/shared/bridge'
import { Button } from '@/shared/ui/Button'
import { Card, Note } from '@/shared/ui/Card'
import { LogoMark } from '@/shared/ui/LogoMark'
import { Screen } from '@/shared/ui/Screen'
import { useUi } from '@/shared/store/ui'

const SYLLABARIES = [
  { value: 'hiragana', label: 'ひらがな', caption: 'Hiragana' },
  { value: 'katakana', label: 'カタカナ', caption: 'Katakana' },
] as const

const MODES = [
  { value: 'recognition', label: 'Recognition', caption: 'Pick the reading' },
  { value: 'input', label: 'Writing', caption: 'Type with the IME' },
] as const

/**
 * Come si chiamano le famiglie, coi termini giapponesi e senza glossa.
 *
 * Sono i nomi che usa chi studia, e tradurli («Voiced», «Combinations») dava una parola
 * che non si incontra da nessun'altra parte: qui l'immersione costa niente, perche' i
 * segni della famiglia sono li' accanto a spiegarla meglio di un'etichetta. Adesso lo
 * sono ancora di piu', perche' la tavola sta aperta sotto il nome.
 *
 * `japanese` dice se il nome e' scritto in giapponese, e serve a due cose che vanno
 * insieme: il font imbarcato e l'attributo di lingua. Senza, 外来音 finirebbe nel
 * ripiego di sistema, cioe' rettangoli vuoti su un Linux senza font CJK, che e'
 * esattamente il caso per cui il font e' imbarcato.
 */
const GROUP_LABELS: Record<KanaGroup, { text: string; japanese?: boolean }> = {
  base: { text: 'Base' },
  dakuten: { text: 'Dakuten' },
  handakuten: { text: 'Handakuten' },
  yoon: { text: 'Yōon' },
  gairaion: { text: '外来音', japanese: true },
}

/** Le cinque colonne della tavola, nell'ordine tradizionale. */
const VOWELS: Vowel[] = ['a', 'i', 'u', 'e', 'o']

/**
 * La geometria della tavola, famiglia per famiglia.
 *
 * E' **presentazione e non dominio**: dice quanto e' larga una riga sullo schermo, non
 * dove sta un segno. Dove sta lo dice il core, che manda la colonna di ciascuno, e qui
 * ci si limita a mettere ogni segno nel posto che la sua colonna indica, lasciando
 * vuoti quelli che nessun segno reclama. E' cosi' che la riga `ya` esce や・_・ゆ・_・よ
 * invece di far scalare よ al posto di ゆ.
 *
 * Lo yoon ha tre colonne e non cinque perche' le sue combinazioni esistono solo su ゃ,
 * ゅ e ょ: le colonne `i` ed `e` non ci sono proprio, e tenerle vuote per tutte e
 * dodici le righe vorrebbe dire disegnare due colonne di niente.
 *
 * I 外来音 tengono invece le cinque, anche se le loro righe sono sbilenche (`fa` non
 * ha la `u`, `che` ha il solo チェ): allineandoli alle altre famiglie si legge dove
 * ogni suono cade, che e' proprio quello che distingue ファ da フォ.
 */
const TABLE: Record<KanaGroup, { columns: Vowel[]; grid: string }> = {
  base: { columns: VOWELS, grid: 'grid-cols-5' },
  dakuten: { columns: VOWELS, grid: 'grid-cols-5' },
  handakuten: { columns: VOWELS, grid: 'grid-cols-5' },
  yoon: { columns: ['a', 'u', 'o'], grid: 'grid-cols-3' },
  gairaion: { columns: VOWELS, grid: 'grid-cols-5' },
}

const PATTERN_LABELS: Record<PatternGroup, string> = {
  double: 'Double',
  long: 'Long',
}

/**
 * La geometria delle due regole.
 *
 * `columns` a `null` vuol dire che la riga **non e' una tavola di vocali**: le caselle
 * si mettono una dopo l'altra, nell'ordine in cui arrivano. E' il caso del sokuon, che
 * in una colonna non ci sta, mentre le vocali lunghe ce l'hanno e infatti i loro buchi
 * cadono dove devono, sotto ええ e おお.
 */
const PATTERN_TABLE: Record<PatternGroup, { columns: Vowel[] | null; grid: string }> = {
  double: { columns: null, grid: 'grid-cols-5' },
  long: { columns: VOWELS, grid: 'grid-cols-5' },
}

/**
 * La scelta dell'ambito sui kana.
 *
 * # Com'e' fatta
 *
 * Tre fasce che dicono tre cose diverse, e solo quella di mezzo scorre:
 *
 * - **in cima, fissa,** in quale dei due sillabari si sta guardando;
 * - **in mezzo,** tutte le famiglie aperte a cascata, ognuna con la sua tavola e la
 *   sua casella di spunta;
 * - **in fondo, fissa,** come si vuole rispondere e il bottone che fa partire il giro.
 *
 * Le due fasce ferme sono quelle che si toccano prima e dopo aver guardato, e scorrere
 * per ritrovarle sarebbe il difetto che questo impianto toglie.
 *
 * # Perche' i segni si vedono tutti
 *
 * Perche' prima si sceglieva al buio: cinque pastiglie con un numero accanto, e cosa
 * ci fosse dentro «Yōon» lo sapeva solo chi lo sapeva gia'. La tavola aperta e' la
 * cosa stessa che si sta per studiare, e la casella di spunta accanto al nome dice che
 * quella li' entra nel giro.
 *
 * La scelta della materia non e' piu' qui dentro: e' una sezione, e la barra in fondo
 * arriva dall'alto come nodo gia' fatto. Questa schermata sa solo di kana, e con la
 * regola di non incrocio fra feature non potrebbe nemmeno nominare i kanji.
 */
export function KanaHomeScreen({
  tabs,
}: {
  /** La barra delle sezioni, composta dalla radice. */
  tabs: ReactNode
}) {
  const { kana: scope, setSyllabary, setKanaMode, toggleGroup, goTo } = useUi()
  /**
   * I due cataloghi, tenuti tutti e due.
   *
   * `undefined` vuol dire non ancora arrivato, `null` che la richiesta e' fallita: sono
   * due cose diverse e la schermata le dice diversamente.
   */
  const [catalogues, setCatalogues] = useState<
    Partial<Record<Syllabary, KanaSet[] | null>>
  >({})
  /**
   * Le due regole di sola consultazione, tenute a parte dal catalogo.
   *
   * Sono una seconda chiamata e non un pezzo della prima perche' rispondono a due
   * domande diverse: il catalogo dice cosa si puo' allenare, e da li' escono il
   * conteggio e lo stato di Start; queste dicono cosa c'e' da sapere e basta. Tenute
   * separate, non possono influenzare niente di quello per sbaglio.
   *
   * Se non arrivano non si dice niente e non si mostra niente: il resto della schermata
   * funziona, perche' qui non c'e' nessuna scelta da fare che possa restare a meta'.
   */
  const [references, setReferences] = useState<Partial<Record<Syllabary, KanaPattern[]>>>(
    {},
  )

  // **Si chiedono tutti e due all'apertura, una volta sola.**
  //
  // Prima si chiedeva quello del sillabario scelto a ogni cambio, e passare da
  // hiragana a katakana lasciava un buco: le pastiglie sparivano, al loro posto
  // restava un «Loading…» alto una riga, e la schermata si accartocciava e si riapriva.
  // Il difetto era lo stesso della griglia dei kanji, ma qui c'e' un rimedio migliore
  // del tenere il posto: i cataloghi sono **due**, sono tabelle statiche dentro il
  // binario e non cambiano mai finche' l'app e' viva, quindi la cosa giusta non e'
  // riempire meglio l'attesa, e' non avere nessuna attesa da riempire. Dopo l'apertura
  // scambiare sillabario non chiede piu' niente a nessuno.
  //
  // Adesso conta il doppio: il catalogo non porta piu' cinque conteggi ma tutti i
  // segni, e sono loro che riempiono la cascata.
  useEffect(() => {
    let current = true

    for (const { value } of SYLLABARIES) {
      kanaCatalogue(value)
        .then((sets) => current && setCatalogues((c) => ({ ...c, [value]: sets })))
        .catch(() => current && setCatalogues((c) => ({ ...c, [value]: null })))

      kanaPatterns(value)
        .then((p) => current && setReferences((r) => ({ ...r, [value]: p })))
        .catch(() => {})
    }

    return () => {
      current = false
    }
  }, [])

  const entry = catalogues[scope.syllabary]
  const sets = entry ?? null
  const failed = entry === null

  // Si guarda cosa il **catalogo** offre e non cosa e' rimasto selezionato: i 外来音
  // esistono solo in katakana, quindi tornando all'hiragana quella scelta non ha piu'
  // un insieme dietro. Filtrando qui, la selezione sopravvive al giro di andata e
  // ritorno fra i due sillabari senza che si possa partire su una coda vuota.
  const chosen = sets?.filter((s) => scope.groups.includes(s.group)) ?? []
  const total = chosen.reduce((sum, s) => sum + s.size, 0)

  return (
    <Screen
      textured
      title="Tanren"
      mark={<LogoMark />}
      tabs={tabs}
      action={
        <div className="enter flex flex-col gap-2">
          <div className="grid grid-cols-2 gap-2">
            {MODES.map((m) => (
              <Card
                key={m.value}
                pressed={scope.mode === m.value}
                onClick={() => setKanaMode(m.value)}
              >
                <span className="text-base">{m.label}</span>
                <span className="text-muted text-xs">{m.caption}</span>
              </Card>
            ))}
          </div>

          {/* Il bottone resta spento finche' non c'e' niente di spuntato, e non e'
              cortesia: nel core una lista di famiglie **vuota significa tutte**, quindi
              questa e' l'unica cosa che impedisce di partire per sbaglio sull'intero
              sillabario credendo di aver scelto niente. */}
          <Button disabled={chosen.length === 0} onClick={() => goTo('session')}>
            {total > 0 ? `Start with ${total} characters` : 'Start'}
          </Button>
        </div>
      }
    >
      {/* Le due fasce si animano a parte invece di stare dentro un involucro solo: la
          prima e' incollata in cima, e un antenato che si muove e' l'ultima cosa che le
          serve. Partono insieme, quindi si vede un movimento solo. */}
      <div className="enter bg-ink sticky top-0 z-10 -mx-4 grid grid-cols-2 gap-2 px-4 pt-1 pb-3">
        {SYLLABARIES.map((s) => (
          <Card
            key={s.value}
            pressed={scope.syllabary === s.value}
            onClick={() => setSyllabary(s.value)}
          >
            <span className="font-jp text-2xl" lang="ja">
              {s.label}
            </span>
            <span className="text-muted text-xs">{s.caption}</span>
          </Card>
        ))}
      </div>

      <div className="enter flex flex-col gap-6">
        {failed && <Note>The catalogue is not reachable.</Note>}
        {!failed && !sets && <Note>Loading…</Note>}
        {sets?.map((set) => (
          <Family
            key={set.group}
            set={set}
            chosen={scope.groups.includes(set.group)}
            onToggle={() => toggleGroup(set.group)}
          />
        ))}

        {/* In fondo, dopo le famiglie che si allenano: sono cose da sapere, non da
            scegliere, e l'ordine lo dice. */}
        {references[scope.syllabary]?.map((pattern) => (
          <Reference key={pattern.group} pattern={pattern} />
        ))}
      </div>
    </Screen>
  )
}

/**
 * Una famiglia: il nome con la sua casella, e sotto la tavola dei suoi segni.
 *
 * Dakuten e handakuten restano **due blocchi con due caselle**, anche se si leggono
 * come un seguito solo: sono due famiglie che si possono studiare separatamente, e una
 * casella per due toglierebbe proprio quella scelta.
 */
function Family({
  set,
  chosen,
  onToggle,
}: {
  set: KanaSet
  chosen: boolean
  onToggle: () => void
}) {
  const label = GROUP_LABELS[set.group]
  const { columns, grid } = TABLE[set.group]

  return (
    <section className="flex flex-col gap-2">
      {/* La riga intera e' il comando, non il quadratino: e' larga quanto lo schermo e
          alta quanto serve al pollice, mentre un bersaglio da venti pixel su un
          telefono si manca. */}
      <button
        type="button"
        role="checkbox"
        aria-checked={chosen}
        onClick={onToggle}
        className="flex min-h-11 w-full items-center gap-3 text-left active:opacity-70"
      >
        <Box chosen={chosen} />
        {/* Lo stato si legge anche senza il colore del quadratino: il nome di una
            famiglia che entra nel giro e' testo pieno, quello di una che resta fuori e'
            secondario. E' la stessa gerarchia della barra delle sezioni. */}
        <span
          className={`text-sm ${chosen ? 'text-paper' : 'text-muted'} ${
            label.japanese ? 'font-jp' : ''
          }`}
          lang={label.japanese ? 'ja' : undefined}
        >
          {label.text}
        </span>
        <span className="text-muted ml-auto text-xs">{set.size}</span>
      </button>

      <div className="flex flex-col gap-1.5">
        {set.rows.map((row) => (
          <Row key={row.row} cells={row.cells} columns={columns} grid={grid} />
        ))}
      </div>
    </section>
  )
}

/**
 * La casella di spunta. Piena vuol dire che la famiglia entra nel giro.
 *
 * Spenta porta il bordo di cio' che e' inattivo, che e' il token fatto apposta e fa
 * coppia con `inactive`. Il nome della famiglia resta invece leggibile come le altre:
 * quello che non si puo' fare e' sceglierla, non leggerla.
 */
function Box({ chosen, off = false }: { chosen: boolean; off?: boolean }) {
  return (
    <span
      aria-hidden="true"
      className={`flex size-5 shrink-0 items-center justify-center rounded text-xs transition-colors ${
        chosen ? 'bg-selected text-paper' : off ? 'border-hairline-soft border' : 'border-hairline border'
      }`}
    >
      {chosen ? '✓' : ''}
    </span>
  )
}

/**
 * Una regola che si mostra e non si sceglie: il sokuon e le vocali lunghe.
 *
 * # Perche' la casella c'e' lo stesso, spenta
 *
 * Perche' il gruppo **esiste** e va visto, ma non e' ancora qualcosa che si possa
 * allenare: una casella spenta lo dice senza che serva scriverlo, com'e' spenta la
 * barra del progresso nelle celle. Resta un bottone e non diventa un pezzo di testo
 * perche' cosi' chi naviga da tastiera ci arriva e sente che c'e' e che non e'
 * disponibile: con l'attributo `disabled` il bottone uscirebbe dal giro dei fuochi e di
 * quel gruppo non saprebbe niente, che e' l'opposto di quello che serve qui.
 *
 * # Perche' al posto del numero c'e' una parola
 *
 * Perche' accanto alle altre famiglie quel numero ha un significato solo, **quanti
 * segni aggiunge al giro**, e qui sarebbe zero: scriverlo direbbe il falso proprio dove
 * si sta dicendo che questo gruppo non entra da nessuna parte. Al suo posto, nello
 * stesso angolo dove l'occhio lo cerca gia', c'e' il motivo per cui la casella e'
 * spenta.
 */
function Reference({ pattern }: { pattern: KanaPattern }) {
  const label = PATTERN_LABELS[pattern.group]
  const { columns, grid } = PATTERN_TABLE[pattern.group]

  return (
    <section className="flex flex-col gap-2">
      <button
        type="button"
        role="checkbox"
        aria-checked={false}
        aria-disabled="true"
        className="flex min-h-11 w-full items-center gap-3 text-left"
      >
        <Box chosen={false} off />
        <span className="text-muted text-sm">{label}</span>
        <span className="text-muted ml-auto text-xs">Reference</span>
      </button>

      <div className="flex flex-col gap-1.5">
        {pattern.rows.map((cells, i) => (
          <Row key={i} cells={cells} columns={columns} grid={grid} />
        ))}
      </div>
    </section>
  )
}

/**
 * Quello che serve a disegnare una casella.
 *
 * Le due fonti hanno la stessa faccia ma non sono la stessa cosa, ed e' voluto che i
 * tipi restino due: `KanaCell` viene dal catalogo di cio' che si allena, `PatternCell`
 * da una regola che non si allena. Qui si guarda solo come si disegnano.
 */
type Cella = KanaCell | PatternCell

/**
 * Una riga, coi buchi al posto loro.
 *
 * Con delle `columns` ogni segno va nella casella della sua colonna, che decide il
 * core, e le caselle che nessun segno reclama restano vuote. ん non ha colonna e va
 * nella prima, che e' come sta nella tavola vera, da solo in fondo.
 *
 * Senza `columns` la riga non e' una tavola di vocali e le caselle si mettono una dopo
 * l'altra: e' il sokuon, che in una colonna non ci sta.
 */
function Row({
  cells,
  columns,
  grid,
}: {
  cells: Cella[]
  columns: Vowel[] | null
  grid: string
}) {
  const slots: (Cella | null)[] = columns ? columns.map(() => null) : [...cells]

  if (columns) {
    for (const cell of cells) {
      const at = cell.column ? columns.indexOf(cell.column) : 0
      if (at >= 0) slots[at] = cell
    }
  }

  return (
    <div className={`grid gap-1.5 ${grid}`}>
      {slots.map((cell, i) => (
        <Cell key={cell?.character ?? `vuota-${i}`} cell={cell} />
      ))}
    </div>
  )
}

/**
 * Un segno, con la sua trascrizione sotto e la barra del progresso.
 *
 * **La barra e' spenta, e non per finta**: sui kana non si scrive nessuna carta, quindi
 * un progresso per segno oggi non esiste da nessuna parte e non c'e' niente da
 * riempire. Sta qui perche' il posto e' suo, e quando il learning arrivera' si
 * accendera' senza far ballare la griglia.
 */
function Cell({ cell }: { cell: Cella | null }) {
  if (!cell) return <div aria-hidden="true" />

  return (
    <div className="border-hairline bg-ink-soft flex flex-col items-center gap-1 rounded-lg border px-1 py-2">
      <span className="font-jp text-xl leading-none" lang="ja">
        {cell.character}
      </span>
      <span className="text-muted text-xs leading-none">{cell.romaji}</span>
      <span className="bg-hairline h-1 w-full rounded-full" aria-hidden="true" />
    </div>
  )
}
