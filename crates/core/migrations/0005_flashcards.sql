-- I mazzi di flashcard e le carte che contengono.
--
-- E' il primo **contenuto scritto dall'utente**: i kana e i kanji sono tabelle
-- versionate dentro il binario, generate da uno script e uguali per tutti. Queste no,
-- quindi stanno nel database dei dati utente accanto a `cards` e `answers` e non in
-- `crates/core/data`.
--
-- # Perche' non si chiamano `cards`
--
-- Perche' `cards` c'e' gia' e vuol dire un'altra cosa: lo **stato di studio** di un
-- elemento per un tipo di esercizio. Sono due cose distinte e restano distinte, una
-- cancellabile e riscrivibile, l'altra in crescita continua. Si incontrano attraverso
-- l'identificatore `flashcard:<uuid>` e non attraverso una chiave esterna, perche'
-- `cards` non sa a quale materia appartenga cio' che pianifica e non deve saperlo.
--
-- # Le lapidi
--
-- Cancellare scrive `deleted_at` invece di togliere la riga, come gia' su `cards`: un
-- sync futuro deve poter dire «questo mazzo e' stato eliminato» e non limitarsi a non
-- trovarlo piu'. Per la stessa ragione ci sono `updated_at` e `rev`.

CREATE TABLE decks (
    id         TEXT    NOT NULL PRIMARY KEY,
    name       TEXT    NOT NULL,

    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL,
    rev        INTEGER NOT NULL DEFAULT 1,
    deleted_at TEXT
) STRICT;

CREATE TABLE flashcards (
    id         TEXT    NOT NULL PRIMARY KEY,
    deck_id    TEXT    NOT NULL REFERENCES decks (id),

    -- Il lato giapponese: una parola o una frase, esattamente come l'utente l'ha
    -- scritta. Non si normalizza qui: la normalizzazione serve al confronto, e si fa
    -- al momento del confronto, altrimenti si perderebbe la forma originale senza
    -- poterla piu' rileggere.
    japanese   TEXT    NOT NULL,
    -- Il significato, nella lingua con cui l'utente studia. Non c'e' nessun campo che
    -- dica quale sia: il testo e' libero, e la lingua di riferimento la conosce solo
    -- chi l'ha scritto.
    meaning    TEXT    NOT NULL,

    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL,
    rev        INTEGER NOT NULL DEFAULT 1,
    deleted_at TEXT
) STRICT;

-- Le carte si cercano sempre per mazzo.
CREATE INDEX flashcards_deck ON flashcards (deck_id) WHERE deleted_at IS NULL;
