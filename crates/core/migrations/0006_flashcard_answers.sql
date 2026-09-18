-- Le risposte che una carta accetta oltre a quella principale.
--
-- # Perche' non bastavano i due campi di prima
--
-- Perche' una carta ha **una** forma scritta e **un** significato principale, ma le
-- risposte giuste possono essere piu' d'una, e in due modi diversi che non si
-- assomigliano:
--
-- - sul lato significato, una parola si traduce in piu' modi (こんにちは e' «hello» e
--   anche «good afternoon»), e sono tutte risposte buone alla stessa domanda;
-- - sul lato giapponese, chi risponde puo' scrivere la parola in kanji oppure la sua
--   lettura in kana, che e' la stessa parola detta in un altro modo.
--
-- **Il furigana lo scrive l'utente e non lo calcola l'app**, e non e' un limite: la
-- lettura di un kanji dipende dal contesto (日 e' ひ o にち, 生 e' せい, い o ふ) e la
-- stessa sequenza si legge diversamente a seconda della parola. Nessun derivatore e'
-- affidabile, e chi ha scritto la carta la lettura giusta la sa.
--
-- # Perche' una tabella e non otto colonne, ne' un campo con un separatore
--
-- **Non un separatore**, perche' si romperebbe alla prima carta che contiene quel
-- carattere, e i significati sono testo libero scritto da una persona.
--
-- **Non otto colonne**, perche' otto e' una decisione di prodotto e non una proprieta'
-- dello schema: metterla qui vorrebbe dire una migrazione per cambiarla. In una tabella
-- figlia il tetto resta una regola del dominio, dove si puo' cambiare idea.
--
-- # Perche' qui non ci sono `rev` ne' lapidi
--
-- Perche' queste righe non sono cose con una vita propria: sono un **valore della
-- carta**, come lo e' il suo significato principale. Chi cambia i significati cambia la
-- carta, e sono `flashcards.rev` e `flashcards.updated_at` a dirlo. Un sync che rimanda
-- la carta rimanda anche le sue risposte.

-- La lettura dell'intera parola o frase, quando contiene kanji. NULL quando non serve,
-- che e' cosa diversa da una stringa vuota: NULL vuol dire «questa carta non ne ha
-- bisogno», e nessuno l'ha lasciata a meta'.
--
-- **Una sola per carta.** Se una parola avesse due letture legittime sarebbero due
-- carte diverse, perche' sono due cose da imparare e non due modi di scrivere la stessa.
ALTER TABLE flashcards ADD COLUMN furigana TEXT;

-- I significati accettati oltre al primo. L'ordine e' quello in cui sono stati scritti.
CREATE TABLE flashcard_meanings (
    card_id  TEXT    NOT NULL REFERENCES flashcards (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    text     TEXT    NOT NULL,

    PRIMARY KEY (card_id, position)
) STRICT;
