# -*- coding: utf-8 -*-
"""Genera le tabelle dei kanji joyo a partire da KANJIDIC2.

Le tabelle non si modificano a mano: si cambia questo script e si rigenera, come
per i kana. La differenza e' che qui il dato non e' scritto qui dentro ma deriva
da una fonte esterna, quindi ogni file porta la versione di KANJIDIC2 da cui e'
uscito: senza, un domani non si saprebbe piu' quale edizione del dizionario si sta
studiando.

    python3 generate.py                     scarica KANJIDIC2 e rigenera
    python3 generate.py --source FILE       usa una copia locale (.xml o .xml.gz)

KANJIDIC2 e' dell'Electronic Dictionary Research and Development Group, distribuito
in CC BY-SA 4.0. Anche i file prodotti qui lo sono, perche' la licenza si estende
esplicitamente ai dati derivati. Vedi ATTRIBUTION.md in questa cartella.
"""
import argparse
import gzip
import json
import sys
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path

SOURCE_URL = "http://www.edrdg.org/kanjidic/kanjidic2.xml.gz"

# I gradi di KANJIDIC2: da 1 a 6 sono gli anni della scuola elementare (i kyoiku),
# l'8 sono i joyo restanti, insegnati alle medie e alle superiori. Il 7 non esiste,
# e il 9 e il 10 sono i jinmeiyo, i kanji ammessi solo nei nomi di persona: fuori
# da qui, perche' non si studiano per leggere.
GRADES = {
    1: "first",
    2: "second",
    3: "third",
    4: "fourth",
    5: "fifth",
    6: "sixth",
    8: "secondary",
}

# Versione del formato di questi file, non del dizionario: cambia se cambia la forma
# di una voce. La versione della fonte viaggia a parte, nel campo `source`.
FORMAT_VERSION = 1

LICENCE = "CC BY-SA 4.0"
PROJECT_URL = "https://www.edrdg.org/wiki/index.php/KANJIDIC_Project"


def load(source: str | None) -> ET.Element:
    """L'albero XML di KANJIDIC2, da una copia locale o dalla rete."""
    if source is None:
        print(f"scarico {SOURCE_URL}", file=sys.stderr)
        with urllib.request.urlopen(SOURCE_URL) as response:
            raw = gzip.decompress(response.read())
    else:
        data = Path(source).read_bytes()
        raw = gzip.decompress(data) if source.endswith(".gz") else data
    return ET.fromstring(raw)


def readings(character: ET.Element) -> tuple[list[str], list[str], list[str]]:
    """Letture on, letture kun e significati inglesi, **come li da' KANJIDIC2**.

    Non si ripulisce niente, di proposito. Le letture kun portano il punto che separa
    la parte scritta col kanji dall'okurigana (`い.きる`) e il trattino che segna i
    prefissi e i suffissi (`-り`, `なま-`): sono informazione, non sporcizia, e
    toglierla qui vorrebbe dire decidere nel posto sbagliato come si formula una
    domanda. Il dato resta fedele e l'esercizio sceglie cosa farne.
    """
    on: list[str] = []
    kun: list[str] = []
    meanings: list[str] = []

    rm = character.find("reading_meaning")
    if rm is None:
        return on, kun, meanings
    group = rm.find("rmgroup")
    if group is None:
        return on, kun, meanings

    for reading in group.findall("reading"):
        kind = reading.get("r_type")
        if kind == "ja_on":
            on.append(reading.text)
        elif kind == "ja_kun":
            kun.append(reading.text)

    # Senza `m_lang` il significato e' quello inglese. Le altre lingue che KANJIDIC2
    # porta (spagnolo, francese, portoghese) non servono: l'italiano non c'e', quindi
    # la scelta della lingua va comunque affrontata quando i significati verranno
    # usati davvero. Vedi ATTRIBUTION.md e CLAUDE.md.
    meanings = [m.text for m in group.findall("meaning") if m.get("m_lang") is None]

    # I nanori, le letture che un kanji prende solo nei nomi propri, restano fuori:
    # non si studiano per leggere e allargherebbero le letture accettate con roba che
    # in un testo non compare.
    return on, kun, meanings


def entry(character: ET.Element) -> dict:
    misc = character.find("misc")
    frequency = misc.findtext("freq")
    on, kun, meanings = readings(character)
    return {
        "character": character.findtext("literal"),
        "strokes": int(misc.find("stroke_count").text),
        # Il rango di frequenza sui giornali, da 1 a 2501. Manca sui kanji fuori dai
        # 2500 piu' comuni, che pure sono joyo.
        "frequency": int(frequency) if frequency else None,
        "on": on,
        "kun": kun,
        "meanings": meanings,
    }


def render(table: dict) -> str:
    """Una voce per riga, compatta.

    Il file e' generato, ma va comunque letto dagli umani e soprattutto **diffato**:
    quando KANJIDIC2 cambia si deve poter vedere quali kanji sono cambiati. Tutto su
    una riga sola darebbe una riga cambiata, e con l'indentazione piena i file
    triplicherebbero.
    """
    entries = ",\n".join(
        json.dumps(e, ensure_ascii=False, separators=(",", ":")) for e in table["entries"]
    )
    head = {k: v for k, v in table.items() if k != "entries"}
    lines = [json.dumps(head, ensure_ascii=False, indent=2)[1:-1].rstrip()]
    return "{" + lines[0] + ",\n  \"entries\": [\n" + entries + "\n]\n}\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", help="copia locale di kanjidic2.xml o .xml.gz")
    args = parser.parse_args()

    root = load(args.source)
    header = root.find("header")
    source = {
        "dataset": "KANJIDIC2",
        # La versione del dizionario, nella forma anno-progressivo. E' l'unica cosa
        # che dice davvero quale edizione del dato si sta studiando.
        "databaseVersion": header.findtext("database_version"),
        "created": header.findtext("date_of_creation"),
        "licence": LICENCE,
        "url": PROJECT_URL,
    }

    per_grade: dict[int, list[dict]] = {g: [] for g in GRADES}
    for character in root.findall("character"):
        grade = character.find("misc").findtext("grade")
        if grade is None:
            continue
        grade = int(grade)
        if grade not in per_grade:
            continue
        per_grade[grade].append(entry(character))

    here = Path(__file__).parent
    for grade, name in GRADES.items():
        entries = per_grade[grade]
        # In ordine di frequenza, i piu' comuni per primi, e in coda quelli che un
        # rango non ce l'hanno. Cosi' "i primi N di questo grado" e' un prefisso della
        # lista invece di un filtro, e l'ordine resta stabile fra due rigenerazioni
        # perche' a parita' di rango decide il carattere.
        entries.sort(key=lambda e: (e["frequency"] is None, e["frequency"] or 0, e["character"]))
        table = {
            "version": FORMAT_VERSION,
            "grade": name,
            "source": source,
            "entries": entries,
        }
        path = here / f"{name}.json"
        path.write_text(render(table), encoding="utf-8")
        print(f"{path.name}: {len(entries)} kanji, {path.stat().st_size / 1024:.0f} kB")


if __name__ == "__main__":
    main()
