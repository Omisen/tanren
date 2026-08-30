-- Toglie il livello dagli identificatori dei kanji: `kanji:1:一` diventa `kanji:一`.
--
-- # Perche'
--
-- Il livello non fa parte dell'identita' di un kanji: e' una decisione editoriale su
-- dove metterlo nel percorso, e le decisioni cambiano. Un identificatore invece finisce
-- nell'archivio e ci resta per sempre.
--
-- Averlo dentro voleva dire che riordinare i livelli orfanava lo storico, **in
-- silenzio**, perche' una carta che non si risolve viene ignorata senza dire niente.
-- Misurato sul riordino che ha sganciato il percorso da una lista proprietaria: ha
-- spostato di livello il **97% dei kanji**. Con uno storico sparso sarebbe sparito
-- quasi tutto.
--
-- # Cosa fa
--
-- Riscrive `item_id` nelle due tabelle che lo contengono, togliendo il segmento in
-- mezzo. Il livello si chiede adesso a un indice generato insieme al contenuto.
--
-- Tocca solo le righe dei kanji: **i kana non c'entrano**, e il loro identificatore ha
-- la stessa forma a tre segmenti ma per un'altra ragione, cioe' il sillabario, che e'
-- una proprieta' del carattere e non una nostra scelta.
--
-- # Perche' una migrazione per cinque righe
--
-- Perche' e' il meccanismo che il progetto ha gia' per i cambi di schema, si applica da
-- sola a ogni apertura e vale su qualunque copia del database, non solo su quella dove
-- e' stato fatto il cambio. Farlo a mano non lascerebbe traccia e varrebbe una volta
-- sola. Le righe oggi sono cinque; il punto e' che il modo di trattarle resti scritto.

UPDATE cards
SET item_id = 'kanji:' || substr(item_id, 6 + instr(substr(item_id, 7), ':') + 1),
    updated_at = updated_at,
    rev = rev + 1
WHERE exercise_type LIKE 'kanji.%'
  AND item_id LIKE 'kanji:%'
  AND instr(substr(item_id, 7), ':') > 0;

UPDATE answers
SET item_id = 'kanji:' || substr(item_id, 6 + instr(substr(item_id, 7), ':') + 1)
WHERE exercise_type LIKE 'kanji.%'
  AND item_id LIKE 'kanji:%'
  AND instr(substr(item_id, 7), ':') > 0;
