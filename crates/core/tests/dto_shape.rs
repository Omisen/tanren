//! La forma dei dati che attraversano il confine verso il frontend.
//!
//! I tipi TypeScript in `src/shared/bridge/types.ts` sono scritti a mano e rispecchiano
//! questi. Senza qualcosa che li leghi, una rinomina in Rust passerebbe la revisione e
//! romperebbe la UI a runtime. Questi test fissano il JSON prodotto: chi cambia un
//! nome di campo qui vede fallire un test e sa che deve aggiornare anche il lato
//! TypeScript.
//!
//! Un giorno si potrebbe generare il TypeScript dal Rust, con `ts-rs` o `specta`, e
//! allora questi test diventerebbero superflui. Finche' i tipi si scrivono a mano,
//! sono il guardrail.

use chrono::{TimeZone, Utc};
use serde_json::json;
use tanren_core::features::kana::data::{KanaGroup, Syllabary};
use tanren_core::features::kana::session::{Mode, Outcome, Progress, Scope};
use tanren_core::shared::error::CoreError;
use tanren_core::shared::exercise::{
    AnswerFormat, ExerciseTypeId, ItemId, Prompt, Question, Verdict,
};

#[test]
fn una_domanda_a_scelta_multipla() {
    let q = Question {
        exercise_type: ExerciseTypeId::new("kana.recognition"),
        item: ItemId::new("kana:hiragana:か"),
        prompt: Prompt::Japanese("か".into()),
        format: AnswerFormat::Choice {
            options: vec!["ka".into(), "ki".into()],
        },
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kana.recognition",
            "item": "kana:hiragana:か",
            "prompt": { "script": "japanese", "text": "か" },
            "format": { "mode": "choice", "options": ["ka", "ki"] }
        })
    );
}

#[test]
fn una_domanda_a_input_libero() {
    let q = Question {
        exercise_type: ExerciseTypeId::new("kana.input"),
        item: ItemId::new("kana:katakana:カ"),
        prompt: Prompt::Latin("ka".into()),
        format: AnswerFormat::Input,
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kana.input",
            "item": "kana:katakana:カ",
            "prompt": { "script": "latin", "text": "ka" },
            "format": { "mode": "input" }
        })
    );
}

#[test]
fn un_esito_giusto_e_uno_sbagliato() {
    let due_at = Utc.with_ymd_and_hms(2026, 8, 29, 12, 0, 0).unwrap();

    let giusto = Outcome {
        verdict: Verdict::Correct,
        due_at,
        interval_days: 3.5,
    };
    assert_eq!(
        serde_json::to_value(&giusto).unwrap(),
        json!({
            "verdict": { "outcome": "correct" },
            "dueAt": "2026-08-29T12:00:00Z",
            "intervalDays": 3.5
        })
    );

    let sbagliato = Outcome {
        verdict: Verdict::Incorrect {
            accepted: vec!["shi".into(), "si".into()],
        },
        due_at,
        interval_days: 0.01,
    };
    assert_eq!(
        serde_json::to_value(&sbagliato).unwrap()["verdict"],
        json!({ "outcome": "incorrect", "accepted": ["shi", "si"] })
    );
}

#[test]
fn l_ambito_e_l_avanzamento() {
    let scope = Scope {
        syllabary: Syllabary::Hiragana,
        groups: vec![KanaGroup::Base, KanaGroup::Yoon],
        mode: Mode::Input,
    };
    assert_eq!(
        serde_json::to_value(&scope).unwrap(),
        json!({
            "syllabary": "hiragana",
            "groups": ["base", "yoon"],
            "mode": "input"
        })
    );

    // L'ambito arriva dal frontend, quindi deve anche potersi rileggere.
    let riletto: Scope = serde_json::from_value(serde_json::to_value(&scope).unwrap()).unwrap();
    assert_eq!(riletto, scope);

    assert_eq!(
        serde_json::to_value(Progress { total: 46, due: 12 }).unwrap(),
        json!({ "total": 46, "due": 12 })
    );
}

#[test]
fn gli_errori_arrivano_riconoscibili() {
    assert_eq!(
        serde_json::to_value(CoreError::UnknownItem {
            id: "kana:hiragana:X".into()
        })
        .unwrap(),
        json!({ "kind": "unknown_item", "id": "kana:hiragana:X" })
    );

    assert_eq!(
        serde_json::to_value(CoreError::Storage {
            message: "disco pieno".into()
        })
        .unwrap(),
        json!({ "kind": "storage", "message": "disco pieno" })
    );
}
