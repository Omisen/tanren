//! Gli esercizi sui kana.
//!
//! Due tipi, che sono l'uno il rovescio dell'altro:
//!
//! - [`KanaRecognition`] mostra il segno e chiede la trascrizione, a scelta multipla.
//! - [`KanaInput`] mostra la trascrizione e chiede il segno, digitato con l'IME.
//!
//! # Il problema delle trascrizioni ambigue
//!
//! `じ` e `ぢ` si scrivono entrambe `ji`, `ず` e `づ` entrambe `zu`. Nella scelta
//! multipla non e' un problema, perche' la domanda parte dal segno. Nell'esercizio di
//! input invece la domanda parte dalla trascrizione, e chiedere «scrivi ji» sarebbe
//! una domanda senza una risposta sola.
//!
//! La soluzione e' in [`prompt_romaji`]: a ogni segno viene assegnata la prima delle
//! sue trascrizioni non ancora presa da un altro segno dello stesso sillabario.
//! Scorrendo la tabella nell'ordine tradizionale, `じ` prende `ji` e `ぢ` ripiega su
//! `di`, `ず` prende `zu` e `づ` ripiega su `du`. Nessun segno resta senza prompt e
//! nessun prompt vale per due segni.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::features::kana::data::{Kana, Syllabary, table};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{
    Answer, AnswerFormat, ExerciseType, ExerciseTypeId, ItemId, Prompt, Question, QuestionRequest,
    Verdict,
};
use crate::shared::text::normalize_input;

/// Prefisso degli identificatori prodotti da questa feature.
const NAMESPACE: &str = "kana";

/// Costruisce l'identificatore di un segno, nella forma `kana:hiragana:か`.
pub fn item_id(syllabary: Syllabary, character: &str) -> ItemId {
    ItemId::new(format!(
        "{NAMESPACE}:{}:{character}",
        syllabary_key(syllabary)
    ))
}

fn syllabary_key(syllabary: Syllabary) -> &'static str {
    match syllabary {
        Syllabary::Hiragana => "hiragana",
        Syllabary::Katakana => "katakana",
    }
}

/// Risale dall'identificatore al segno.
///
/// Un identificatore di un'altra materia non e' un dato corrotto: e' semplicemente
/// qualcosa di cui questo esercizio non si occupa, e viene distinto dal caso di un
/// segno inesistente.
fn resolve(id: &ItemId, exercise: &ExerciseTypeId) -> Result<(Syllabary, &'static Kana)> {
    let not_supported = || CoreError::ItemNotSupported {
        exercise: exercise.to_string(),
        id: id.to_string(),
    };

    let rest = id
        .as_str()
        .strip_prefix(NAMESPACE)
        .ok_or_else(not_supported)?;
    let rest = rest.strip_prefix(':').ok_or_else(not_supported)?;
    let (syllabary, character) = rest.split_once(':').ok_or_else(not_supported)?;

    let syllabary = match syllabary {
        "hiragana" => Syllabary::Hiragana,
        "katakana" => Syllabary::Katakana,
        _ => return Err(not_supported()),
    };

    let kana = table(syllabary)
        .all()
        .iter()
        .find(|k| k.character == character)
        .ok_or_else(|| CoreError::UnknownItem { id: id.to_string() })?;

    Ok((syllabary, kana))
}

/// La trascrizione da mostrare come domanda, garantita unica nel sillabario.
///
/// Vedi la nota sulle ambiguita' in cima al modulo.
pub fn prompt_romaji(syllabary: Syllabary, character: &str) -> Option<&'static str> {
    prompts(syllabary).get(character).copied()
}

static HIRAGANA_PROMPTS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| build_prompts(Syllabary::Hiragana));
static KATAKANA_PROMPTS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| build_prompts(Syllabary::Katakana));

fn prompts(syllabary: Syllabary) -> &'static HashMap<&'static str, &'static str> {
    match syllabary {
        Syllabary::Hiragana => &HIRAGANA_PROMPTS,
        Syllabary::Katakana => &KATAKANA_PROMPTS,
    }
}

