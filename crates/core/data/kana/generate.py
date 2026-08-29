# -*- coding: utf-8 -*-
"""Genera le tabelle kana. L'hiragana e' scritto qui, il katakana e' derivato:
i due sillabari sono allineati in Unicode a distanza 0x60."""
import json

# (riga, [(kana, [romaji accettati, il primo e' quello canonico])])
BASE = [
    ("a",  [("あ", ["a"]), ("い", ["i"]), ("う", ["u"]), ("え", ["e"]), ("お", ["o"])]),
    ("ka", [("か", ["ka"]), ("き", ["ki"]), ("く", ["ku"]), ("け", ["ke"]), ("こ", ["ko"])]),
    ("sa", [("さ", ["sa"]), ("し", ["shi", "si"]), ("す", ["su"]), ("せ", ["se"]), ("そ", ["so"])]),
    ("ta", [("た", ["ta"]), ("ち", ["chi", "ti"]), ("つ", ["tsu", "tu"]), ("て", ["te"]), ("と", ["to"])]),
    ("na", [("な", ["na"]), ("に", ["ni"]), ("ぬ", ["nu"]), ("ね", ["ne"]), ("の", ["no"])]),
    ("ha", [("は", ["ha"]), ("ひ", ["hi"]), ("ふ", ["fu", "hu"]), ("へ", ["he"]), ("ほ", ["ho"])]),
    ("ma", [("ま", ["ma"]), ("み", ["mi"]), ("む", ["mu"]), ("め", ["me"]), ("も", ["mo"])]),
    ("ya", [("や", ["ya"]), ("ゆ", ["yu"]), ("よ", ["yo"])]),
    ("ra", [("ら", ["ra"]), ("り", ["ri"]), ("る", ["ru"]), ("れ", ["re"]), ("ろ", ["ro"])]),
    ("wa", [("わ", ["wa"]), ("を", ["wo", "o"])]),
    ("n",  [("ん", ["n", "nn"])]),
]

DAKUTEN = [
    ("ga", [("が", ["ga"]), ("ぎ", ["gi"]), ("ぐ", ["gu"]), ("げ", ["ge"]), ("ご", ["go"])]),
    ("za", [("ざ", ["za"]), ("じ", ["ji", "zi"]), ("ず", ["zu"]), ("ぜ", ["ze"]), ("ぞ", ["zo"])]),
    ("da", [("だ", ["da"]), ("ぢ", ["ji", "di", "dzi"]), ("づ", ["zu", "du", "dzu"]), ("で", ["de"]), ("ど", ["do"])]),
    ("ba", [("ば", ["ba"]), ("び", ["bi"]), ("ぶ", ["bu"]), ("べ", ["be"]), ("ぼ", ["bo"])]),
]

HANDAKUTEN = [
    ("pa", [("ぱ", ["pa"]), ("ぴ", ["pi"]), ("ぷ", ["pu"]), ("ぺ", ["pe"]), ("ぽ", ["po"])]),
]

# (riga, kana base in -i, prefissi romaji per ゃ ゅ ょ)
YOON = [
    ("ka", "き", [["kya"], ["kyu"], ["kyo"]]),
    ("ga", "ぎ", [["gya"], ["gyu"], ["gyo"]]),
    ("sa", "し", [["sha", "sya"], ["shu", "syu"], ["sho", "syo"]]),
    ("za", "じ", [["ja", "zya", "jya"], ["ju", "zyu", "jyu"], ["jo", "zyo", "jyo"]]),
    ("ta", "ち", [["cha", "tya"], ["chu", "tyu"], ["cho", "tyo"]]),
    ("da", "ぢ", [["ja", "dya"], ["ju", "dyu"], ["jo", "dyo"]]),
    ("na", "に", [["nya"], ["nyu"], ["nyo"]]),
    ("ha", "ひ", [["hya"], ["hyu"], ["hyo"]]),
    ("ba", "び", [["bya"], ["byu"], ["byo"]]),
    ("pa", "ぴ", [["pya"], ["pyu"], ["pyo"]]),
    ("ma", "み", [["mya"], ["myu"], ["myo"]]),
    ("ra", "り", [["rya"], ["ryu"], ["ryo"]]),
]
SMALL = ["ゃ", "ゅ", "ょ"]


def entries():
    out = []
    for group, table in (("base", BASE), ("dakuten", DAKUTEN), ("handakuten", HANDAKUTEN)):
        for row, kana in table:
            for ch, romaji in kana:
                out.append({"character": ch, "romaji": romaji, "group": group, "row": row})
    for row, base, romaji_sets in YOON:
        for small, romaji in zip(SMALL, romaji_sets):
            out.append({"character": base + small, "romaji": romaji, "group": "yoon", "row": row})
    return out


def to_katakana(text):
    return "".join(chr(ord(c) + 0x60) for c in text)


def write(path, syllabary, rows):
    doc = {
        "version": 1,
        "syllabary": syllabary,
        "entries": rows,
    }
    with open(path, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"{path}: {len(rows)} voci")


hira = entries()
kata = [dict(e, character=to_katakana(e["character"])) for e in hira]
write("crates/core/data/kana/hiragana.json", "hiragana", hira)
write("crates/core/data/kana/katakana.json", "katakana", kata)
