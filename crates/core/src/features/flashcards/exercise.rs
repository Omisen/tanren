//! I due versi in cui si puo' chiedere una carta.
//!
//! # Perche' non implementano `ExerciseType`
//!
//! Il tratto condiviso presume che l'esercizio **sappia dove trovare il proprio
//! contenuto**: `grade` e' sincrono e non ha il database sottomano, e la ricerca
//! dell'esercizio restituisce un `&'static dyn ExerciseType`, cioe' qualcosa senza
//! stato che vive per tutto il programma. Va benissimo per i kana e i kanji, che sono
//! tabelle dentro il binario; qui il contenuto e' una riga scritta dall'utente in
//! SQLite, e per leggerla ci vuole un'attesa.
//!
//! La via scelta e' la meno invasiva: le flashcard non entrano in quel tratto, e chi
//! le studia passa la carta gia' letta. Cambiare il tratto per farcele stare vorrebbe
//! dire toccare il giro di kana e kanji per una materia sola, che e' il contrario di
//! quello che il progetto fa quando generalizza.
//!
//! Quello che invece si riusa **e' tutto il resto**: `Question`, `Verdict` e `Prompt`
//! sono gli stessi tipi che attraversano il confine per le altre materie, quindi la
//! schermata di studio condivisa non ha dovuto imparare niente di nuovo.

use serde::{Deserialize, Serialize};

use crate::features::flashcards::deck::{self, Flashcard};
use crate::shared::exercise::{Answer, AnswerFormat, ExerciseTypeId, Prompt, Question, Verdict};
use crate::shared::text;

/// In che verso va la domanda.
///
/// Sono **due carte di studio distinte**, non due modi di guardare la stessa: si
/// imparano in tempi diversi, perche' riconoscere e produrre sono due abilita'
/// diverse. E' la stessa ragione per cui sui kana il riconoscimento e la scrittura
/// hanno due righe separate in `cards`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Si vede il giapponese e si risponde col significato.
    JpToMeaning,
    /// Si vede il significato e si scrive il giapponese.
    MeaningToJp,
}

impl Direction {
    const JP_TO_MEANING: ExerciseTypeId = ExerciseTypeId::new("flashcard.jp_to_meaning");
    const MEANING_TO_JP: ExerciseTypeId = ExerciseTypeId::new("flashcard.meaning_to_jp");

    /// Il nome con cui questa direzione finisce nell'archivio.
    pub fn exercise_id(self) -> ExerciseTypeId {
        match self {
            Self::JpToMeaning => Self::JP_TO_MEANING,
            Self::MeaningToJp => Self::MEANING_TO_JP,
        }
    }

    /// Da un nome riletto dall'archivio alla direzione, se e' una delle nostre.
    pub fn from_exercise(id: &ExerciseTypeId) -> Option<Self> {
        match id.as_str() {
            s if s == Self::JP_TO_MEANING.as_str() => Some(Self::JpToMeaning),
            s if s == Self::MEANING_TO_JP.as_str() => Some(Self::MeaningToJp),
            _ => None,
        }
    }

    /// Cosa si vede.
    fn prompt(self, card: &Flashcard) -> Prompt {
        match self {
            // Il significato attraversa il confine come testo latino. E' un limite
            // dichiarato: chi si scrivesse una carta col significato in giapponese lo
            // vedrebbe nel font di ripiego. Non vale un terzo alfabeto adesso, perche'
            // di quelle carte non ne esiste ancora nessuna.
            Self::JpToMeaning => Prompt::Japanese(card.japanese.clone()),
            Self::MeaningToJp => Prompt::Latin(card.meaning.clone()),
        }
    }

    /// Cosa si vuole sapere.
    ///
    /// E' **un'etichetta da mappare, non testo da mostrare**: il core dice `meaning`,
    /// l'interfaccia decide che si scrive «Meaning». La direzione e' fissa per tutta
    /// la sessione e la schermata la conosce gia', ma una domanda deve dire da sola
    /// cosa chiede: e' la stessa regola che vale per `asks` sui kanji.
    fn asks(self) -> &'static str {
        match self {
            Self::JpToMeaning => "meaning",
            Self::MeaningToJp => "japanese",
        }
    }

    /// Tutte le risposte che questo verso accetta, la principale per prima.
    ///
    /// # Perche' ogni campo serve alla sua direzione, e a una sola
    ///
    /// Gli altri **significati** valgono dove si risponde col significato: sono
    /// traduzioni, e una traduzione non e' una risposta a «scrivi questa parola in
    /// giapponese». Il **furigana** vale dove si risponde in giapponese: e' la stessa
    /// parola scritta in kana, e non e' una traduzione.
    ///
    /// Mescolarli farebbe passare «hello» a una domanda che chiede こんにちは, e non
    /// sarebbe indulgenza: sarebbe non aver chiesto niente.
    fn accepted(self, card: &Flashcard) -> Vec<&str> {
        match self {
            Self::JpToMeaning => std::iter::once(card.meaning.as_str())
                .chain(card.alternatives.iter().map(String::as_str))
                .collect(),
            Self::MeaningToJp => std::iter::once(card.japanese.as_str())
                .chain(card.furigana.as_deref())
                .collect(),
        }
    }
}

