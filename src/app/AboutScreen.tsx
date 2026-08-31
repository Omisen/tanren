import { useEffect, useState } from 'react'

import { appVersion, credits, type Credit } from '@/shared/bridge'
import { Note } from '@/shared/ui/Card'
import { ExternalLink } from '@/shared/ui/ExternalLink'
import { Screen } from '@/shared/ui/Screen'
import { Sheet } from '@/shared/ui/Sheet'
import { useUi } from '@/shared/store/ui'

/**
 * Da dove vengono i dati, e sotto quale licenza Tanren li ridistribuisce.
 *
 * # Non e' una schermata di cortesia
 *
 * La CC BY-SA di kanjium e la licenza dell'EDRDG obbligano ad attribuire **dentro il
 * mezzo in cui l'opera viaggia**. Per un'app quel mezzo e' l'APK installato: il README
 * della repo non assolve l'obbligo, perche' chi installa l'app il README non lo vede.
 * Finche' questa schermata non c'e', ogni release coi dati dei kanji e' in violazione.
 *
 * # Le due cose che la rendono conforme e non «quasi»
 *
 * Che sia **raggiungibile** da chi ha solo l'APK, ed e' il motivo per cui sta dietro un
 * bottone in cima a ogni schermata iniziale. E che ci sia il **testo** delle licenze e
 * non solo il nome: dove il testo e' imbarcato si apre qui, dove non lo e' resta il
 * link, che e' l'alternativa che quelle licenze ammettono.
 *
 * # E la meta' che si dimentica
 *
 * Lo ShareAlike non chiede solo di dire da chi hai preso: chiede di dire **sotto quale
 * licenza ridistribuisci tu**. E' la voce «Tanren» in fondo all'elenco, e non e' un
 * vezzo, e' l'obbligo.
 */
export function AboutScreen() {
  const goTo = useUi((s) => s.goTo)
  const [fonti, setFonti] = useState<Credit[] | null | 'failed'>(null)
  const [versione, setVersione] = useState<string | null>(null)
  const [aperta, setAperta] = useState<Credit | null>(null)

  useEffect(() => {
    let current = true
    credits()
      .then((c) => current && setFonti(c))
      .catch(() => current && setFonti('failed'))
    appVersion()
      .then((v) => current && setVersione(v))
      .catch(() => {})
    return () => {
      current = false
    }
  }, [])

  return (
    <>
      <Screen title="Sources and licences" onBack={() => goTo('home')}>
        {fonti === null && <Note>Loading…</Note>}
        {fonti === 'failed' && <Note>The credits could not be loaded.</Note>}

        {Array.isArray(fonti) && (
          <div className="flex flex-col gap-6">
            <p className="text-muted text-sm">
              Tanren is built on work other people made and gave away. What follows says
              what each one covers, under which licence, and under which licence Tanren
              passes it on.
            </p>

            {fonti.map((c) => (
              <Entry key={c.name} credit={c} onRead={() => setAperta(c)} />
            ))}

            <p className="text-muted text-xs">
              Tanren {versione ?? ''} · github.com/Omisen/tanren
            </p>
          </div>
        )}
      </Screen>

      {aperta && <LicenceSheet credit={aperta} onClose={() => setAperta(null)} />}
    </>
  )
}

function Entry({ credit, onRead }: { credit: Credit; onRead: () => void }) {
  return (
    <section className="border-hairline bg-ink-soft flex flex-col gap-2 rounded-xl border p-4">
      <h2 className="text-base">{credit.name}</h2>
      <p className="text-muted text-sm">{credit.covers}</p>

      {/* La frase che una fonte chiede di riportare va riportata, non riassunta: se una
          licenza detta le parole, sono quelle. */}
      {credit.notice && (
        <p className="border-hairline text-paper mt-1 border-l-2 pl-3 text-sm italic">
          {credit.notice}
        </p>
      )}

      {credit.edition && (
        <p className="text-muted text-xs">Edition shipped: {credit.edition}</p>
      )}

      <div className="mt-2 flex flex-col gap-1">
        <p className="text-sm">{credit.licence}</p>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
          {credit.licenceFile && (
            <button
              type="button"
              onClick={onRead}
              className="text-type-kanji min-h-11 underline underline-offset-4"
            >
              Read the licence
            </button>
          )}
          <ExternalLink
            href={credit.licenceUrl}
            className="text-muted flex min-h-11 items-center underline underline-offset-4"
          >
            {credit.licenceUrl}
          </ExternalLink>
          {credit.sourceUrl && (
            <ExternalLink
              href={credit.sourceUrl}
              className="text-muted flex min-h-11 items-center underline underline-offset-4"
            >
              {credit.sourceUrl}
            </ExternalLink>
          )}
        </div>
      </div>
    </section>
  )
}

/**
 * Il testo di una licenza, letto dai file imbarcati con l'app.
 *
 * Sta in `public/`, che finisce dentro il pacchetto: e' lo stesso posto e lo stesso
 * modo con cui gia' viaggia la licenza del font. Se per qualche ragione non si
 * caricasse, resta il link, che quelle licenze ammettono come alternativa.
 */
function LicenceSheet({ credit, onClose }: { credit: Credit; onClose: () => void }) {
  const [testo, setTesto] = useState<string | null | 'failed'>(null)

  useEffect(() => {
    let current = true
    if (!credit.licenceFile) return
    fetch(credit.licenceFile)
      .then((r) => (r.ok ? r.text() : Promise.reject(new Error('non trovata'))))
      .then((t) => current && setTesto(t))
      .catch(() => current && setTesto('failed'))
    return () => {
      current = false
    }
  }, [credit.licenceFile])

  return (
    <Sheet title={credit.licence} onClose={onClose}>
      {testo === null && <Note>Loading…</Note>}
      {testo === 'failed' && (
        <div className="flex flex-col gap-2">
          <Note>The bundled copy could not be opened. It is online here:</Note>
          <ExternalLink
            href={credit.licenceUrl}
            className="text-type-kanji text-sm underline underline-offset-4"
          >
            {credit.licenceUrl}
          </ExternalLink>
        </div>
      )}
      {typeof testo === 'string' && testo !== 'failed' && (
        <pre className="text-muted text-xs whitespace-pre-wrap">{testo}</pre>
      )}
    </Sheet>
  )
}
