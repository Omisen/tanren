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
use tanren_core::features::kanji::levels::Level;
use tanren_core::features::kanji::progress::{Blocked, Gate, LevelProgress, LevelSummary};
use tanren_core::features::kanji::study::{Mode as StudyMode, Scope as StudyScope};
use tanren_core::shared::session::Task;
use tanren_core::shared::error::CoreError;
use tanren_core::shared::exercise::{
    AnswerFormat, ExerciseTypeId, ItemId, Note, Prompt, Question, Verdict,
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
        asks: None,
        focus: None,
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kana.recognition",
            "item": "kana:hiragana:か",
            "prompt": { "script": "japanese", "text": "か" },
            "format": { "mode": "choice", "options": ["ka", "ki"] },
            "asks": null,
            "focus": null
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
        asks: None,
        focus: None,
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kana.input",
            "item": "kana:katakana:カ",
            "prompt": { "script": "latin", "text": "ka" },
            "format": { "mode": "input" },
            "asks": null,
            "focus": null
        })
    );
}

/// Una domanda che deve dire cosa chiede, come quelle sui kanji.
///
/// `asks` e' un'etichetta da mappare e non testo da mostrare: chi la legge di la' del
/// confine deve trovarci `on`, non «On reading».
#[test]
fn una_domanda_che_precisa_cosa_chiede() {
    let q = Question {
        exercise_type: ExerciseTypeId::new("kanji.recognition"),
        item: ItemId::new("kanji:first:on:生"),
        prompt: Prompt::Japanese("生".into()),
        format: AnswerFormat::Choice {
            options: vec!["セイ".into(), "ジン".into()],
        },
        asks: Some("on".into()),
        focus: None,
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kanji.recognition",
            "item": "kanji:first:on:生",
            "prompt": { "script": "japanese", "text": "生" },
            "format": { "mode": "choice", "options": ["セイ", "ジン"] },
            "asks": "on",
            "focus": null
        })
    );
}

/// La porzione su cui verte la domanda attraversa il confine come dato.
///
/// Lo stimolo e' la parola intera, ma quello che si chiede e' il pezzo scritto col
/// kanji: senza `focus` l'interfaccia dovrebbe ricavare da sola dove finisce il kanji
/// e comincia l'okurigana, cioe' portarsi dentro sapere di dominio.
#[test]
fn una_domanda_che_verte_su_una_porzione() {
    let q = Question {
        exercise_type: ExerciseTypeId::new("kanji.okurigana"),
        item: ItemId::new("kanji:大きい"),
        prompt: Prompt::Japanese("大きい".into()),
        format: AnswerFormat::Input,
        asks: Some("okurigana".into()),
        focus: Some("大".into()),
    };

    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        json!({
            "exerciseType": "kanji.okurigana",
            "item": "kanji:大きい",
            "prompt": { "script": "japanese", "text": "大きい" },
            "format": { "mode": "input" },
            "asks": "okurigana",
            "focus": "大"
        })
    );
}

#[test]
fn un_esito_giusto_e_uno_sbagliato() {
    assert_eq!(
        serde_json::to_value(Verdict::correct()).unwrap(),
        json!({ "outcome": "correct" }),
        "senza rilievi il campo non attraversa nemmeno il confine"
    );

    // Giusta, ma scritta contro la convenzione: il rilievo viaggia come etichetta da
    // mappare, non come frase gia' scritta.
    assert_eq!(
        serde_json::to_value(Verdict::Correct {
            note: Some(Note {
                kind: "on_in_hiragana".into(),
                expected: "イチ".into(),
            }),
        })
        .unwrap(),
        json!({
            "outcome": "correct",
            "note": { "kind": "on_in_hiragana", "expected": "イチ" }
        })
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
            asks: None,
            focus: None,
        }),
        queue: vec![
            Task::new(ItemId::new("kana:hiragana:か"), ExerciseTypeId::new("kana.recognition")),
            Task::new(ItemId::new("kana:hiragana:き"), ExerciseTypeId::new("kana.recognition")),
        ],
    };

    // La coda dice su cosa si sta per chiedere e **che cosa** se ne chiede: un giro
    // puo' mescolare esercizi diversi, come succede sulle faccette di un kanji.
    assert_eq!(
        serde_json::to_value(&step).unwrap()["queue"],
        json!([
            { "item": "kana:hiragana:か", "exercise": "kana.recognition" },
            { "item": "kana:hiragana:き", "exercise": "kana.recognition" }
        ])
    );

    // La coda torna indietro dal frontend com'era arrivata, quindi deve anche
    // potersi rileggere: e' il punto in cui un giro si spezzerebbe alla seconda
    // domanda invece che subito.
    let riletta: Vec<Task> = serde_json::from_value(serde_json::to_value(&step).unwrap()["queue"].clone())
        .expect("la coda si rilegge");
    assert_eq!(riletta, step.queue);

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

