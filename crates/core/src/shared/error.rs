//! Tipo di errore del core.
//!
//! Il confine con il frontend attraversa i comandi Tauri, quindi l'errore deve poter
//! essere serializzato senza perdere il motivo del fallimento.

use serde::Serialize;
use thiserror::Error;

/// Cio' che puo' andare storto dentro il dominio.
///
/// Volutamente povero: qui finiscono solo i fallimenti che hanno senso per chi sta
/// dall'altra parte del confine. Gli errori che sono difetti nostri, come un file di
/// contenuto malformato incluso nel binario, non passano di qui.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreError {
    /// L'identificatore non corrisponde a nessun elemento conosciuto.
    #[error("elemento sconosciuto: {id}")]
    UnknownItem { id: String },

    /// L'elemento esiste ma questo tipo di esercizio non sa che farsene.
    #[error("l'esercizio {exercise} non si applica all'elemento {id}")]
    ItemNotSupported { exercise: String, id: String },

    /// Il database locale non ha risposto come previsto.
    ///
    /// Il messaggio di sqlx viene appiattito in una stringa perche' l'errore deve
    /// attraversare il confine verso il frontend, dove un tipo di libreria non
    /// arriverebbe.
    #[error("errore di archiviazione: {message}")]
    Storage { message: String },

    /// La pianificazione della ripetizione non e' andata a buon fine.
    #[error("errore di pianificazione: {message}")]
    Scheduling { message: String },
}

impl From<sqlx::Error> for CoreError {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage {
            message: e.to_string(),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for CoreError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        Self::Storage {
            message: format!("migrazione fallita: {e}"),
        }
    }
}

/// Scorciatoia per i risultati del dominio.
pub type Result<T> = std::result::Result<T, CoreError>;
