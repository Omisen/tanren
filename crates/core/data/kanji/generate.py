# -*- coding: utf-8 -*-
"""Genera le tabelle dei kanji joyo a partire da kanjium.

Le tabelle non si modificano a mano: si cambia questo script e si rigenera. Il dato
deriva da una fonte esterna, quindi ogni file dichiara da dove viene.

    python3 generate.py                      scarica quello che serve e rigenera
    python3 generate.py --db FILE --freq FILE  usa copie locali

# Le fonti, e perche' due

`kanjidb.sqlite` (36 MB) porta kanji, letture, significati, composti e la
segmentazione delle letture dentro i composti. **Non porta pero' una frequenza
numerica**: la sua colonna `frequency` e' una classe testuale (`Very common`,
`Common`, `Uncommon`, `Rare`), quindi da sola non permette di calcolare quanto un
kanji ricorra da solo invece che dentro un composto.

`wikipedia_freq.txt` (20.001 parole con conteggi veri) copre quel buco, ed e' il
motivo per cui si scaricano due file invece di uno. **`novels_freq.txt`, che sta
accanto, si scarta**: da' 人 isolato a zero, che e' impossibile, e mette 要る come
parola piu' frequente della lingua. E' rumore di lemmatizzazione.

# Licenza

kanjium e' CC BY-SA 4.0 e contiene EDICT, KANJIDIC e KRADFILE dell'EDRDG, anch'essi
CC BY-SA 4.0. Le due attribuzioni **si sommano**: vedi ATTRIBUTION.md accanto a
questo file. I file prodotti qui sono a loro volta CC BY-SA 4.0, perche' la licenza
si estende esplicitamente ai dati derivati.
"""
import argparse
import collections
import json
import re
import sqlite3
import sys
import urllib.request
from pathlib import Path

QUI = Path(__file__).parent
USCITA = QUI / "levels"

BASE = "https://raw.githubusercontent.com/mifunetoshiro/kanjium/master/data"
DB_URL = f"{BASE}/kanjidb.sqlite"
FREQ_URL = f"{BASE}/source_files/raw/wikipedia_freq.txt"

# La provenienza esatta: kanjium non ha numeri di versione, quindi si registra il
# commit che ha toccato per l'ultima volta il database. E' fermo al 2020, cosa da
# sapere: KANJIDIC2 invece si aggiorna ogni giorno. Per la lista dei joyo, ferma per
# decreto dal 2010, sei anni di scarto non spostano niente.
SOURCE = {
    "dataset": "kanjium",
    "commit": "c97ef30c8777de7d5f8e5b04c696a3fe00a2a83a",
    "committed": "2020-03-30",
    "licence": "CC BY-SA 4.0",
    "url": "https://github.com/mifunetoshiro/kanjium",
    "includes": "EDICT, KANJIDIC, KRADFILE (EDRDG, CC BY-SA 4.0)",
}

FORMAT_VERSION = 1

# Quanti kanji per livello nella coda oltre WaniKani, vedi `livelli()`.
CODA = 25

# Quanto pesa un composto quando si cerca la lettura on dominante. Le classi sono
# quelle di kanjium; l'asterisco marca l'ambiguo e non cambia la classe.
PESO = {"Very common": 8, "Common": 4, "Uncommon": 2, "Rare": 1}

# Quanti esempi tenere per kanji.
ESEMPI = 4


def scarica(url: str, dove: Path) -> None:
    print(f"  scarico {dove.name}", file=sys.stderr)
    with urllib.request.urlopen(url) as r:
        dove.write_bytes(r.read())


def intero(v) -> int | None:
    """Un numero, oppure niente.

    kanjium usa la **stringa vuota** dove ci si aspetterebbe NULL, e lo fa su piu' di
    una colonna: `wanikani` e `frequency` almeno. Chi la legge come se fosse un numero
    conta 2.136 kanji con un livello dove sono 1.662, ed e' un errore che non si vede
    finche' qualcosa non prova a leggerlo davvero.
    """
    return v if isinstance(v, int) else None