/// La domanda su una carta, nel verso scelto.
pub fn question(card: &Flashcard, direction: Direction) -> Question {
    Question {
        exercise_type: direction.exercise_id(),
        item: deck::item_id(&card.id),
        prompt: direction.prompt(card),
        format: AnswerFormat::Input,
        asks: Some(direction.asks().to_owned()),
        // Lo stimolo e' tutto quello che si vede: qui non c'e' un pezzo che fa da
        // contesto, come l'okurigana sui kanji.
        focus: None,
    }
}

/// Giudica una risposta.
///
/// Il confronto passa da [`text::normalize_input`], cioe' la stessa pulizia che usa
/// la scrittura sui kana: NFKC, niente spazi, e **il sillabario resta quello che e'**.
/// Il lato giapponese di una carta e' una **forma scritta** che l'utente ha scelto, e
/// far passare ネコ per ねこ vorrebbe dire decidere al posto suo che le due grafie sono
/// la stessa cosa.
///
/// Gli spazi cadono su tutti e due i lati, quindi una frase scritta attaccata o
/// separata per parole vale uguale.
///
/// Le risposte accettate possono essere piu' d'una, e quali siano lo decide il **verso**
/// della domanda: vedi [`Direction::accepted`]. Sbagliando si mostrano tutte, perche'
/// sapere che ce n'era anche un'altra fa parte della correzione.
pub fn grade(card: &Flashcard, direction: Direction, answer: &Answer) -> Verdict {
    let accepted = direction.accepted(card);
    let given = comparable(answer.as_str());

    if accepted.iter().any(|a| comparable(a) == given) {
        Verdict::correct()
    } else {
        Verdict::Incorrect {
            accepted: accepted.into_iter().map(str::to_owned).collect(),
        }
    }
}

