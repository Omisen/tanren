//! I mazzi e le carte: cosa sono e cosa se ne puo' fare.
//!
//! # Perche' le query stanno qui e non in `shared::storage`
//!
//! Perche' `decks` e `flashcards` sono tabelle di **questa** materia, e il livello
//! condiviso non deve sapere cosa sia un mazzo. Quello che sta in `shared` e' la
//! connessione e lo schema, non ogni tabella che ci vive dentro: la regola di
//! dipendenza vale anche per il database.
//!
//! # Cosa e' contenuto e cosa e' studio
//!
//! Qui c'e' solo il **contenuto**: il testo delle due facce e a quale mazzo
//! appartiene. Lo stato di studio di una carta vive in `cards`, che e' di tutte le
//! materie, e le due cose si incontrano attraverso [`item_id`]. Sono separate perche'
//! hanno vite diverse: una carta si riscrive e si cancella, uno storico no.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::ItemId;
use crate::shared::storage::{CardFilter, Database};

/// Un mazzo, come lo vede chi lo ha creato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub id: String,
    pub name: String,
}

/// Un mazzo nell'elenco, con quante carte contiene.
///
/// Il conteggio sta qui e non in [`Deck`] perche' e' una cosa che si calcola e non una
/// proprieta' del mazzo: chi ha appena rinominato un mazzo non ha bisogno di sapere
/// quante carte ha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DeckSummary {
    pub id: String,
    pub name: String,
    pub cards: i64,
}

/// Quanti significati si accettano oltre a quello principale.
///
/// E' una decisione di prodotto e sta qui e non nello schema: in una tabella figlia il
/// tetto si puo' cambiare idea senza una migrazione. L'interfaccia lo riceve da qui
/// invece di averne una copia, perche' due verita' prima o poi si sganciano.
pub const MAX_ALTERNATIVES: usize = 8;

/// Una carta: il giapponese da una parte, il significato dall'altra.
///
/// Il testo e' quello che l'utente ha scritto, non normalizzato. La normalizzazione
/// serve al confronto e si fa al momento del confronto: farla qui vorrebbe dire
/// perdere la forma originale senza poterla piu' rileggere.
///
/// # Perche' le risposte in piu' sono due campi e non uno
///
/// Perche' servono a **due domande diverse**, e non sono intercambiabili: gli altri
/// significati valgono quando si risponde col significato, il furigana quando si
/// risponde in giapponese. Tenerli insieme vorrebbe dire accettare una traduzione dove
/// si chiede una parola giapponese.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flashcard {
    pub id: String,
    pub deck_id: String,
    pub japanese: String,
    /// Il significato principale, quello che si mostra.
    pub meaning: String,
    /// Gli altri significati accettati, nell'ordine in cui sono stati scritti.
    pub alternatives: Vec<String>,
    /// La lettura dell'intera parola o frase, quando contiene kanji.
    ///
    /// **Una sola.** Se una parola avesse due letture legittime sarebbero due carte
    /// diverse, perche' sono due cose da imparare e non due modi di scrivere la stessa.
    /// La scrive chi crea la carta: la lettura di un kanji dipende dal contesto, e
    /// nessun derivatore automatico e' affidabile.
    pub furigana: Option<String>,
}

/// Cosa c'e' scritto su una carta, come arriva da chi la sta scrivendo.
///
/// Sta insieme perche' e' una cosa sola, e perche' crearla e correggerla vogliono
/// esattamente gli stessi campi: due firme con quattro stringhe in fila sarebbero due
/// posti in cui scambiarne due senza che se ne accorga nessuno.
#[derive(Debug, Clone, Copy)]
pub struct Content<'a> {
    pub japanese: &'a str,
    pub meaning: &'a str,
    /// Gli altri significati. Le caselle lasciate vuote non sono risposte e cadono.
    pub alternatives: &'a [String],
    /// La lettura. Vuota vale come assente.
    pub furigana: &'a str,
}

/// La riga come sta nell'archivio, prima che le si accostino i suoi significati.
#[derive(Debug, Clone, FromRow)]
struct Row {
    id: String,
    deck_id: String,
    japanese: String,
    meaning: String,
    furigana: Option<String>,
}

/// L'identificatore di studio di una carta.
///
/// Due segmenti e basta, `flashcard:<uuid>`: **il mazzo non ci entra**. Un
/// identificatore finisce nell'archivio e ci resta per sempre, mentre il mazzo e' una
/// decisione che l'utente puo' cambiare; scrivercelo dentro vorrebbe dire orfanare lo
/// storico il giorno in cui una carta cambia mazzo. E' la stessa lezione che i kanji
/// hanno imparato col livello, e qui si applica prima di sbagliare invece che dopo.
pub fn item_id(card: &str) -> ItemId {
    ItemId::new(format!("flashcard:{card}"))
}