def kata_to_hira(s: str) -> str:
    return "".join(chr(ord(c) - 0x60) if "ァ" <= c <= "ヶ" else c for c in s)


# Le sonore e le semisonore, per riconoscere il rendaku: 生 in 誕生 si legge じょう,
# che e' ショウ sonorizzata. Senza questa tabella quel composto non verrebbe contato.
SORDA = {}
for sonora, sorda in zip("がぎぐげござじずぜぞだぢづでどばびぶべぼぱぴぷぺぽ",
                         "かきくけこさしすせそたちつてとはひふへほはひふへほ"):
    SORDA[sonora] = sorda


def combacia(segmento: str, on: str) -> bool:
    """Se una porzione di lettura di un composto e' quella lettura on.

    Dentro un composto una lettura on si deforma in modi regolari: raddoppia in
    geminata (ガク diventa がっ in 学校), perde l'ultima mora (ニチ diventa に), e si
    sonorizza per rendaku (ショウ diventa じょう in 誕生). Senza riconoscerle si
    perderebbero proprio i composti piu' comuni, che sono quelli che deformano.
    """
    o = kata_to_hira(on)
    for s in {segmento, SORDA.get(segmento[0], segmento[0]) + segmento[1:]}:
        if s == o or s == o[:-1] or (s.endswith("っ") and s[:-1] == o[:-1]):
            return True
    return False


def letture(reg: str) -> tuple[list[str], list[str]]:
    """Le letture di `reg_on` o `reg_kun`, separate fra regolari e rare.

    L'asterisco marca la lettura rara. Ce l'hanno 157 kanji su 6.813, quindi **non**
    e' il modo di distinguere la lettura primaria dalle secondarie: dice solo che
    quella lettura si incontra di rado. La primaria si deriva a parte.
    """
    regolari, rare = [], []
    for r in filter(None, (x.strip() for x in (reg or "").split("、"))):
        (rare if r.endswith("*") else regolari).append(r.rstrip("*"))
    return regolari, rare


def spezza_kun(regolari: list[str], rare: list[str]) -> tuple[list, list, list]:
    """Divide le kun fra letture del kanji nudo e forme scritte con l'okurigana.

    kanjium scrive l'okurigana fra parentesi piene: `い（きる）` vuol dire che il
    kanji copre solo `い` e il resto si scrive in kana. La forma scritta e' quindi
    `生きる` e si legge `いきる`.

    **Una forma puo' avere piu' letture ed e' un item solo**: 行く si legge sia いく
    sia ゆく. Sono due risposte buone alla stessa domanda, non due domande.
    """
    nude, nude_rare = [], []
    forme: dict[str, list[str]] = {}
    for elenco, dove in ((regolari, nude), (rare, nude_rare)):
        for r in elenco:
            m = re.match(r"^([^（]+)（([^）]+)）$", r)
            if m:
                forme.setdefault(m.group(2), []).append(m.group(1) + m.group(2))
            else:
                dove.append(r)
    return nude, nude_rare, forme