/// Come la porta del Learning attraversa il confine.
///
/// Il motivo del rifiuto **deve** arrivare distinguibile: «consolida quello che hai» e
/// «torna fra quattro ore» sono due consigli diversi, e un `false` non li direbbe.
#[test]
fn la_porta_dice_perche_e_chiusa() {
    assert_eq!(
        serde_json::to_value(Gate::Open { room: 3 }).unwrap(),
        json!({ "state": "open", "room": 3 })
    );

    assert_eq!(
        serde_json::to_value(Gate::Closed(Blocked::Consolidate {
            current: 0.5,
            needed: 0.75,
        }))
        .unwrap(),
        json!({ "state": "closed", "reason": "consolidate", "current": 0.5, "needed": 0.75 })
    );

    // Un istante solo per tutti e due i freni a tempo: la schermata ne fa un conto
    // alla rovescia e non ha bisogno di sapere quale dei due ha morso.
    assert_eq!(
        serde_json::to_value(Gate::Closed(Blocked::Wait {
            until: "2026-03-16T02:00:00Z".parse().unwrap(),
        }))
        .unwrap(),
        json!({ "state": "closed", "reason": "wait", "until": "2026-03-16T02:00:00Z" })
    );

    assert_eq!(
        serde_json::to_value(Gate::Closed(Blocked::NothingNew)).unwrap(),
        json!({ "state": "closed", "reason": "nothing_new" })
    );
}

/// Una riga della dashboard.
///
/// L'avanzamento e' **appiattito** dentro la riga invece di stare in un oggetto
/// annidato: chi la legge vuole i numeri di quel livello, non una scatola dentro una
/// scatola. `flatten` e' facile da sbagliare, quindi la forma si fissa qui.
#[test]
fn una_riga_della_dashboard() {
    let riga = LevelSummary {
        progress: LevelProgress {
            level: Level::new(2).unwrap(),
            total: 37,
            new: 30,
            learning: 5,
            mature: 2,
            ratio: 0.054_054_055,
            complete: false,
        },
        recall: Some(0.8),
        unlocked: true,
    };

    let json = serde_json::to_value(riga).unwrap();
    assert_eq!(json["level"], json!(2), "l'avanzamento e' appiattito");
    assert_eq!(json["total"], json!(37));
    assert_eq!(json["mature"], json!(2));
    assert_eq!(json["complete"], json!(false));
    assert_eq!(json["unlocked"], json!(true));

    // Le proporzioni sono `f32` e attraversano il confine allargate a `f64`, quindi
    // 0,8 arriva come 0,800000011920929. Non si arrotonda qui: chi le mostra le
    // arrotonda per mostrarle, e arrotondare alla fonte butterebbe via precisione per
    // un problema di presentazione.
    assert!((json["recall"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    assert!((json["ratio"].as_f64().unwrap() - 0.054_054).abs() < 1e-5);

    // Senza faccette attive non c'e' niente da misurare, e si dice `null` invece di
    // uno zero, che vorrebbe dire «non reggi niente».
    let vuoto = LevelSummary {
        recall: None,
        ..riga
    };
    assert_eq!(serde_json::to_value(vuoto).unwrap()["recall"], json!(null));
}

/// L'ambito di uno studio sui kanji, che il frontend costruisce e il core rilegge.
#[test]
fn l_ambito_dello_studio_attraversa_il_confine_nei_due_versi() {
    let scope = StudyScope {
        mode: StudyMode::Learning,
        level: Level::new(3).unwrap(),
    };
    assert_eq!(
        serde_json::to_value(scope).unwrap(),
        json!({ "mode": "learning", "level": 3 })
    );

    let riletto: StudyScope = serde_json::from_value(json!({ "mode": "drill", "level": 1 })).unwrap();
    assert_eq!(riletto.mode, StudyMode::Drill);

    // Un livello fuori scala non deve entrare: le tabelle sono sessantanove.
    assert!(serde_json::from_value::<StudyScope>(json!({ "mode": "drill", "level": 200 })).is_err());
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