/// La forma su cui si confronta.
///
/// NFKC, niente spazi, e **niente maiuscole**. Le maiuscole non sono una conoscenza da
/// verificare: chi risponde «Cat» a una carta che dice «cat» ha ricordato il
/// significato, e contarlo errore direbbe a FSRS che il ricordo e' debole quando il
/// problema non c'era. E' lo stesso ragionamento per cui i kanji non fanno digitare il
/// significato, cioe' che si vuole il senso e non l'ortografia.
///
/// Sul lato giapponese non cambia niente, perche' i kana e i kanji le maiuscole non le
/// hanno: vale su tutti e due i lati perche' una regola sola e' piu' facile da
/// spiegare di due, non perche' li' serva.
fn comparable(text: &str) -> String {
    text::normalize_input(text).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carta() -> Flashcard {
        Flashcard {
            id: "abc".into(),
            deck_id: "deck".into(),
            japanese: "ねこ".into(),
            meaning: "cat".into(),
            alternatives: Vec::new(),
            furigana: None,
        }
    }

    /// La carta dell'esempio: una parola in kanji, con la sua lettura e un secondo
    /// significato.
    fn nihongo() -> Flashcard {
        Flashcard {
            id: "abc".into(),
            deck_id: "deck".into(),
            japanese: "日本語".into(),
            meaning: "japanese".into(),
            alternatives: vec!["the japanese language".into()],
            furigana: Some("にほんご".into()),
        }
    }

    #[test]
    fn il_verso_decide_cosa_si_vede_e_cosa_si_chiede() {
        let q = question(&carta(), Direction::JpToMeaning);
        assert_eq!(q.prompt, Prompt::Japanese("ねこ".into()));
        assert_eq!(q.asks.as_deref(), Some("meaning"));
        assert_eq!(q.format, AnswerFormat::Input);

        let q = question(&carta(), Direction::MeaningToJp);
        assert_eq!(q.prompt, Prompt::Latin("cat".into()));
        assert_eq!(q.asks.as_deref(), Some("japanese"));
    }

    #[test]
    fn le_due_direzioni_sono_due_esercizi_distinti() {
        assert_ne!(
            Direction::JpToMeaning.exercise_id(),
            Direction::MeaningToJp.exercise_id()
        );
        for d in [Direction::JpToMeaning, Direction::MeaningToJp] {
            assert_eq!(Direction::from_exercise(&d.exercise_id()), Some(d));
        }
        assert_eq!(
            Direction::from_exercise(&ExerciseTypeId::new("kana.input")),
            None
        );
    }

    #[test]
    fn la_domanda_verte_sulla_carta_e_non_sul_mazzo() {
        let q = question(&carta(), Direction::JpToMeaning);
        assert_eq!(q.item.as_str(), "flashcard:abc");
    }

    #[test]
    fn si_giudica_nel_verso_giusto() {
        let c = carta();
        assert!(grade(&c, Direction::JpToMeaning, &Answer::new("cat")).is_correct());
        assert!(!grade(&c, Direction::JpToMeaning, &Answer::new("ねこ")).is_correct());

        assert!(grade(&c, Direction::MeaningToJp, &Answer::new("ねこ")).is_correct());
        assert!(!grade(&c, Direction::MeaningToJp, &Answer::new("cat")).is_correct());
    }

    #[test]
    fn gli_spazi_non_contano() {
        let c = Flashcard {
            japanese: "まいにち はしります".into(),
            meaning: "I run every day".into(),
            ..carta()
        };

        // Scritta attaccata o separata per parole e' la stessa risposta.
        assert!(
            grade(
                &c,
                Direction::MeaningToJp,
                &Answer::new("まいにちはしります")
            )
            .is_correct()
        );
        assert!(
            grade(
                &c,
                Direction::MeaningToJp,
                &Answer::new(" まいにち はしります ")
            )
            .is_correct()
        );
        assert!(grade(&c, Direction::JpToMeaning, &Answer::new("Irun every day")).is_correct());
    }

    #[test]
    fn le_maiuscole_non_contano() {
        let c = carta();
        assert!(grade(&c, Direction::JpToMeaning, &Answer::new("Cat")).is_correct());
        assert!(grade(&c, Direction::JpToMeaning, &Answer::new("CAT")).is_correct());
    }

    #[test]
    fn il_sillabario_conta() {
        // ネコ e ねこ sono due grafie diverse della stessa parola, e la carta ne ha
        // scelta una: non tocca all'app decidere che sono la stessa cosa.
        let c = carta();
        assert!(!grade(&c, Direction::MeaningToJp, &Answer::new("ネコ")).is_correct());
    }

    #[test]
    fn scrivendo_il_giapponese_vale_anche_il_furigana() {
        let c = nihongo();
        for risposta in ["日本語", "にほんご", "日本 語", " にほんご "] {
            assert!(
                grade(&c, Direction::MeaningToJp, &Answer::new(risposta)).is_correct(),
                "{risposta} doveva passare"
            );
        }

        // Il furigana e' quello scritto sulla carta, non una lettura qualunque: la
        // regola che il sillabario conta vale anche qui.
        assert!(!grade(&c, Direction::MeaningToJp, &Answer::new("ニホンゴ")).is_correct());
    }

    #[test]
    fn rispondendo_col_significato_valgono_tutti_i_significati() {
        let c = nihongo();
        for risposta in ["japanese", "Japanese", "the japanese language"] {
            assert!(
                grade(&c, Direction::JpToMeaning, &Answer::new(risposta)).is_correct(),
                "{risposta} doveva passare"
            );
        }
    }

    #[test]
    fn ogni_campo_serve_alla_sua_direzione_e_a_una_sola() {
        let c = nihongo();

        // Il furigana e' una parola giapponese, non una traduzione.
        assert!(!grade(&c, Direction::JpToMeaning, &Answer::new("にほんご")).is_correct());

        // Un significato non e' una risposta a «scrivi questa parola in giapponese».
        assert!(
            !grade(&c, Direction::MeaningToJp, &Answer::new("the japanese language"))
                .is_correct()
        );
        assert!(!grade(&c, Direction::MeaningToJp, &Answer::new("japanese")).is_correct());
    }

    #[test]
    fn senza_furigana_vale_solo_la_forma_scritta() {
        let c = Flashcard {
            furigana: None,
            ..nihongo()
        };
        assert!(grade(&c, Direction::MeaningToJp, &Answer::new("日本語")).is_correct());
        assert!(!grade(&c, Direction::MeaningToJp, &Answer::new("にほんご")).is_correct());
    }

    #[test]
    fn sbagliando_si_vedono_tutte_quelle_che_andavano_bene() {
        let c = nihongo();
        assert_eq!(
            grade(&c, Direction::MeaningToJp, &Answer::new("ちがう")),
            Verdict::Incorrect {
                accepted: vec!["日本語".into(), "にほんご".into()]
            }
        );
        assert_eq!(
            grade(&c, Direction::JpToMeaning, &Answer::new("wrong")),
            Verdict::Incorrect {
                accepted: vec!["japanese".into(), "the japanese language".into()]
            }
        );
    }

    #[test]
    fn sbagliando_si_vede_cosa_c_era_scritto() {
        let c = carta();
        assert_eq!(
            grade(&c, Direction::MeaningToJp, &Answer::new("いぬ")),
            Verdict::Incorrect {
                accepted: vec!["ねこ".into()]
            }
        );
    }
}
