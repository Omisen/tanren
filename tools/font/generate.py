# -*- coding: utf-8 -*-
"""Ritaglia il font giapponese sui soli caratteri che l'app puo' mostrare.

M PLUS Rounded 1c intero pesa 3,3 MB. Imbarcarlo tutto significherebbe portarsi
dietro migliaia di kanji per un'app che oggi mostra due sillabari. Ritagliato
sui caratteri veri sta in poche decine di kB.

Va rieseguito quando l'insieme dei caratteri cambia, cioe' quando arriveranno i
kanji: allora COPERTURA cresce e il file si rifa'. Non e' un passo della build,
si lancia a mano come `crates/core/data/kana/generate.py`.

    pip install fonttools brotli
    python tools/font/generate.py

Il file prodotto e' **versionato**, cosi' chi clona compila senza dover prima
procurarsi un font, per la stessa ragione per cui sqlx usa l'API a runtime.
"""
import subprocess
import sys
import urllib.request
from pathlib import Path

RADICE = Path(__file__).resolve().parents[2]
USCITA = RADICE / "public" / "fonts"

# M PLUS Rounded 1c, SIL Open Font License 1.1.
#
# Scelto fra quattro candidati, con due criteri che hanno deciso da soli:
#
# 1. **I segni di sonorizzazione combinanti** `U+3099` e `U+309A`. Il campo di
#    risposta mostra il testo grezzo prima della normalizzazione, e un IME puo'
#    produrre `か` + `゙` staccati: senza quei glifi si vedrebbe un rettangolo
#    vuoto proprio nel caso che l'anteprima esiste per gestire. Zen Maru Gothic
#    e Klee One non li hanno e sono caduti qui.
# 2. **Il tono** chiesto dalla regola 3 della sezione 4 di CLAUDE.md, friendly e
#    arrotondato e non clinico. Fra i due superstiti Noto Sans JP e' il neutro,
#    questo e' l'arrotondato. Regge anche meglio in bianco su un blocco a tinta
#    piena, perche' ha i tratti un filo piu' pieni, ed e' un terzo del peso.
NOME = "MPLUSRounded1c"
SORGENTE = (
    "https://github.com/google/fonts/raw/main/ofl/mplusrounded1c/"
    "MPLUSRounded1c-Regular.ttf"
)
# La cartella di google/fonts per questo font **non contiene** il suo `OFL.txt`:
# e' una lacuna del loro repository, non un dubbio sulla licenza. Il `METADATA.pb`
# dichiara `license: "OFL"`, e la tabella dei nomi dentro il binario dice
# «This Font Software is licensed under the SIL Open Font License, Version 1.1».
# Quindi il testo canonico si prende da un'altra cartella dello stesso repository
# e gli si rimette in testa il copyright di questo font, che e' l'unica riga
# specifica del file.
LICENZA_CANONICA = (
    "https://github.com/google/fonts/raw/main/ofl/notosansjp/OFL.txt"
)
COPYRIGHT = "Copyright 2016 The Rounded M+ Project Authors."

# Cosa l'app puo' davvero mostrare in `font-jp`:
#   latino di base   il campo di risposta ha font-jp e ci si digita romaji prima
#                    che l'IME converta
#   U+3000-30FF      punteggiatura CJK, hiragana, katakana, i segni di
#                    sonorizzazione e il prolungamento
#   U+31F0-31FF      piccoli katakana di estensione
#   U+FF01-FF9F      forme a piena larghezza e katakana a mezza larghezza, che un
#                    IME puo' restituire e che si vedono prima della
#                    normalizzazione
COPERTURA = "U+0020-007E,U+3000-30FF,U+31F0-31FF,U+FF01-FF9F"


def scarica(url: str, dove: Path) -> None:
    print(f"  scarico {dove.name}")
    with urllib.request.urlopen(url) as r:
        dove.write_bytes(r.read())


def main() -> int:
    try:
        import fontTools  # noqa: F401
        import brotli  # noqa: F401
    except ImportError:
        print("servono fonttools e brotli:  pip install fonttools brotli")
        return 1

    USCITA.mkdir(parents=True, exist_ok=True)
    intero = USCITA / f"{NOME}-intero.ttf"
    ritagliato = USCITA / f"{NOME}-kana.woff2"

    scarica(SORGENTE, intero)

    # La OFL chiede che la licenza accompagni il font, anche ritagliato.
    print("  compongo OFL.txt")
    with urllib.request.urlopen(LICENZA_CANONICA) as r:
        testo = r.read().decode("utf-8")
    righe = testo.split("\n")
    assert righe[0].startswith("Copyright"), "la prima riga non e' il copyright"
    righe[0] = COPYRIGHT
    (USCITA / "OFL.txt").write_text("\n".join(righe), encoding="utf-8")

    print("  ritaglio")
    subprocess.run(
        [
            sys.executable, "-m", "fontTools.subset", str(intero),
            f"--unicodes={COPERTURA}",
            "--flavor=woff2",
            f"--output-file={ritagliato}",
            "--layout-features=*",
            "--no-hinting",
            "--desubroutinize",
        ],
        check=True,
    )

    # Il .ttf intero e' solo materia prima: non va imbarcato nell'app, e
    # `public/` finisce tutto dentro il binario.
    intero.unlink()

    prima = 3.3 * 1024
    dopo = ritagliato.stat().st_size / 1024
    print(f"\n  {ritagliato.relative_to(RADICE)}: {dopo:.1f} kB "
          f"(da ~{prima / 1024:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