fn build_prompts(syllabary: Syllabary) -> HashMap<&'static str, &'static str> {
    let mut presi: HashSet<&str> = HashSet::new();
    let mut mappa = HashMap::new();

    for kana in table(syllabary).all() {
        // L'ordine della tabella e' quello tradizionale, quindi la trascrizione
        // canonica va al segno che viene prima: じ prende ji, ぢ ripiega su di.
        let scelta = kana
            .romaji
            .iter()
            .map(String::as_str)
            .find(|r| !presi.contains(r))
            .unwrap_or_else(|| {
                panic!(
                    "{} non ha nessuna trascrizione libera: la tabella va corretta",
                    kana.character
                )
            });
        presi.insert(scelta);
        mappa.insert(kana.character.as_str(), scelta);
    }

    mappa
}

/// Mostra il segno, si sceglie la trascrizione.
pub struct KanaRecognition;

impl KanaRecognition {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kana.recognition");
}

impl ExerciseType for KanaRecognition {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, rng: &mut dyn Rng) -> Result<Question> {
        let (_, kana) = resolve(request.item, &Self::ID)?;
        let corretta = canonical(kana);

        let mut options = distractor_romaji(&request, corretta, rng);
        options.push(corretta.to_string());
        options.shuffle(rng);

        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(kana.character.clone()),
            format: AnswerFormat::Choice { options },
            // Visto un kana c'e' una cosa sola da chiedere, quindi non c'e' niente da
            // precisare. Un kanji invece ha piu' famiglie di letture e deve dirlo.
            asks: None,
        })
    }

    fn grade(&self, item: &ItemId, answer: &Answer) -> Result<Verdict> {
        let (_, kana) = resolve(item, &Self::ID)?;
        let dato = answer.as_str().trim().to_ascii_lowercase();

        // Vengono accettate tutte le trascrizioni note del segno, non solo quella
        // canonica: chi scrive `si` per し ha capito il segno.
        Ok(if kana.romaji.contains(&dato) {
            Verdict::correct()
        } else {
            Verdict::Incorrect {
                accepted: kana.romaji.clone(),
            }
        })
    }
}

/// Mostra la trascrizione, si digita il segno con l'IME.
pub struct KanaInput;

impl KanaInput {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kana.input");
}

impl ExerciseType for KanaInput {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, _rng: &mut dyn Rng) -> Result<Question> {
        let (syllabary, kana) = resolve(request.item, &Self::ID)?;
        let romaji =
            prompt_romaji(syllabary, &kana.character).ok_or_else(|| CoreError::UnknownItem {
                id: request.item.to_string(),
            })?;

        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Latin(romaji.to_string()),
            format: AnswerFormat::Input,
            asks: None,
        })
    }

    fn grade(&self, item: &ItemId, answer: &Answer) -> Result<Verdict> {
        let (_, kana) = resolve(item, &Self::ID)?;

        // Qui si usa `normalize_input` e non `normalize_reading`: la domanda chiede un
        // sillabario preciso, quindi rispondere か a una domanda su カ e' sbagliato.
        // La pulizia Unicode serve lo stesso, perche' l'IME puo' restituire katakana a
        // mezza larghezza o segni di sonorizzazione staccati.
        Ok(
            if normalize_input(answer.as_str()) == normalize_input(&kana.character) {
                Verdict::correct()
            } else {
                Verdict::Incorrect {
                    accepted: vec![kana.character.clone()],
                }
            },
        )
    }
}

fn canonical(kana: &Kana) -> &str {
    // La tabella garantisce almeno una trascrizione per segno, e c'e' un test che lo
    // verifica su tutte le voci.
    kana.romaji
        .first()
        .map(String::as_str)
        .expect("segno senza trascrizione")
}

