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

use serde_json::json;
use tanren_core::features::kana::data::{KanaGroup, Syllabary};
use tanren_core::features::kana::session::{Mode, Scope, Step};
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
    assert_eq!(
        serde_json::to_value(Verdict::Correct).unwrap(),
        json!({ "outcome": "correct" })
    );

    assert_eq!(
        serde_json::to_value(Verdict::Incorrect {
            accepted: vec!["shi".into(), "si".into()],
        })
        .unwrap(),
        json!({ "outcome": "incorrect", "accepted": ["shi", "si"] })
    );
}

#[test]
fn un_passo_della_sessione() {
    let step = Step {
        question: Some(Question {
            exercise_type: ExerciseTypeId::new("kana.recognition"),
            item: ItemId::new("kana:hiragana:か"),
            prompt: Prompt::Japanese("か".into()),
            format: AnswerFormat::Choice {
                options: vec!["ka".into()],
            },
        }),
        queue: vec![ItemId::new("kana:hiragana:か"), ItemId::new("kana:hiragana:き")],
    };

    assert_eq!(
        serde_json::to_value(&step).unwrap()["queue"],
        json!(["kana:hiragana:か", "kana:hiragana:き"])
    );

    // A giro finito la domanda manca, e il frontend lo riconosce da qui.
    assert_eq!(
        serde_json::to_value(Step {
            question: None,
            queue: Vec::new(),
        })
        .unwrap(),
        json!({ "question": null, "queue": [] })
    );
}

#[test]
fn l_ambito_attraversa_il_confine_nei_due_versi() {
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