def livelli(voci: list[dict]) -> None:
    """Assegna a ogni kanji il livello a cui appartiene, scrivendolo dentro la voce.

    L'asse e' quello di **WaniKani**, che kanjium porta: cinquanta livelli da una
    ventina di kanji, ordinati per componenti, cioe' una progressione pensata per chi
    impara. Gli anni di scuola giapponesi restano nel dato ma non sono l'asse: non
    sono un ordine di difficolta' per chi non e' madrelingua, e un anno da 328 kanji
    non si finisce mai.

    **WaniKani pero' non copre tutti i joyo: sono 1.662 su 2.136.** I 474 che restano
    fuori sono quasi tutti kanji rari delle medie e delle superiori (il piu' comune e'
    藩, al posto 807 per frequenza); delle elementari ne mancano dieci. Lasciarli fuori
    vorrebbe dire non poter mai studiare un quinto dei joyo, quindi si accodano in
    fondo in livelli da venticinque, ordinati per frequenza. Che stiano per ultimi non
    e' un ripiego: sono i joyo che si incontrano di meno.
    """
    coperti = [v for v in voci if v.pop("_wk") is not None]
    resto = [v for v in voci if "level" not in v]

    resto.sort(key=lambda v: (v["frequency"] is None, v["frequency"] or 0, v["character"]))
    ultimo = max((v["level"] for v in coperti), default=0)
    for i, voce in enumerate(resto):
        voce["level"] = ultimo + 1 + i // CODA


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", help="copia locale di kanjidb.sqlite")
    parser.add_argument("--freq", help="copia locale di wikipedia_freq.txt")
    args = parser.parse_args()

    db_path = Path(args.db) if args.db else QUI / "kanjidb.sqlite"
    freq_path = Path(args.freq) if args.freq else QUI / "wikipedia_freq.txt"
    scaricati = []
    if not db_path.exists():
        scarica(DB_URL, db_path)
        scaricati.append(db_path)
    if not freq_path.exists():
        scarica(FREQ_URL, freq_path)
        scaricati.append(freq_path)

    db = sqlite3.connect(db_path)
    c = db.cursor()
    joyo = "grade LIKE 'Kyōiku-Jōyō%' OR grade LIKE 'Jōyō%'"

    # --- il corpus: conteggi veri, l'unica fonte numerica di tutto il giro ---
    corpus: dict[str, int] = {}
    for riga in freq_path.read_text(encoding="utf-8").splitlines():
        if riga.startswith("#"):
            continue
        parti = riga.split("\t")
        if len(parti) == 2 and parti[1].isdigit():
            corpus[parti[0]] = int(parti[1])

    totale = collections.Counter()
    isolato: dict[str, int] = {}
    for parola, n in corpus.items():
        for ch in set(parola):
            totale[ch] += n
        if len(parola) == 1:
            isolato[parola] = n

    # --- quali forme con okurigana sono parole che si incontrano ---
    comuni = set()
    for k, oku, fr in c.execute("SELECT kanji,okurigana,frequency FROM edict"):
        if (fr or "").rstrip("*") in ("Very common", "Common"):
            comuni.add(oku)

    # --- la lettura on dominante nei composti, pesata sulla frequenza ---
    #
    # `onyomi_statistics` risponderebbe alla stessa domanda ma conta i composti
    # distinti, non quanto si incontrano: per 生 elegge ショウ (66 composti) su セイ
    # (35), mentre i composti che si leggono davvero (生活, 学生, 先生, 発生) sono
    # tutti せい. Qui si pesa, e infatti viene セイ.
    peso_lettura: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    for k, segs, fr in c.execute("SELECT kanji,segments,frequency FROM jukugo WHERE segments<>''"):
        w = PESO.get((fr or "").rstrip("*"), 1)
        for pezzo in segs.split(";"):
            parti = pezzo.strip().split(" ")
            if len(parti) == 2 and parti[0] == k:
                peso_lettura[k][parti[1]] += w

    # --- gli esempi, i piu' comuni per primi ---
    #
    # Una stessa parola puo' comparire due volte, e per due ragioni diverse: 半年 si
    # legge sia はんとし sia はんねん, e quelle sono due voci vere; 岡山 おかやま invece
    # e' proprio ripetuto nella fonte. Le prime si tengono, le seconde no, e la
    # differenza e' se cambia anche la lettura.
    esempi: dict[str, list] = collections.defaultdict(list)
    visti: dict[str, set] = collections.defaultdict(set)
    ordine = {"Very common": 0, "Common": 1, "Uncommon": 2, "Rare": 3}
    for k, parola, lettura, sign, fr in c.execute(
        "SELECT kanji,jukugo,reading,meaning,frequency FROM jukugo"
    ):
        if (parola, lettura) in visti[k]:
            continue
        visti[k].add((parola, lettura))
        esempi[k].append((ordine.get((fr or "").rstrip("*"), 4), parola, lettura, sign))

    per_livello: dict[int, list] = collections.defaultdict(list)
    tutte: list[dict] = []

    for (kanji, strokes, wk, freq_rank, reg_on, reg_kun, nanori, meaning,
         compact) in c.execute(f"""SELECT kanji,strokes,wanikani,frequency,reg_on,reg_kun,
                                          nanori,meaning,compact_meaning
                                   FROM kanjidict WHERE {joyo}"""):
        on, on_rare = letture(reg_on)
        kun_reg, kun_rare_reg = letture(reg_kun)
        kun, kun_rare, forme = spezza_kun(kun_reg, kun_rare_reg)

        # La lettura on primaria: quella che pesa di piu' nei composti, fra quelle
        # che il kanji ha davvero. Se nessuna combacia si ripiega sulla prima, che
        # e' l'ordine convenzionale del dizionario.
        primaria = None
        if on:
            punteggi = {
                lettura: sum(p for seg, p in peso_lettura[kanji].items() if combacia(seg, lettura))
                for lettura in on
            }
            migliore = max(punteggi.values())
            primaria = next(l for l in on if punteggi[l] == migliore) if migliore else on[0]

        # I significati: la lista compatta se c'e' (230 joyo non ce l'hanno), e il
        # primo e' il primario. **Non e' un dato dichiarato**: kanjium non marca il
        # significato principale, e l'ordine e' l'unico segnale disponibile.
        significati = [m for m in (compact or meaning or "").split(";") if m]

        tot = totale.get(kanji, 0)
        voce = {
            "character": kanji,
            "strokes": strokes,
            "frequency": intero(freq_rank),
            # Quanto quel kanji ricorre da solo invece che dentro un composto.
            # `null` per i 205 joyo che il corpus non contiene.
            "aloneRatio": round(isolato.get(kanji, 0) / tot, 4) if tot else None,
            "meanings": significati,
            "on": on,
            "primaryOn": primaria,
            "kun": kun,
            "okurigana": [
                {
                    "form": kanji + coda,
                    "readings": sorted(set(letture_forma)),
                    # Se quella forma e' una parola che si incontra: la dice il
                    # corpus, o la classe di frequenza di edict. Serve a chi
                    # costruira' l'esercizio, non al contenuto: qui non si taglia.
                    "common": (kanji + coda) in comuni or corpus.get(kanji + coda, 0) > 0,
                }
                for coda, letture_forma in forme.items()
            ],
            "nanori": [n for n in (nanori or "").split("、") if n],
            "examples": [
                {"word": p, "reading": l, "meaning": s.split(";")[0]}
                for _, p, l, s in sorted(esempi.get(kanji, []))[:ESEMPI]
            ],
        }
        if on_rare:
            voce["onRare"] = on_rare
        if kun_rare:
            voce["kunRare"] = kun_rare
        # Il livello si assegna dopo, quando si sa chi resta fuori da WaniKani.
        if intero(wk) is not None:
            voce["level"] = wk
        voce["_wk"] = intero(wk)
        tutte.append(voce)

    livelli(tutte)
    for voce in tutte:
        per_livello[voce["level"]].append(voce)

    USCITA.mkdir(parents=True, exist_ok=True)
    for vecchio in USCITA.glob("level-*.json"):
        vecchio.unlink()

    totale_byte = 0
    for livello in sorted(per_livello):
        voci = per_livello[livello]
        # Dentro un livello, i piu' frequenti per primi. A parita' decide il
        # carattere, cosi' due rigenerazioni danno lo stesso file.
        voci.sort(key=lambda v: (v["frequency"] is None, v["frequency"] or 0, v["character"]))
        tabella = {
            "version": FORMAT_VERSION,
            "level": livello,
            "source": SOURCE,
            "entries": voci,
        }
        righe = ",\n".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) for v in voci)
        testa = json.dumps({k: v for k, v in tabella.items() if k != "entries"},
                           ensure_ascii=False, indent=2)[1:-1].rstrip()
        path = USCITA / f"level-{livello:02d}.json"
        path.write_text("{" + testa + ',\n  "entries": [\n' + righe + "\n]\n}\n", encoding="utf-8")
        totale_byte += path.stat().st_size

    kanji_totali = sum(len(v) for v in per_livello.values())
    print(f"  {len(per_livello)} livelli, {kanji_totali} kanji, {totale_byte / 1024:.0f} kB")
    for path in scaricati:
        path.unlink()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