/// Tutti i mazzi, in ordine alfabetico, con quante carte contengono.
///
/// In ordine di nome e non di creazione perche' un elenco si scorre cercando qualcosa,
/// e l'ordine in cui i mazzi sono nati non aiuta a trovarlo. `COLLATE NOCASE` in SQLite
/// pareggia le sole lettere latine: basta a non separare «verbi» da «Verbi», e su un
/// nome giapponese l'ordine resta quello dei punti di codice, che e' comunque stabile.
pub async fn decks(db: &Database) -> Result<Vec<DeckSummary>> {
    let decks = sqlx::query_as::<_, DeckSummary>(
        "SELECT d.id, d.name, count(f.id) AS cards
         FROM decks d
         LEFT JOIN flashcards f ON f.deck_id = d.id AND f.deleted_at IS NULL
         WHERE d.deleted_at IS NULL
         GROUP BY d.id, d.name
         ORDER BY d.name COLLATE NOCASE ASC, d.created_at ASC",
    )
    .fetch_all(db.pool())
    .await?;

    Ok(decks)
}

/// Crea un mazzo. Serve solo il nome: cosa ci va dentro si decide dopo.
///
/// Due mazzi possono chiamarsi uguale, e non e' una svista: l'identita' e' l'UUID, e
/// rifiutare un nome ripetuto vorrebbe dire impedire a chi sta riorganizzando i propri
/// mazzi di averne due «N5» per il tempo di spostare le carte.
pub async fn create_deck(db: &Database, name: &str, now: DateTime<Utc>) -> Result<Deck> {
    let name = required("name", name)?;
    // UUID versione 7: porta dentro l'istante di creazione, quindi e' unico fra
    // dispositivi e ordinabile, che e' la stessa ragione per cui lo usa `answers`.
    let id = Uuid::now_v7().to_string();

    sqlx::query(
        "INSERT INTO decks (id, name, created_at, updated_at, rev)
         VALUES (?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(&name)
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await?;

    Ok(Deck { id, name })
}

/// Cambia il nome di un mazzo. Le carte e i loro progressi non si toccano.
pub async fn rename_deck(db: &Database, deck: &str, name: &str, now: DateTime<Utc>) -> Result<()> {
    let name = required("name", name)?;

    let esito = sqlx::query(
        "UPDATE decks SET name = ?, updated_at = ?, rev = rev + 1
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&name)
    .bind(now)
    .bind(deck)
    .execute(db.pool())
    .await?;

    found(esito.rows_affected(), deck)
}

/// Elimina un mazzo con tutte le sue carte.
///
/// # Cosa vuol dire eliminare
///
/// Tre cose, in quest'ordine: il mazzo e le sue carte prendono la lapide, e lo **stato
/// di studio** delle carte viene ritirato. Senza l'ultimo passo una carta cancellata
/// continuerebbe a scadere e potrebbe tornare in un ripasso, che e' il modo peggiore
/// di scoprire che una cancellazione non era completa.
///
/// Lo storico delle risposte invece resta: `answers` e' in sola aggiunta, e una
/// risposta data e' successa davvero anche se la carta non c'e' piu'.
pub async fn delete_deck(db: &Database, deck: &str, now: DateTime<Utc>) -> Result<()> {
    let carte = cards(db, deck).await?;

    let mut tx = db.pool().begin().await?;

    let esito = sqlx::query("UPDATE decks SET deleted_at = ?, updated_at = ?, rev = rev + 1 WHERE id = ? AND deleted_at IS NULL")
        .bind(now)
        .bind(now)
        .bind(deck)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE flashcards SET deleted_at = ?, updated_at = ?, rev = rev + 1 WHERE deck_id = ? AND deleted_at IS NULL")
        .bind(now)
        .bind(now)
        .bind(deck)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    found(esito.rows_affected(), deck)?;

    retire(db, &carte, now).await
}

/// Le carte di un mazzo, nell'ordine in cui sono state aggiunte.
///
/// Non alfabetico: questo e' l'elenco che si scorre per correggere qualcosa, e
/// ritrovare una carta dove la si e' messa vale piu' che averle in ordine.
pub async fn cards(db: &Database, deck: &str) -> Result<Vec<Flashcard>> {
    let righe = sqlx::query_as::<_, Row>(
        "SELECT id, deck_id, japanese, meaning, furigana
         FROM flashcards
         WHERE deck_id = ? AND deleted_at IS NULL
         ORDER BY created_at ASC, id ASC",
    )
    .bind(deck)
    .fetch_all(db.pool())
    .await?;

    // I significati di tutto il mazzo in una lettura sola: chiederli una carta per
    // volta sarebbe una query per riga per una cosa che si mostra tutta insieme.
    let mut per_carta: HashMap<String, Vec<String>> = HashMap::new();
    let coppie: Vec<(String, String)> = sqlx::query_as(
        "SELECT m.card_id, m.text
         FROM flashcard_meanings m
         JOIN flashcards f ON f.id = m.card_id
         WHERE f.deck_id = ? AND f.deleted_at IS NULL
         ORDER BY m.card_id ASC, m.position ASC",
    )
    .bind(deck)
    .fetch_all(db.pool())
    .await?;
    for (card, text) in coppie {
        per_carta.entry(card).or_default().push(text);
    }

    Ok(righe
        .into_iter()
        .map(|r| {
            let alternatives = per_carta.remove(&r.id).unwrap_or_default();
            assemble(r, alternatives)
        })
        .collect())
}

/// Una carta sola, se esiste ancora.
///
/// Serve al giro di studio, che si porta dietro i soli identificatori e va a prendere
/// il testo di quella che sta per chiedere. `None` non e' un errore: una carta puo'
/// essere stata cancellata mentre la coda era gia' in mano all'interfaccia.
pub async fn card(db: &Database, id: &str) -> Result<Option<Flashcard>> {
    let riga = sqlx::query_as::<_, Row>(
        "SELECT id, deck_id, japanese, meaning, furigana
         FROM flashcards
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(db.pool())
    .await?;

    let Some(riga) = riga else { return Ok(None) };
    Ok(Some(assemble(riga, alternatives_of(db, id).await?)))
}

/// Gli altri significati di una carta, nell'ordine.
async fn alternatives_of(db: &Database, card: &str) -> Result<Vec<String>> {
    let righe: Vec<(String,)> = sqlx::query_as(
        "SELECT text FROM flashcard_meanings WHERE card_id = ? ORDER BY position ASC",
    )
    .bind(card)
    .fetch_all(db.pool())
    .await?;

    Ok(righe.into_iter().map(|(t,)| t).collect())
}

/// Mette insieme la riga e le sue risposte in piu'.
fn assemble(row: Row, alternatives: Vec<String>) -> Flashcard {
    Flashcard {
        id: row.id,
        deck_id: row.deck_id,
        japanese: row.japanese,
        meaning: row.meaning,
        alternatives,
        furigana: row.furigana,
    }
}

/// Aggiunge una carta a un mazzo.
///
/// Il mazzo si controlla prima invece di lasciar fallire la chiave esterna: un mazzo
/// che non esiste e' una cosa che chi chiama deve poter distinguere, e un errore di
/// SQLite appiattito in una stringa non glielo direbbe.
pub async fn create_card(
    db: &Database,
    deck: &str,
    content: Content<'_>,
    now: DateTime<Utc>,
) -> Result<Flashcard> {
    let (japanese, meaning, alternatives, furigana) = clean(content)?;
    if !deck_exists(db, deck).await? {
        return Err(CoreError::UnknownItem {
            id: deck.to_owned(),
        });
    }

    let id = Uuid::now_v7().to_string();
    let mut tx = db.pool().begin().await?;

    sqlx::query(
        "INSERT INTO flashcards (
             id, deck_id, japanese, meaning, furigana, created_at, updated_at, rev
         )
         VALUES (?, ?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(deck)
    .bind(&japanese)
    .bind(&meaning)
    .bind(&furigana)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    write_alternatives(&mut tx, &id, &alternatives).await?;
    tx.commit().await?;

    Ok(Flashcard {
        id,
        deck_id: deck.to_owned(),
        japanese,
        meaning,
        alternatives,
        furigana,
    })
}

/// Aggiunge in blocco un elenco di carte, **in una transazione sola**.
///
/// # Perche' non basta chiamare [`create_card`] N volte
///
/// Perche' quella apre una transazione per carta, quindi un import interrotto a meta'
/// lascerebbe dentro le prime N e nessuno saprebbe quali. Qui o entrano tutte o non
/// entra niente, ed e' la stessa ragione per cui [`Database::ensure_cards`] esiste
/// accanto a `ensure_card`.
///
/// # Si valida tutto prima di scrivere la prima riga
///
/// La pulizia passa su **tutte** le carte prima che la transazione si apra: aprirla e
/// poi accorgersi alla centesima riga che manca un campo vorrebbe dire annullare del
/// lavoro gia' fatto, e soprattutto vorrebbe dire che l'errore dipende da **quante**
/// carte c'erano prima, cosa che non deve contare.
///
/// E' la **stessa** [`clean`] della creazione manuale, non una seconda copia: chi
/// importa non deve poter creare una carta che il modulo rifiuterebbe.
///
/// # Perche' torna un numero e non le carte
///
/// Perche' chi chiama ricarica comunque il mazzo, e rimandare indietro duecento carte
/// per contarle sarebbe un viaggio sprecato attraverso il confine.
pub async fn create_cards(
    db: &Database,
    deck: &str,
    contents: &[Content<'_>],
    now: DateTime<Utc>,
) -> Result<usize> {
    let puliti = contents
        .iter()
        .map(|c| clean(*c))
        .collect::<Result<Vec<_>>>()?;

    if !deck_exists(db, deck).await? {
        return Err(CoreError::UnknownItem {
            id: deck.to_owned(),
        });
    }

    let mut tx = db.pool().begin().await?;

    for (japanese, meaning, alternatives, furigana) in &puliti {
        // UUID v7 preso uno alla volta e non tutti insieme: il contesto condiviso del
        // crate tiene un contatore, quindi due identificatori nati nello stesso
        // millisecondo restano in ordine. Conta, perche' le carte di un mazzo si
        // leggono ordinate per `created_at` e poi per `id`, e qui `created_at` e'
        // identico per tutte: senza quell'ordine un file importato uscirebbe mescolato.
        let id = Uuid::now_v7().to_string();

        sqlx::query(
            "INSERT INTO flashcards (
                 id, deck_id, japanese, meaning, furigana, created_at, updated_at, rev
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(&id)
        .bind(deck)
        .bind(japanese)
        .bind(meaning)
        .bind(furigana)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        write_alternatives(&mut tx, &id, alternatives).await?;
    }

    tx.commit().await?;
    Ok(puliti.len())
}

/// Corregge una carta gia' scritta, e dice **se e' davvero cambiata**.
///
/// **Non tocca lo stato di studio**, ed e' una scelta e non una dimenticanza: se la
/// correzione abbia invalidato o no quello che si e' imparato lo sa solo chi ha
/// corretto, quindi lo decide lui: la via per azzerarlo e' una scelta esplicita, e
/// arriva col wizard che segue il salvataggio.
///
/// # Perche' e' il core a dire se qualcosa e' cambiato
///
/// Perche' il confronto va fatto **dopo aver ripulito**, e la pulizia e' qui: una
/// casella lasciata vuota, uno spazio ai bordi o un doppione non sono una modifica.
/// Farlo dall'altra parte del confine vorrebbe dire riscrivere di la' la stessa regola,
/// e due copie di una regola si sganciano al primo ritocco.
pub async fn update_card(
    db: &Database,
    card: &str,
    content: Content<'_>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let (japanese, meaning, alternatives, furigana) = clean(content)?;

    let prima = self::card(db, card).await?.ok_or_else(|| CoreError::UnknownItem {
        id: card.to_owned(),
    })?;
    let changed = prima.japanese != japanese
        || prima.meaning != meaning
        || prima.alternatives != alternatives
        || prima.furigana != furigana;

    let mut tx = db.pool().begin().await?;

    let esito = sqlx::query(
        "UPDATE flashcards
         SET japanese = ?, meaning = ?, furigana = ?, updated_at = ?, rev = rev + 1
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&japanese)
    .bind(&meaning)
    .bind(&furigana)
    .bind(now)
    .bind(card)
    .execute(&mut *tx)
    .await?;
    found(esito.rows_affected(), card)?;

    // I significati si riscrivono per intero invece di cercare cosa e' cambiato: sono
    // un **valore** della carta, non righe con una vita propria, e cinque stringhe si
    // riscrivono in meno tempo di quanto ci voglia a capire quali due si sono mosse.
    sqlx::query("DELETE FROM flashcard_meanings WHERE card_id = ?")
        .bind(card)
        .execute(&mut *tx)
        .await?;
    write_alternatives(&mut tx, card, &alternatives).await?;

    tx.commit().await?;
    Ok(changed)
}

/// Scrive i significati in piu' di una carta, nell'ordine dato.
async fn write_alternatives(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    card: &str,
    alternatives: &[String],
) -> Result<()> {
    for (i, text) in alternatives.iter().enumerate() {
        sqlx::query("INSERT INTO flashcard_meanings (card_id, position, text) VALUES (?, ?, ?)")
            .bind(card)
            .bind(i as i64)
            .bind(text)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Ripulisce quello che arriva, e rifiuta cio' che non puo' entrare.
///
/// Le due facce restano obbligatorie. Gli altri significati perdono le caselle vuote,
/// che non sono risposte, e i doppioni, compreso un doppione di quello principale: sono
/// la stessa liberta' che ci si prende gia' togliendo gli spazi ai bordi, e non si
/// perde niente, perche' una risposta ripetuta accetta esattamente quello che
/// accettava gia'. Il furigana vuoto vale come assente.
pub(super) fn clean(content: Content<'_>) -> Result<(String, String, Vec<String>, Option<String>)> {
    let japanese = required("japanese", content.japanese)?;
    let meaning = required("meaning", content.meaning)?;

    let mut alternatives: Vec<String> = Vec::new();
    for value in content.alternatives {
        let value = value.trim();
        if value.is_empty() || value == meaning {
            continue;
        }
        if !alternatives.iter().any(|a| a == value) {
            alternatives.push(value.to_owned());
        }
    }

    if alternatives.len() > MAX_ALTERNATIVES {
        return Err(CoreError::TooManyValues {
            field: "alternatives".to_owned(),
            max: MAX_ALTERNATIVES,
        });
    }

    let furigana = content.furigana.trim();
    let furigana = (!furigana.is_empty()).then(|| furigana.to_owned());

    Ok((japanese, meaning, alternatives, furigana))
}

/// Se una carta ha dei progressi, cioe' se e' gia' stata studiata almeno una volta.
///
/// Serve a sapere se c'e' qualcosa da decidere dopo una correzione: su una carta mai
/// studiata non c'e' nessuna memoria tarata su niente, quindi non c'e' niente da
/// azzerare e chiedere sarebbe una domanda senza risposte diverse.
pub async fn studied(db: &Database, card: &str) -> Result<bool> {
    let carte = db
        .cards(CardFilter {
            items: Some(&[item_id(card).as_str().to_owned()]),
            exercise_type: None,
        })
        .await?;

    Ok(!carte.is_empty())
}

/// Riporta i progressi di una carta a zero, in **tutti e due i versi**.
///
/// Tutti e due perche' il contenuto e' uno solo: il lato giapponese e' lo stimolo in un
/// verso e la risposta nell'altro, quindi correggerlo invalida entrambi. Nessun filtro
/// sul tipo di esercizio, che e' anche il motivo per cui l'archivio offre il reset per
/// elemento e non per carta.
///
/// **Non e' mai automatico.** Chi lo chiama lo fa perche' qualcuno ha scelto: solo chi
/// ha corretto sa se la correzione ha invalidato la propria associazione mnemonica.
pub async fn reset_card(db: &Database, card: &str, now: DateTime<Utc>) -> Result<()> {
    if !exists(db, card).await? {
        return Err(CoreError::UnknownItem {
            id: card.to_owned(),
        });
    }

    db.reset_cards(&[item_id(card).as_str().to_owned()], now)
        .await?;
    Ok(())
}

/// Elimina una carta, e con lei la sua pianificazione.
///
/// Vedi [`delete_deck`] per cosa vuol dire eliminare, che qui vale identico su una
/// carta sola.
pub async fn delete_card(db: &Database, card: &str, now: DateTime<Utc>) -> Result<()> {
    let esito = sqlx::query(
        "UPDATE flashcards SET deleted_at = ?, updated_at = ?, rev = rev + 1
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(now)
    .bind(card)
    .execute(db.pool())
    .await?;

    found(esito.rows_affected(), card)?;

    db.retire_cards(&[item_id(card).as_str().to_owned()], now)
        .await?;
    Ok(())
}

/// Se una carta esiste ancora.
async fn exists(db: &Database, card: &str) -> Result<bool> {
    Ok(self::card(db, card).await?.is_some())
}

/// Se un mazzo esiste ancora.
async fn deck_exists(db: &Database, deck: &str) -> Result<bool> {
    let riga: Option<(String,)> =
        sqlx::query_as("SELECT id FROM decks WHERE id = ? AND deleted_at IS NULL")
            .bind(deck)
            .fetch_optional(db.pool())
            .await?;

    Ok(riga.is_some())
}

/// Ritira lo stato di studio di un gruppo di carte.
async fn retire(db: &Database, cards: &[Flashcard], now: DateTime<Utc>) -> Result<()> {
    let ids: Vec<String> = cards
        .iter()
        .map(|c| item_id(&c.id).as_str().to_owned())
        .collect();
    db.retire_cards(&ids, now).await?;
    Ok(())
}

/// Un campo di testo che non puo' restare vuoto, ripulito ai bordi.
///
/// Gli spazi ai bordi si tolgono qui e per sempre: sono un incidente della
/// digitazione, e tenerli vorrebbe dire mostrare un nome che sembra rientrato.
fn required(field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::EmptyField {
            field: field.to_owned(),
        });
    }
    Ok(value.to_owned())
}

/// Zero righe toccate vuol dire che quella riga non c'era.
fn found(rows: u64, id: &str) -> Result<()> {
    if rows == 0 {
        return Err(CoreError::UnknownItem { id: id.to_owned() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    fn adesso() -> DateTime<Utc> {
        "2026-03-15T08:00:00Z".parse().expect("istante valido")
    }

    async fn mazzo(db: &Database) -> Deck {
        create_deck(db, "N5", adesso()).await.unwrap()
    }

    /// Le sole due facce, che sono quello che serve a quasi tutte le prove.
    fn testo<'a>(japanese: &'a str, meaning: &'a str) -> Content<'a> {
        Content {
            japanese,
            meaning,
            alternatives: &[],
            furigana: "",
        }
    }

    #[tokio::test]
    async fn create_cards_scrive_tutto_in_un_colpo() {
        let db = db().await;
        let m = mazzo(&db).await;

        let quante = create_cards(
            &db,
            &m.id,
            &[testo("猫", "cat"), testo("犬", "dog"), testo("鳥", "bird")],
            adesso(),
        )
        .await
        .unwrap();

        assert_eq!(quante, 3);
        assert_eq!(cards(&db, &m.id).await.unwrap().len(), 3);
    }

    /// L'ordine del file e' quello in cui si rileggeranno, e non e' scontato: tutte le
    /// carte nascono nello **stesso istante**, quindi a distinguerle resta il solo
    /// identificatore. Regge perche' l'UUID v7 e' monotono dentro il millisecondo, ed e'
    /// il genere di cosa che va provata invece che creduta.
    #[tokio::test]
    async fn le_carte_importate_restano_nell_ordine_del_file() {
        let db = db().await;
        let m = mazzo(&db).await;

        let attese: Vec<String> = (0..200).map(|i| format!("carta{i:03}")).collect();
        let contenuti: Vec<Content<'_>> = attese.iter().map(|w| testo(w, "x")).collect();

        create_cards(&db, &m.id, &contenuti, adesso()).await.unwrap();

        let lette: Vec<String> = cards(&db, &m.id)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.japanese)
            .collect();
        assert_eq!(lette, attese);
    }

    /// La regola vincolante: o tutto o niente.
    #[tokio::test]
    async fn una_carta_invalida_non_ne_lascia_entrare_nessuna() {
        let db = db().await;
        let m = mazzo(&db).await;

        let esito = create_cards(
            &db,
            &m.id,
            &[testo("猫", "cat"), testo("  ", "orphan"), testo("犬", "dog")],
            adesso(),
        )
        .await;

        assert!(matches!(esito, Err(CoreError::EmptyField { .. })));
        assert!(
            cards(&db, &m.id).await.unwrap().is_empty(),
            "le due buone non devono essere entrate"
        );
    }

    #[tokio::test]
    async fn create_cards_scrive_anche_le_risposte_in_piu() {
        let db = db().await;
        let m = mazzo(&db).await;

        let alternativi = vec!["the japanese language".to_owned()];
        create_cards(
            &db,
            &m.id,
            &[Content {
                japanese: "日本語",
                meaning: "japanese",
                alternatives: &alternativi,
                furigana: "にほんご",
            }],
            adesso(),
        )
        .await
        .unwrap();

        let carta = &cards(&db, &m.id).await.unwrap()[0];
        assert_eq!(carta.alternatives, ["the japanese language"]);
        assert_eq!(carta.furigana.as_deref(), Some("にほんご"));
    }

    #[tokio::test]
    async fn create_cards_su_un_mazzo_che_non_esiste_non_scrive_niente() {
        let db = db().await;
        let esito = create_cards(&db, "nessun-mazzo", &[testo("猫", "cat")], adesso()).await;
        assert!(matches!(esito, Err(CoreError::UnknownItem { .. })));
    }

    /// Importare un file vuoto non e' un errore: e' un import che non fa niente.
    #[tokio::test]
    async fn create_cards_senza_carte_non_e_un_errore() {
        let db = db().await;
        let m = mazzo(&db).await;
        assert_eq!(create_cards(&db, &m.id, &[], adesso()).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn un_mazzo_nasce_col_solo_nome() {
        let db = db().await;
        let deck = create_deck(&db, "Verbi", adesso()).await.unwrap();

        assert_eq!(deck.name, "Verbi");
        let elenco = decks(&db).await.unwrap();
        assert_eq!(elenco.len(), 1);
        assert_eq!(elenco[0].id, deck.id);
        assert_eq!(elenco[0].cards, 0, "nasce vuoto");
    }

    #[tokio::test]
    async fn il_nome_vuoto_non_passa() {
        let db = db().await;
        let esito = create_deck(&db, "   ", adesso()).await;

        assert_eq!(
            esito,
            Err(CoreError::EmptyField {
                field: "name".into()
            })
        );
        assert!(decks(&db).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn gli_spazi_ai_bordi_non_entrano_nel_nome() {
        let db = db().await;
        let deck = create_deck(&db, "  N5  ", adesso()).await.unwrap();
        assert_eq!(deck.name, "N5");
    }

    #[tokio::test]
    async fn i_mazzi_escono_in_ordine_alfabetico() {
        let db = db().await;
        for nome in ["verbi", "Aggettivi", "N5"] {
            create_deck(&db, nome, adesso()).await.unwrap();
        }

        let nomi: Vec<String> = decks(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(nomi, ["Aggettivi", "N5", "verbi"]);
    }

    #[tokio::test]
    async fn rinominare_non_tocca_le_carte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();

        rename_deck(&db, &deck.id, "JLPT N5", adesso())
            .await
            .unwrap();

        let elenco = decks(&db).await.unwrap();
        assert_eq!(elenco[0].name, "JLPT N5");
        assert_eq!(elenco[0].cards, 1);
    }

    #[tokio::test]
    async fn un_mazzo_che_non_esiste_si_riconosce() {
        let db = db().await;
        let esito = rename_deck(&db, "non-esiste", "x", adesso()).await;
        assert_eq!(
            esito,
            Err(CoreError::UnknownItem {
                id: "non-esiste".into()
            })
        );
    }

    #[tokio::test]
    async fn una_carta_vive_dentro_un_mazzo() {
        let db = db().await;
        let deck = mazzo(&db).await;

        let card = create_card(&db, &deck.id, testo(" ねこ ", " cat "), adesso())
            .await
            .unwrap();

        assert_eq!(card.japanese, "ねこ");
        assert_eq!(card.meaning, "cat");
        assert_eq!(card.deck_id, deck.id);
        assert_eq!(cards(&db, &deck.id).await.unwrap(), vec![card]);
        assert_eq!(decks(&db).await.unwrap()[0].cards, 1);
    }

    #[tokio::test]
    async fn una_carta_vuole_tutte_e_due_le_facce() {
        let db = db().await;
        let deck = mazzo(&db).await;

        assert_eq!(
            create_card(&db, &deck.id, testo("", "cat"), adesso()).await,
            Err(CoreError::EmptyField {
                field: "japanese".into()
            })
        );
        assert_eq!(
            create_card(&db, &deck.id, testo("ねこ", ""), adesso()).await,
            Err(CoreError::EmptyField {
                field: "meaning".into()
            })
        );
        assert!(cards(&db, &deck.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn una_carta_senza_mazzo_non_si_crea() {
        let db = db().await;
        let esito = create_card(&db, "non-esiste", testo("ねこ", "cat"), adesso()).await;

        assert_eq!(
            esito,
            Err(CoreError::UnknownItem {
                id: "non-esiste".into()
            })
        );
    }

    #[tokio::test]
    async fn correggere_una_carta_riscrive_le_due_facce() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねご", "cat"), adesso())
            .await
            .unwrap();

        update_card(&db, &card.id, testo("ねこ", "cat, feline"), adesso())
            .await
            .unwrap();

        let riletta = &cards(&db, &deck.id).await.unwrap()[0];
        assert_eq!(riletta.japanese, "ねこ");
        assert_eq!(riletta.meaning, "cat, feline");
        assert_eq!(riletta.id, card.id, "resta la stessa carta");
    }

    /// La carta dell'esempio, con tutto quello che puo' portare.
    fn completa<'a>(alternatives: &'a [String], furigana: &'a str) -> Content<'a> {
        Content {
            japanese: "日本語",
            meaning: "japanese",
            alternatives,
            furigana,
        }
    }

    #[tokio::test]
    async fn le_risposte_in_piu_si_rileggono_come_sono_state_scritte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let extra = vec!["the japanese language".to_owned(), "nihongo".to_owned()];

        let card = create_card(&db, &deck.id, completa(&extra, "にほんご"), adesso())
            .await
            .unwrap();

        assert_eq!(card.alternatives, extra, "e nell'ordine in cui sono arrivate");
        assert_eq!(card.furigana.as_deref(), Some("にほんご"));

        // E si ritrovano uguali rileggendole, una carta per volta e tutto il mazzo.
        let riletta = self::card(&db, &card.id).await.unwrap().unwrap();
        assert_eq!(riletta, card);
        assert_eq!(cards(&db, &deck.id).await.unwrap(), vec![card]);
    }

    #[tokio::test]
    async fn una_carta_senza_risposte_in_piu_non_ne_ha() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();

        assert!(card.alternatives.is_empty());
        // `None` e non una stringa vuota: vuol dire «questa carta non ne ha bisogno».
        assert_eq!(card.furigana, None);
    }

    #[tokio::test]
    async fn le_caselle_vuote_e_i_doppioni_non_sono_risposte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let extra = vec![
            "  ".to_owned(),
            " nihongo ".to_owned(),
            "nihongo".to_owned(),
            "japanese".to_owned(),
        ];

        let card = create_card(&db, &deck.id, completa(&extra, "  "), adesso())
            .await
            .unwrap();

        // Una casella lasciata vuota non e' una risposta; un doppione accetta quello
        // che si accettava gia', compreso un doppione del significato principale.
        assert_eq!(card.alternatives, ["nihongo"]);
        assert_eq!(card.furigana, None);
    }

    #[tokio::test]
    async fn oltre_il_tetto_non_si_passa() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let troppi: Vec<String> = (0..=MAX_ALTERNATIVES).map(|i| format!("m{i}")).collect();

        let esito = create_card(&db, &deck.id, completa(&troppi, ""), adesso()).await;

        assert_eq!(
            esito,
            Err(CoreError::TooManyValues {
                field: "alternatives".into(),
                max: MAX_ALTERNATIVES
            })
        );
        assert!(cards(&db, &deck.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn correggere_riscrive_anche_le_risposte_in_piu() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let prima = vec!["uno".to_owned(), "due".to_owned()];
        let card = create_card(&db, &deck.id, completa(&prima, "にほんご"), adesso())
            .await
            .unwrap();

        let dopo = vec!["tre".to_owned()];
        update_card(&db, &card.id, completa(&dopo, ""), adesso())
            .await
            .unwrap();

        let riletta = self::card(&db, &card.id).await.unwrap().unwrap();
        assert_eq!(riletta.alternatives, ["tre"], "le vecchie se ne vanno");
        assert_eq!(riletta.furigana, None, "e si puo' togliere anche il furigana");
    }

    #[tokio::test]
    async fn una_correzione_che_non_cambia_niente_lo_dice() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let extra = vec!["nihongo".to_owned()];
        let card = create_card(&db, &deck.id, completa(&extra, "にほんご"), adesso())
            .await
            .unwrap();

        // Gli stessi valori, con spazi ai bordi, una casella vuota e un doppione: dopo
        // la pulizia e' esattamente quello che c'era gia'.
        let uguale = vec![" nihongo ".to_owned(), "  ".to_owned(), "nihongo".to_owned()];
        let cambiata = update_card(
            &db,
            &card.id,
            Content {
                japanese: " 日本語 ",
                meaning: "japanese",
                alternatives: &uguale,
                furigana: " にほんご ",
            },
            adesso(),
        )
        .await
        .unwrap();
        assert!(!cambiata, "non e' una modifica");

        // Basta pero' una risposta in piu' perche' lo diventi.
        let diversi = vec!["nihongo".to_owned(), "the japanese language".to_owned()];
        assert!(
            update_card(&db, &card.id, completa(&diversi, "にほんご"), adesso())
                .await
                .unwrap()
        );

        // E anche togliere il solo furigana.
        assert!(
            update_card(&db, &card.id, completa(&diversi, ""), adesso())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn le_risposte_in_piu_restano_attaccate_alla_loro_carta() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let sua = vec!["the japanese language".to_owned()];
        create_card(&db, &deck.id, completa(&sua, "にほんご"), adesso())
            .await
            .unwrap();
        create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();

        let elenco = cards(&db, &deck.id).await.unwrap();
        assert_eq!(elenco[0].alternatives, sua);
        assert!(elenco[1].alternatives.is_empty(), "l'altra non se le prende");
    }

    #[tokio::test]
    async fn le_carte_restano_nell_ordine_in_cui_sono_state_aggiunte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        for (jp, en) in [("ねこ", "cat"), ("いぬ", "dog"), ("とり", "bird")] {
            create_card(&db, &deck.id, testo(jp, en), adesso()).await.unwrap();
        }

        let ordine: Vec<String> = cards(&db, &deck.id)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.japanese)
            .collect();
        assert_eq!(ordine, ["ねこ", "いぬ", "とり"]);
    }

    #[tokio::test]
    async fn cancellare_una_carta_la_toglie_dal_mazzo() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();

        delete_card(&db, &card.id, adesso()).await.unwrap();

        assert!(cards(&db, &deck.id).await.unwrap().is_empty());
        assert_eq!(decks(&db).await.unwrap()[0].cards, 0);
        // Cancellare due volte non riesce: la seconda volta non c'e' piu' niente.
        assert!(delete_card(&db, &card.id, adesso()).await.is_err());
    }

    #[tokio::test]
    async fn cancellare_un_mazzo_porta_via_le_sue_carte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let altro = create_deck(&db, "N4", adesso()).await.unwrap();
        create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();
        create_card(&db, &altro.id, testo("いぬ", "dog"), adesso())
            .await
            .unwrap();

        delete_deck(&db, &deck.id, adesso()).await.unwrap();

        let elenco = decks(&db).await.unwrap();
        assert_eq!(elenco.len(), 1, "resta solo l'altro mazzo");
        assert_eq!(elenco[0].id, altro.id);
        assert!(cards(&db, &deck.id).await.unwrap().is_empty());
        assert_eq!(
            cards(&db, &altro.id).await.unwrap().len(),
            1,
            "l'altro mazzo non e' stato toccato"
        );
    }

    #[tokio::test]
    async fn cancellare_una_carta_ne_ritira_anche_la_pianificazione() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();
        let item = item_id(&card.id);
        db.ensure_card(item.as_str(), "flashcard.jp_to_meaning", adesso())
            .await
            .unwrap();

        delete_card(&db, &card.id, adesso()).await.unwrap();

        // Senza questo una carta cancellata continuerebbe a scadere, e tornerebbe in
        // un ripasso mesi dopo: e' il modo peggiore di scoprire che una cancellazione
        // non era completa.
        assert_eq!(
            db.card(item.as_str(), "flashcard.jp_to_meaning")
                .await
                .unwrap(),
            None
        );
    }

    /// Studia una carta, cosi' che abbia dei progressi da azzerare.
    async fn studia(db: &Database, card: &str, esercizio: &str) {
        use crate::shared::srs::{Grade, MemoryState, Scheduled};
        use crate::shared::storage::{NewAnswer, Scheduling};

        db.record_answer(NewAnswer {
            item_id: item_id(card).as_str(),
            exercise_type: esercizio,
            correct: true,
            answer: "x",
            answered_at: adesso(),
            response_time_ms: None,
            scheduling: Some(Scheduling {
                grade: Grade::Good,
                next: Scheduled {
                    memory: MemoryState {
                        stability: 12.0,
                        difficulty: 5.0,
                    },
                    due_at: adesso() + chrono::TimeDelta::days(12),
                    interval_days: 12.0,
                },
            }),
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn una_carta_mai_studiata_non_ha_niente_da_azzerare() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();

        assert!(!studied(&db, &card.id).await.unwrap());
    }

    #[tokio::test]
    async fn azzerare_riporta_la_carta_a_mai_studiata() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();
        studia(&db, &card.id, "flashcard.jp_to_meaning").await;
        assert!(studied(&db, &card.id).await.unwrap());

        reset_card(&db, &card.id, adesso()).await.unwrap();

        let carta = db
            .card(item_id(&card.id).as_str(), "flashcard.jp_to_meaning")
            .await
            .unwrap()
            .expect("la riga resta, azzerata");
        assert_eq!(carta.due_at, None, "dovuta subito, come una mai vista");
        assert_eq!(carta.last_reviewed_at, None);
        assert_eq!(carta.reps, 0);
        assert_eq!(carta.lapses, 0);
        assert_eq!(carta.memory(), None, "FSRS riparte da zero");

        // Lo storico invece resta: quelle risposte sono state date davvero.
        let storico = db
            .answers(item_id(&card.id).as_str(), "flashcard.jp_to_meaning")
            .await
            .unwrap();
        assert_eq!(storico.len(), 1);
    }

    #[tokio::test]
    async fn azzerare_vale_per_tutti_e_due_i_versi() {
        let db = db().await;
        let deck = mazzo(&db).await;
        let card = create_card(&db, &deck.id, testo("ねこ", "cat"), adesso())
            .await
            .unwrap();
        for verso in ["flashcard.jp_to_meaning", "flashcard.meaning_to_jp"] {
            studia(&db, &card.id, verso).await;
        }

        reset_card(&db, &card.id, adesso()).await.unwrap();

        // Il contenuto e' uno solo: il giapponese e' lo stimolo in un verso e la
        // risposta nell'altro, quindi correggerlo invalida entrambi.
        for verso in ["flashcard.jp_to_meaning", "flashcard.meaning_to_jp"] {
            let carta = db
                .card(item_id(&card.id).as_str(), verso)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(carta.memory(), None, "{verso} non e' stato azzerato");
        }
    }

    #[tokio::test]
    async fn azzerare_una_carta_che_non_esiste_non_passa_in_silenzio() {
        let db = db().await;
        assert_eq!(
            reset_card(&db, "non-esiste", adesso()).await,
            Err(CoreError::UnknownItem {
                id: "non-esiste".into()
            })
        );
    }

    #[tokio::test]
    async fn l_identificatore_di_studio_non_nomina_il_mazzo() {
        let id = item_id("0195e0c1-0000-7000-8000-000000000000");
        assert_eq!(
            id.as_str(),
            "flashcard:0195e0c1-0000-7000-8000-000000000000"
        );
        assert_eq!(id.as_str().split(':').count(), 2, "due segmenti, non tre");
    }
}
