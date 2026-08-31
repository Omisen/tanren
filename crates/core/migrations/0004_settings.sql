-- Le preferenze dell'utente, come coppie chiave-valore.
--
-- E' la prima cosa che l'app ricorda oltre allo studio, e sta nel database e non nel
-- frontend per la regola che il progetto si e' gia' dato: nello store del frontend ci
-- va solo cio' che si puo' perdere chiudendo l'app, e una preferenza no.
--
-- Chiave-valore e non una colonna per preferenza: le preferenze nascono una alla volta
-- e ognuna vorrebbe la sua migrazione, che e' molto rumore per un intero. Il valore e'
-- TEXT perche' SQLite non ha un tipo somma, e chi legge sa cosa aspettarsi dalla
-- propria chiave; una preferenza scritta male vale come assente, quindi si ricade sul
-- default invece di rompere l'avvio.
--
-- `updated_at` c'e' per la stessa ragione per cui sta su `cards`: un domani un sync
-- deve poter dire quale delle due copie e' la piu' recente.
CREATE TABLE settings (
    key        TEXT NOT NULL PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
