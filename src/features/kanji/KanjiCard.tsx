import type { Kanji } from '@/shared/bridge'

/**
 * La scheda di un kanji: cosa vuol dire e come si legge.
 *
 * E' la stessa cosa che si mostra per **conoscere** un kanji la prima volta e per
 * **riguardarlo** dopo: sono lo stesso bisogno a due momenti diversi, e due schede
 * diverse divergerebbero come sono gia' divergiti `Chip` e `Card`.
 *
 * # Le due etichette PRIMARY sono su assi diversi
 *
 * Una dice quale significato e' quello principale, l'altra quale lettura on pesa di
 * piu' nei composti veri. Non sono la stessa scelta e non si implicano: 生 significa
 * «life» e si legge セイ, ma sono due fatti indipendenti.
 *
 * # Perche' le on sono in katakana e le kun in hiragana
 *
 * E' la convenzione dei dizionari, e mostrarla insegna a riconoscere il tipo di
 * lettura dalla forma delle lettere prima ancora di saperla.
 */
export function KanjiCard({ kanji }: { kanji: Kanji }) {
  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col items-center gap-3">
        <div className="bg-type-kanji flex aspect-square h-[clamp(5rem,20vh,12rem)] items-center justify-center rounded-3xl">
          <p className="font-jp text-paper text-[clamp(2.5rem,10vh,6rem)] leading-none" lang="ja">
            {kanji.character}
          </p>
        </div>
        <p className="text-center text-xl">
          {kanji.meanings[0]}
          <Primary />
        </p>
        {kanji.meanings.length > 1 && (
          <p className="text-muted text-center text-sm">{kanji.meanings.slice(1).join(', ')}</p>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Readings
          label="On'yomi"
          readings={kanji.on}
          rare={kanji.onRare}
          primary={kanji.primaryOn}
          empty="none"
        />
        <Readings label="Kun'yomi" readings={kanji.kun} rare={kanji.kunRare} empty="none" />
      </div>

      {kanji.okurigana.length > 0 && (
        <Section label="With okurigana">
          <div className="flex flex-col gap-1">
            {kanji.okurigana.map((o) => (
              <p key={o.form} className="flex items-baseline justify-between gap-3">
                <span className="font-jp text-lg" lang="ja">
                  {o.form}
                </span>
                <span className="text-muted font-jp text-sm" lang="ja">
                  {o.readings.join(' · ')}
                </span>
              </p>
            ))}
          </div>
        </Section>
      )}

      {kanji.nanori.length > 0 && (
        /* I nanori si mostrano e non si chiedono: sono le letture dei nomi propri, e
           chiederle vorrebbe dire interrogare su cose che in un testo non si leggono. */
        <Section label="Nanori">
          <p className="text-muted font-jp text-sm" lang="ja">
            {kanji.nanori.join('、')}
          </p>
        </Section>
      )}

      {kanji.examples.length > 0 && (
        <Section label="Examples">
          <div className="flex flex-col gap-2">
            {kanji.examples.map((e) => (
              <div key={e.word} className="flex flex-col">
                <p className="flex items-baseline gap-2">
                  <span className="font-jp text-lg" lang="ja">
                    {e.word}
                  </span>
                  <span className="text-muted font-jp text-sm" lang="ja">
                    {e.reading}
                  </span>
                </p>
                <p className="text-muted text-sm">{e.meaning}</p>
              </div>
            ))}
          </div>
        </Section>
      )}
    </div>
  )
}

/** Il marcatore di cio' che viene prima fra piu' cose vere insieme. */
function Primary() {
  return (
    <span className="text-type-kanji ml-2 align-middle text-[0.6rem] font-medium tracking-[0.15em] uppercase">
      primary
    </span>
  )
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-muted text-xs font-medium tracking-[0.2em] uppercase">{label}</h3>
      {children}
    </section>
  )
}

function Readings({
  label,
  readings,
  rare,
  primary,
  empty,
}: {
  label: string
  readings: string[]
  rare?: string[]
  primary?: string | null
  empty: string
}) {
  return (
    <Section label={label}>
      {readings.length === 0 ? (
        <p className="text-inactive text-sm">{empty}</p>
      ) : (
        <div className="flex flex-col gap-1">
          {readings.map((r) => (
            <p key={r} className="font-jp text-lg" lang="ja">
              {r}
              {r === primary && <Primary />}
            </p>
          ))}
          {/* Le letture rare restano visibili ma smorzate: esistono, e chi legge deve
              poterle trovare, ma non sono quelle da imparare adesso. */}
          {rare?.map((r) => (
            <p key={r} className="text-inactive font-jp text-lg" lang="ja">
              {r}
              <span className="ml-2 align-middle text-[0.6rem] tracking-[0.15em] uppercase">
                rare
              </span>
            </p>
          ))}
        </div>
      )}
    </Section>
  )
}
