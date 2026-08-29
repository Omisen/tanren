-- Stato di FSRS sulle carte, e valutazione sulle risposte.
--
-- Sta in una migrazione a parte perche' lo schema iniziale e' stato scritto prima di
-- scegliere l'algoritmo di ripetizione: le colonne che valgono per qualunque
-- algoritmo stanno in 0001, quelle che parlano la lingua di FSRS stanno qui.

-- FSRS descrive la memoria con due numeri: quanto a lungo il ricordo regge, e quanto
-- l'elemento e' faticoso per questa persona. Sono NULL finche' la carta non e' mai
-- stata studiata.
ALTER TABLE cards ADD COLUMN stability REAL;
ALTER TABLE cards ADD COLUMN difficulty REAL;

-- La valutazione data alla risposta, da 1 (di nuovo) a 4 (facile).
--
-- E' nullable e non ha un default: mettere un valore inventato sulle righe gia'
-- scritte significherebbe raccontare a FSRS una storia che non e' successa, e questa
-- tabella e' proprio quella su cui un domani si riaddestrano i parametri.
ALTER TABLE answers ADD COLUMN rating INTEGER;