/// Pesca i distrattori dal pool, preferendo segni della stessa famiglia.
///
/// Un distrattore preso dalla stessa famiglia somiglia alla risposta giusta e rende
/// la scelta istruttiva; uno preso a caso da tutto il sillabario la rende banale.
fn distractor_romaji(
    request: &QuestionRequest<'_>,
    corretta: &str,
    rng: &mut dyn Rng,
) -> Vec<String> {
    let Ok((_, atteso)) = resolve(request.item, &KanaRecognition::ID) else {
        return Vec::new();
    };

    let mut simili = Vec::new();
    let mut altri = Vec::new();

    for id in request.pool {
        if id == request.item {
            continue;
        }
        // Gli elementi di altre materie finiti nel pool vengono semplicemente
        // ignorati: una sessione mista non deve rompersi qui.
        let Ok((_, kana)) = resolve(id, &KanaRecognition::ID) else {
            continue;
        };
        if kana.group == atteso.group {
            simili.push(kana);
        } else {
            altri.push(kana);
        }
    }

    simili.shuffle(rng);
    altri.shuffle(rng);

    let mut visti = HashSet::from([corretta]);
    let mut scelti = Vec::with_capacity(request.distractors);

    for kana in simili.into_iter().chain(altri) {
        if scelti.len() == request.distractors {
            break;
        }
        let romaji = canonical(kana);
        // Due segni possono condividere la trascrizione canonica, じ e ぢ per
        // esempio: un'opzione ripetuta sarebbe una scelta senza senso.
        if visti.insert(romaji) {
            scelti.push(romaji.to_string());
        }
    }

    scelti
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn hiragana(character: &str) -> ItemId {
        item_id(Syllabary::Hiragana, character)
    }

    fn pool(syllabary: Syllabary) -> Vec<ItemId> {
        table(syllabary)
            .all()
            .iter()
            .map(|k| item_id(syllabary, &k.character))
            .collect()
    }

    #[test]
    fn ogni_segno_ha_un_prompt_e_nessuno_lo_condivide() {
        for syllabary in [Syllabary::Hiragana, Syllabary::Katakana] {
            let mut visti = HashSet::new();
            for kana in table(syllabary).all() {
                let p = prompt_romaji(syllabary, &kana.character)
                    .unwrap_or_else(|| panic!("{} senza prompt", kana.character));
                assert!(visti.insert(p), "prompt ripetuto: {p}");
            }
            assert_eq!(visti.len(), 107);
        }
    }

    #[test]
    fn le_trascrizioni_ambigue_vengono_sciolte() {
        let p = |c| prompt_romaji(Syllabary::Hiragana, c).unwrap();
        // Chi viene prima nella tabella tiene la trascrizione canonica.
        assert_eq!(p("じ"), "ji");
        assert_eq!(p("ぢ"), "di");
        assert_eq!(p("ず"), "zu");
        assert_eq!(p("づ"), "du");
        assert_eq!(p("じゃ"), "ja");
        assert_eq!(p("ぢゃ"), "dya");
    }

    #[test]
    fn il_riconoscimento_mostra_il_segno_e_offre_le_scelte() {
        let item = hiragana("か");
        let pool = pool(Syllabary::Hiragana);
        let q = KanaRecognition
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &pool,
                    distractors: 3,
                },
                &mut rng(),
            )
            .unwrap();

        assert_eq!(q.prompt, Prompt::Japanese("か".into()));
        let AnswerFormat::Choice { options } = q.format else {
            panic!("ci si aspettava una scelta multipla");
        };
        assert_eq!(options.len(), 4);
        assert!(options.contains(&"ka".to_string()));
        let uniche: HashSet<_> = options.iter().collect();
        assert_eq!(uniche.len(), options.len(), "opzioni ripetute");
    }

    #[test]
    fn i_distrattori_escono_solo_dal_pool() {
        let item = hiragana("か");
        // Un pool ristretto alla sola riga a piu' il segno chiesto.
        let mut ristretto: Vec<ItemId> = ["あ", "い", "う", "え", "お"]
            .iter()
            .map(|c| hiragana(c))
            .collect();
        ristretto.push(item.clone());

        let q = KanaRecognition
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &ristretto,
                    distractors: 3,
                },
                &mut rng(),
            )
            .unwrap();

        let AnswerFormat::Choice { options } = q.format else {
            unreachable!()
        };
        let ammesse: HashSet<&str> = ["a", "i", "u", "e", "o", "ka"].into_iter().collect();
        for o in &options {
            assert!(ammesse.contains(o.as_str()), "opzione fuori dal pool: {o}");
        }
    }

    #[test]
    fn un_pool_troppo_piccolo_non_fa_saltare_nulla() {
        let item = hiragana("か");
        let piccolo = vec![item.clone(), hiragana("き")];
        let q = KanaRecognition
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &piccolo,
                    distractors: 5,
                },
                &mut rng(),
            )
            .unwrap();

        let AnswerFormat::Choice { options } = q.format else {
            unreachable!()
        };
        // Un solo distrattore disponibile, piu' la risposta giusta.
        assert_eq!(options.len(), 2);
    }

    #[test]
    fn a_parita_di_seme_la_domanda_e_identica() {
        let item = hiragana("か");
        let pool = pool(Syllabary::Hiragana);
        let request = QuestionRequest {
            item: &item,
            pool: &pool,
            distractors: 3,
        };
        let a = KanaRecognition.question(request, &mut rng()).unwrap();
        let b = KanaRecognition.question(request, &mut rng()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn il_riconoscimento_accetta_tutte_le_trascrizioni_note() {
        let shi = hiragana("し");
        for risposta in ["shi", "si", " SHI ", "Shi"] {
            assert!(
                KanaRecognition
                    .grade(&shi, &Answer::new(risposta))
                    .unwrap()
                    .is_correct(),
                "rifiutata: {risposta}"
            );
        }
    }

    #[test]
    fn il_riconoscimento_sbagliato_dice_cosa_si_aspettava() {
        let shi = hiragana("し");
        let v = KanaRecognition.grade(&shi, &Answer::new("chi")).unwrap();
        assert_eq!(
            v,
            Verdict::Incorrect {
                accepted: vec!["shi".into(), "si".into()],
            }
        );
    }

    #[test]
    fn l_input_mostra_la_trascrizione_e_attende_la_digitazione() {
        let item = hiragana("か");
        let q = KanaInput
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &[],
                    distractors: 3,
                },
                &mut rng(),
            )
            .unwrap();

        assert_eq!(q.prompt, Prompt::Latin("ka".into()));
        assert_eq!(q.format, AnswerFormat::Input);
    }

    #[test]
    fn l_input_pretende_il_sillabario_giusto() {
        let hira = hiragana("か");
        let kata = item_id(Syllabary::Katakana, "カ");

        assert!(
            KanaInput
                .grade(&hira, &Answer::new("か"))
                .unwrap()
                .is_correct()
        );
        assert!(
            KanaInput
                .grade(&kata, &Answer::new("カ"))
                .unwrap()
                .is_correct()
        );

        // Il suono e' giusto ma la grafia no: la domanda chiedeva l'altro sillabario.
        assert!(
            !KanaInput
                .grade(&hira, &Answer::new("カ"))
                .unwrap()
                .is_correct()
        );
        assert!(
            !KanaInput
                .grade(&kata, &Answer::new("か"))
                .unwrap()
                .is_correct()
        );
    }

    #[test]
    fn l_input_sopporta_quello_che_esce_dall_ime() {
        let kata = item_id(Syllabary::Katakana, "ガ");
        // Katakana a mezza larghezza con il segno di sonorizzazione staccato, e spazi
        // di troppo: casi che un IME produce davvero.
        for risposta in ["ガ", " ガ ", "ｶﾞ", "カ\u{3099}"] {
            assert!(
                KanaInput
                    .grade(&kata, &Answer::new(risposta))
                    .unwrap()
                    .is_correct(),
                "rifiutata: {risposta}"
            );
        }
    }

    #[test]
    fn un_segno_inesistente_e_un_errore_diverso_da_una_materia_estranea() {
        let inventato = hiragana("X");
        assert!(matches!(
            KanaInput.grade(&inventato, &Answer::new("か")),
            Err(CoreError::UnknownItem { .. })
        ));

        let altra_materia = ItemId::new("kanji:日");
        assert!(matches!(
            KanaInput.grade(&altra_materia, &Answer::new("ひ")),
            Err(CoreError::ItemNotSupported { .. })
        ));
    }

    #[test]
    fn i_tipi_di_esercizio_stanno_dietro_un_trait_object() {
        // E' la promessa architetturale di questo passaggio: una sessione puo' tenere
        // insieme esercizi diversi senza conoscerne il tipo concreto.
        let tipi: Vec<Box<dyn ExerciseType>> = vec![Box::new(KanaRecognition), Box::new(KanaInput)];
        let item = hiragana("か");
        let pool = pool(Syllabary::Hiragana);

        for tipo in &tipi {
            let q = tipo
                .question(
                    QuestionRequest {
                        item: &item,
                        pool: &pool,
                        distractors: 3,
                    },
                    &mut rng(),
                )
                .unwrap();
            assert_eq!(q.exercise_type, tipo.id());
            assert_eq!(q.item, item);
        }

        let ids: Vec<_> = tipi.iter().map(|t| t.id().to_string()).collect();
        assert_eq!(ids, ["kana.recognition", "kana.input"]);
    }
}
