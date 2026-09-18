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

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::ItemId;
use crate::shared::storage::Database;

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

/// Una carta: il giapponese da una parte, il significato dall'altra.
///
/// Il testo e' quello che l'utente ha scritto, non normalizzato. La normalizzazione
/// serve al confronto e si fa al momento del confronto: farla qui vorrebbe dire
/// perdere la forma originale senza poterla piu' rileggere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Flashcard {
    pub id: String,
    pub deck_id: String,
    pub japanese: String,
    pub meaning: String,
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
    let cards = sqlx::query_as::<_, Flashcard>(
        "SELECT id, deck_id, japanese, meaning
         FROM flashcards
         WHERE deck_id = ? AND deleted_at IS NULL
         ORDER BY created_at ASC, id ASC",
    )
    .bind(deck)
    .fetch_all(db.pool())
    .await?;

    Ok(cards)
}

/// Aggiunge una carta a un mazzo.
///
/// Il mazzo si controlla prima invece di lasciar fallire la chiave esterna: un mazzo
/// che non esiste e' una cosa che chi chiama deve poter distinguere, e un errore di
/// SQLite appiattito in una stringa non glielo direbbe.
pub async fn create_card(
    db: &Database,
    deck: &str,
    japanese: &str,
    meaning: &str,
    now: DateTime<Utc>,
) -> Result<Flashcard> {
    let japanese = required("japanese", japanese)?;
    let meaning = required("meaning", meaning)?;
    if !deck_exists(db, deck).await? {
        return Err(CoreError::UnknownItem {
            id: deck.to_owned(),
        });
    }

    let id = Uuid::now_v7().to_string();

    sqlx::query(
        "INSERT INTO flashcards (id, deck_id, japanese, meaning, created_at, updated_at, rev)
         VALUES (?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(deck)
    .bind(&japanese)
    .bind(&meaning)
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await?;

    Ok(Flashcard {
        id,
        deck_id: deck.to_owned(),
        japanese,
        meaning,
    })
}

/// Corregge una carta gia' scritta.
///
/// **Non tocca lo stato di studio**, ed e' una scelta e non una dimenticanza: se la
/// correzione abbia invalidato o no quello che si e' imparato lo sa solo chi ha
/// corretto, quindi lo decide lui: la via per azzerarlo e' una scelta esplicita, e
/// arriva col wizard che segue il salvataggio.
pub async fn update_card(
    db: &Database,
    card: &str,
    japanese: &str,
    meaning: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let japanese = required("japanese", japanese)?;
    let meaning = required("meaning", meaning)?;

    let esito = sqlx::query(
        "UPDATE flashcards SET japanese = ?, meaning = ?, updated_at = ?, rev = rev + 1
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&japanese)
    .bind(&meaning)
    .bind(now)
    .bind(card)
    .execute(db.pool())
    .await?;

    found(esito.rows_affected(), card)
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
        create_card(&db, &deck.id, "ねこ", "cat", adesso())
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

        let card = create_card(&db, &deck.id, " ねこ ", " cat ", adesso())
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
            create_card(&db, &deck.id, "", "cat", adesso()).await,
            Err(CoreError::EmptyField {
                field: "japanese".into()
            })
        );
        assert_eq!(
            create_card(&db, &deck.id, "ねこ", "", adesso()).await,
            Err(CoreError::EmptyField {
                field: "meaning".into()
            })
        );
        assert!(cards(&db, &deck.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn una_carta_senza_mazzo_non_si_crea() {
        let db = db().await;
        let esito = create_card(&db, "non-esiste", "ねこ", "cat", adesso()).await;

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
        let card = create_card(&db, &deck.id, "ねご", "cat", adesso())
            .await
            .unwrap();

        update_card(&db, &card.id, "ねこ", "cat, feline", adesso())
            .await
            .unwrap();

        let riletta = &cards(&db, &deck.id).await.unwrap()[0];
        assert_eq!(riletta.japanese, "ねこ");
        assert_eq!(riletta.meaning, "cat, feline");
        assert_eq!(riletta.id, card.id, "resta la stessa carta");
    }

    #[tokio::test]
    async fn le_carte_restano_nell_ordine_in_cui_sono_state_aggiunte() {
        let db = db().await;
        let deck = mazzo(&db).await;
        for (jp, en) in [("ねこ", "cat"), ("いぬ", "dog"), ("とり", "bird")] {
            create_card(&db, &deck.id, jp, en, adesso()).await.unwrap();
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
        let card = create_card(&db, &deck.id, "ねこ", "cat", adesso())
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
        create_card(&db, &deck.id, "ねこ", "cat", adesso())
            .await
            .unwrap();
        create_card(&db, &altro.id, "いぬ", "dog", adesso())
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
        let card = create_card(&db, &deck.id, "ねこ", "cat", adesso())
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
