//! Persistenza dei dati utente su SQLite, tramite sqlx.
//!
//! Si usa l'API runtime di sqlx (`query` e `query_as` con binding), non le macro
//! verificate a compile-time: la build deve funzionare su un repo appena clonato,
//! senza un database gia' presente.
//!
//! Ogni riga di dato utente porta con se' timestamp e versione, cosi' un
//! eventuale sync futuro non richiede di rifare lo schema.

// Da definire nello step 6: connessione, migrazioni e repository.
