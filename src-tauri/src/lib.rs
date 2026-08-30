//! La shell Tauri: apre il database, tiene lo stato dell'app e registra i comandi.
//!
//! Tutto il resto sta in `tanren-core`.

mod commands;

use tanren_core::shared::storage::Database;
use tauri::Manager;

/// Quello che ogni comando ha bisogno di avere sottomano.
///
/// Non c'e' nessuno scheduler: sui kana la ripetizione spaziata non si usa, e quando
/// arriveranno i kanji sara' la loro sessione a costruirsene uno.
pub struct AppState {
    pub db: Database,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Il database vive nella cartella dati dell'app, quella che il sistema
            // assegna a questo identificatore: su Android e' privata all'app, su
            // desktop sta sotto la home dell'utente.
            let path = app.path().app_data_dir()?.join("tanren.db");
            log::info!("database utente: {}", path.display());

            let db = tauri::async_runtime::block_on(Database::open(&path))?;

            app.manage(AppState { db });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::credits,
            commands::app_version,
            commands::normalize_input,
            commands::kana_catalogue,
            commands::start_kana_session,
            commands::next_kana_step,
            commands::submit_kana_answer,
            commands::normalize_reading,
            commands::kanji_overview,
            commands::kanji_grid,
            commands::kanji_dashboard,
            commands::kanji_details,
            commands::kanji_current_level,
            commands::kanji_level_count,
            commands::start_kanji_study,
            commands::next_kanji_study_step,
            commands::submit_kanji_study_answer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
