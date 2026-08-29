-- Schema iniziale dei dati utente.
--
-- Ogni tabella e' pensata perche' un sync futuro possa essere aggiunto senza
-- riscrivere lo schema. Servono tre cose: un'identita' valida su piu' dispositivi,
-- un modo per accorgersi che una riga e' cambiata, e le lapidi per le righe
-- cancellate. Dove una di queste non serve, il motivo e' scritto qui sotto.

-- Lo stato di studio di un elemento per un tipo di esercizio.
--
-- Lo stesso kana studiato in riconoscimento e in input sono due carte distinte: si
-- imparano in tempi diversi e vanno pianificate separatamente.
CREATE TABLE cards (
    item_id          TEXT    NOT NULL,
    exercise_type    TEXT    NOT NULL,

    -- Pianificazione. Queste colonne valgono per qualunque algoritmo di ripetizione;
    -- lo stato specifico di FSRS arrivera' con una migrazione dedicata.
    due_at           TEXT,             -- NULL: mai studiata, quindi dovuta subito
    last_reviewed_at TEXT,
    reps             INTEGER NOT NULL DEFAULT 0,
    lapses           INTEGER NOT NULL DEFAULT 0,

    -- Metadati per il sync.
    -- La chiave e' gia' valida ovunque, perche' non e' un numero progressivo ma il
    -- nome dell'elemento piu' quello dell'esercizio: due dispositivi che studiano lo
    -- stesso kana producono la stessa chiave e le loro righe si possono confrontare.
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL,
    rev              INTEGER NOT NULL DEFAULT 1,
    deleted_at       TEXT,

    PRIMARY KEY (item_id, exercise_type)
) STRICT;

-- Le carte da studiare adesso si cercano per scadenza.
CREATE INDEX cards_due_at ON cards (due_at) WHERE deleted_at IS NULL;

-- Ogni risposta data, in ordine di tempo.
--
-- La tabella e' in sola aggiunta: una risposta gia' data non cambia e non si
-- cancella. Per questo non ha ne' `updated_at` ne' `rev` ne' lapidi: sarebbero
-- colonne che non cambiano mai. L'identita' e' un UUID versione 7, che contiene
-- l'istante di creazione, quindi e' unico tra dispositivi e allo stesso tempo
-- ordinabile: un sync futuro puo' chiedere "tutto quello che viene dopo questo id"
-- senza altre colonne di appoggio.
CREATE TABLE answers (
    id            TEXT    NOT NULL PRIMARY KEY,
    item_id       TEXT    NOT NULL,
    exercise_type TEXT    NOT NULL,
    answered_at   TEXT    NOT NULL,
    correct       INTEGER NOT NULL,
    -- Cosa ha risposto l'utente, cosi' si puo' rivedere l'errore invece di sapere
    -- soltanto che c'e' stato.
    answer        TEXT    NOT NULL
) STRICT;

CREATE INDEX answers_card ON answers (item_id, exercise_type, answered_at);
