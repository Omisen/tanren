# -*- coding: utf-8 -*-
"""Genera le tabelle kana.

L'hiragana e' scritto qui e il katakana e' derivato: i due sillabari sono allineati in
Unicode a distanza 0x60.

**Il katakana ha pero' una famiglia in piu'**, i 外来音, cioe' i suoni presi da altre
lingue. Quelli l'hiragana non ce li ha, quindi non si derivano da niente e sono scritti
a mano qui sotto: e' l'unico punto in cui i due sillabari smettono di essere la stessa
tabella in due grafie.

**Scrive anche un terzo file, `patterns.json`**, che non e' una tabella di segni ma due
regole di scrittura da mostrare e basta: il sokuon, che raddoppia la consonante dopo di
se', e le vocali lunghe. Sta qui perche' il contenuto dei kana deve avere un punto
d'autore solo, ma non entra nelle tabelle: quelle le interroga il motore d'esercizio, e
queste due cose non si chiedono.
"""
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

# I 外来音: i suoni che il giapponese non aveva e si e' preso da fuori. Esistono **solo
# in katakana**, quindi questa tabella non viene derivata dall'hiragana.
#
# La trascrizione e' quella che l'IME vuole per **produrre** il segno, e per cinque di
# questi non e' il suono ingenuo: `ti` da' ち, non ティ, e `di` `du` `wo` sono per di
# piu' gia' presi da ヂ ヅ ヲ. Le forme giuste sono `thi` `dhi` `twu` `dwu` `who`, e
# sono giuste due volte: tolgono l'ambiguita' e sono quello che si digita davvero.
#
# Le righe sono **proprie** e non prese in prestito da quelle esistenti. La tabella
# separa gia' le righe sonorizzate da quelle sorde (`ga` non sta dentro `ka`), quindi
# infilare ファ dentro `ha` andrebbe contro la sua stessa regola.
GAIRAION = [
    ("fa",  [("ファ", ["fa"]), ("フィ", ["fi"]), ("フェ", ["fe"]), ("フォ", ["fo"])]),
    ("va",  [("ヴァ", ["va"]), ("ヴィ", ["vi"]), ("ヴ", ["vu"]), ("ヴェ", ["ve"]), ("ヴォ", ["vo"])]),
    ("thi", [("ティ", ["thi"]), ("トゥ", ["twu"])]),
    ("dhi", [("ディ", ["dhi"]), ("ドゥ", ["dwu"])]),
    ("wi",  [("ウィ", ["wi"]), ("ウェ", ["we"]), ("ウォ", ["who"])]),
    ("che", [("チェ", ["che"])]),
    ("je",  [("ジェ", ["je"])]),
    ("she", [("シェ", ["she"])]),
]


# --- I due gruppi di sola consultazione -------------------------------------------
#
# Non sono segni con una lettura isolata, sono **regole**: っ raddoppia la consonante
# che segue, e le vocali lunghe allungano. Il motore d'esercizio chiede «che lettura ha
# questo segno», e a queste due quella domanda non si puo' porre, quindi stanno fuori
# dalle tabelle e finiscono in un file loro.
#
# La consonante si scrive in latino di proposito: il sokuon non e' un carattere che si
# legge, e `っ+k` dice la regola meglio di qualunque segno.
DOUBLE = [("+k", "kk"), ("+s", "ss"), ("+t", "tt"), ("+p", "pp")]

# **La colonna qui e' scritta e non ricavata dal romaji**, a differenza delle tabelle.
# Li' la colonna e' l'ultima lettera della trascrizione, ed e' vero per costruzione;
# qui no: えい e' una **e** lunga ma la sua trascrizione finisce per `i`, e おう e' una
# **o** lunga che finisce per `u`. Ricavarla le metterebbe nelle colonne sbagliate.
LONG_HIRAGANA = [
    [("ああ", "aa", "a"), ("いい", "ii", "i"), ("うう", "uu", "u"),
     ("ええ", "ee", "e"), ("おお", "oo", "o")],
    [("えい", "ei", "e"), ("おう", "ou", "o")],
]

# **Il katakana non si deriva, e non e' pigrizia del generatore.** La vocale lunga in
# katakana si scrive col choonpu ー, non raddoppiando: アア sarebbe ortograficamente
# sbagliato, e un gruppo che esiste per fare da riferimento grammaticale non puo'
# mostrare una forma scorretta. Anche la seconda riga sparisce, perche' えい e おう sono
# convenzioni dell'hiragana e in katakana non esistono: inventarle sarebbe lo stesso
# errore detto in un altro modo.
LONG_KATAKANA = [
    [("アー", "aa", "a"), ("イー", "ii", "i"), ("ウー", "uu", "u"),
     ("エー", "ee", "e"), ("オー", "oo", "o")],
]


def double(sokuon):
    return [[{"character": sokuon + tail, "romaji": r, "column": None} for tail, r in DOUBLE]]


def long_rows(table):
    return [
        [{"character": c, "romaji": r, "column": col} for c, r, col in row]
        for row in table
    ]


def patterns(sokuon, long_table):
    return [
        {"group": "double", "rows": double(sokuon)},
        {"group": "long", "rows": long_rows(long_table)},
    ]


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


def estesi():
    """I 外来音, che esistono solo in katakana e quindi non si derivano."""
    out = []
    for row, kana in GAIRAION:
        for ch, romaji in kana:
            out.append({"character": ch, "romaji": romaji, "group": "gairaion", "row": row})
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
kata = [dict(e, character=to_katakana(e["character"])) for e in hira] + estesi()
write("crates/core/data/kana/hiragana.json", "hiragana", hira)
write("crates/core/data/kana/katakana.json", "katakana", kata)

doc = {
    "version": 1,
    "hiragana": patterns("っ", LONG_HIRAGANA),
    "katakana": patterns("ッ", LONG_KATAKANA),
}
with open("crates/core/data/kana/patterns.json", "w", encoding="utf-8") as f:
    json.dump(doc, f, ensure_ascii=False, indent=2)
    f.write("\n")
print("crates/core/data/kana/patterns.json: 2 gruppi per sillabario")
