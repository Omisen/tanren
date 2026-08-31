//! I kanji come item di studio: le faccette.
//!
//! # L'introduzione e' per kanji, lo scheduling e' per faccetta
//!
//! Si impara **un kanji alla volta come blocco**: forma, significato, letture, esempi.
//! Ma ricordare il significato di 生 e ricordare che si legge セイ sono due ricordi
//! diversi, che maturano in tempi diversi, quindi ognuno ha la sua carta e la sua
//! scadenza. E' la ragione per cui la chiave dell'archivio e' `(item_id,
//! exercise_type)` e non il solo item: quella forma, scelta per i kana, qui paga.
//!
//! | faccetta | cosa si vede | come si risponde | cosa si accetta |
//! |---|---|---|---|
//! | [`Facet::Meaning`] | 生 | scelta multipla | i suoi significati |
//! | [`Facet::On`] | 生, dicendo «on» | si digita | セイ, ショウ |
//! | [`Facet::Kun`] | 生, dicendo «kun» | si digita | き, なま |
//! | [`Facet::Okurigana`] | 生きる, che dice gia' tutto | si digita | い |
//!
//! # Perche' l'okurigana e' un item a se' e non una faccetta del kanji
//!
//! Perche' «la lettura kun di 生» non e' una domanda: 生 ne ha dieci. 生きる ne ha una.
//! L'okurigana **e' il disambiguatore**, e non e' un espediente: e' l'informazione che
//! porta leggendo un testo vero. Quindi la forma scritta e' l'item, e il kanji nudo
//! resta per le letture kun che stanno da sole (なま, き).
//!
//! # Cosa si chiede e cosa no
//!
//! Si chiedono le **forme comuni**, cioe' quelle che il dato marca come parole che si
//! incontrano davvero. Le altre restano visibili nella scheda del kanji ma non
//! diventano carte: 下 ha sette forme con l'okurigana, e pretenderle tutte mature
//! prima di considerare imparato 下 vorrebbe dire non impararlo mai. I nanori non si
//! chiedono affatto.
//!
//! # La severita' sulla grafia e' morbida
//!
//! Il dizionario scrive le letture on in katakana e le kun in hiragana, ed e' una
//! convenzione che vale la pena insegnare. Ma chi risponde いち a una domanda su イチ
//! **ha ricordato la lettura**: contarlo come errore direbbe a FSRS che il ricordo e'
//! debole, quando il problema era l'ortografia. Quindi la risposta e' giusta e si
//! aggiunge un rilievo ([`Note`]), che non tocca il giudizio.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::features::kanji::levels::{Kanji, Level, level_of_character, table};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{
    Answer, AnswerFormat, ExerciseType, ExerciseTypeId, ItemId, Note, Prompt, Question,
    QuestionRequest, Verdict,
};
use crate::shared::text::normalize_reading;

/// Prefisso degli identificatori prodotti da questa feature.
const NAMESPACE: &str = "kanji";

/// Che cosa si sta chiedendo di un item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facet {
    /// Il significato primario, a scelta multipla.
    Meaning,
    /// La lettura on, digitata.
    On,
    /// La lettura kun del kanji nudo, digitata.
    Kun,
    /// La lettura di una forma scritta col suo okurigana, digitata.
    Okurigana,
}

/// Le faccette che si possono chiedere di un kanji nudo, in ordine di introduzione.
pub const KANJI_FACETS: [Facet; 3] = [Facet::Meaning, Facet::On, Facet::Kun];

static MEANING: MeaningFacet = MeaningFacet;
static ON: OnFacet = OnFacet;
static KUN: KunFacet = KunFacet;
static OKURIGANA: OkuriganaFacet = OkuriganaFacet;

impl Facet {
    /// Il tipo di esercizio che le corrisponde, cioe' la chiave con cui l'archivio
    /// tiene la sua carta.
    pub fn exercise_id(self) -> ExerciseTypeId {
        self.exercise().id()
    }

    pub fn exercise(self) -> &'static dyn ExerciseType {
        match self {
            Self::Meaning => &MEANING,
            Self::On => &ON,
            Self::Kun => &KUN,
            Self::Okurigana => &OKURIGANA,
        }
    }

    fn from_exercise(id: &ExerciseTypeId) -> Option<Self> {
        [Self::Meaning, Self::On, Self::Kun, Self::Okurigana]
            .into_iter()
            .find(|f| f.exercise_id() == *id)
    }
}

/// Un item di studio: una forma scritta, una faccetta, una carta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub facet: Facet,
    /// La forma da mostrare: il kanji, o il kanji col suo okurigana.
    pub form: String,
}

/// Costruisce l'identificatore di una forma, nella forma `kanji:生`.
///
/// # Perche' il livello **non** sta qui dentro
///
/// Perche' non fa parte dell'identita' di un kanji: e' una nostra decisione editoriale
/// su dove metterlo nel percorso, e le decisioni cambiano. Un identificatore invece si
/// scrive nell'archivio e ci resta per sempre.
///
/// Ci stava, ed e' stato tolto dopo averne misurato il costo: il riordino che ha
/// sganciato il percorso da una lista proprietaria ha spostato di livello il **97% dei
/// kanji**. Con uno storico vero sarebbe sparito quasi tutto, e in silenzio, perche'
/// una carta che non si risolve viene ignorata senza dire niente.
///
/// Quale tabella aprire lo dice ora [`level_of`], che lo chiede all'indice.
///
/// **La faccetta non sta qui**: sta nel tipo di esercizio, che e' l'altra meta' della
/// chiave dell'archivio. Cosi' le tre faccette di 生 condividono lo stesso item e si
/// vede che parlano dello stesso kanji.
pub fn item_id(form: &str) -> ItemId {
    ItemId::new(format!("{NAMESPACE}:{form}"))
}

/// Tutti gli item di un livello, kanji per kanji.
///
/// Per ogni kanji: il significato, la lettura on se ne ha, la lettura kun nuda se ne
/// ha, e una forma per ogni okurigana **comune**.
pub fn items(level: Level) -> Vec<Item> {
    let mut out = Vec::new();

    for kanji in table(level).all() {
        let id = item_id(&kanji.character);
        for facet in KANJI_FACETS {
            if has_facet(kanji, facet) {
                out.push(Item {
                    id: id.clone(),
                    facet,
                    form: kanji.character.clone(),
                });
            }
        }
        for oku in kanji.okurigana.iter().filter(|o| o.common) {
            out.push(Item {
                id: item_id(&oku.form),
                facet: Facet::Okurigana,
                form: oku.form.clone(),
            });
        }
    }

    out
}

fn has_facet(kanji: &Kanji, facet: Facet) -> bool {
    match facet {
        Facet::Meaning => !kanji.meanings.is_empty(),
        Facet::On => !kanji.on.is_empty(),
        Facet::Kun => !kanji.kun.is_empty(),
        Facet::Okurigana => false,
    }
}

/// Cosa una forma risolve: il kanji a cui appartiene, e l'okurigana se ce l'ha.
struct Resolved {
    kanji: &'static Kanji,
    /// La forma per intero, cioe' l'identificatore senza il resto.
    form: String,
}

impl Resolved {
    /// Le letture accettate per una faccetta, nella grafia del dizionario.
    fn accepted(&self, facet: Facet) -> Vec<String> {
        match facet {
            Facet::Meaning => self.kanji.meanings.clone(),
            Facet::On => [&self.kanji.on[..], &self.kanji.on_rare[..]].concat(),
            Facet::Kun => [&self.kanji.kun[..], &self.kanji.kun_rare[..]].concat(),
            // La sola parte coperta dal kanji, non la parola intera: l'okurigana
            // e' gia' sotto gli occhi di chi risponde, quindi chiederlo sarebbe
            // chiedere di ricopiarlo. Vedi `Okurigana::stem`.
            Facet::Okurigana => self
                .kanji
                .okurigana
                .iter()
                .find(|o| o.form == self.form)
                .map(|o| o.stem.clone())
                .unwrap_or_default(),
        }
    }
}

/// Risale dall'identificatore alla forma e al suo kanji.
fn resolve(id: &ItemId, exercise: &ExerciseTypeId) -> Result<Resolved> {
    let not_supported = || CoreError::ItemNotSupported {
        exercise: exercise.to_string(),
        id: id.to_string(),
    };
    let unknown = || CoreError::UnknownItem { id: id.to_string() };

    let form = id
        .as_str()
        .strip_prefix(NAMESPACE)
        .and_then(|r| r.strip_prefix(':'))
        .ok_or_else(not_supported)?;

    // Il kanji e' il primo carattere: da solo per le faccette del kanji nudo, seguito
    // dall'okurigana per le altre. Il livello lo dice l'indice, non l'identificatore.
    let first = form.chars().next().ok_or_else(unknown)?;
    let level = level_of_character(first).ok_or_else(unknown)?;
    let kanji = table(level)
        .all()
        .iter()
        .find(|k| k.character.chars().eq(std::iter::once(first)))
        .ok_or_else(unknown)?;

    Ok(Resolved {
        kanji,
        form: form.to_owned(),
    })
}

/// Se un testo e' scritto tutto in katakana, ignorando cio' che kana non e'.
fn is_katakana(s: &str) -> bool {
    let mut visto = false;
    for c in s.chars() {
        if ('\u{3041}'..='\u{3096}').contains(&c) {
            return false;
        }
        if ('\u{30A1}'..='\u{30FA}').contains(&c) {
            visto = true;
        }
    }
    visto
}

/// Giudica una risposta digitata, con la severita' morbida sulla grafia.
///
/// `wants_katakana` dice quale grafia vuole la convenzione. Chi sbaglia solo quella ha
/// comunque risposto: si aggiunge un rilievo e basta.
fn grade_reading(accepted: &[String], answer: &Answer, wants_katakana: bool) -> Verdict {
    let dato = normalize_reading(answer.as_str());
    let Some(giusta) = accepted.iter().find(|r| normalize_reading(r) == dato) else {
        return Verdict::Incorrect {
            accepted: accepted.to_vec(),
        };
    };

    let scritta_in_katakana = is_katakana(answer.as_str().trim());
    let note = (scritta_in_katakana != wants_katakana).then(|| Note {
        kind: if wants_katakana {
            "on_in_hiragana"
        } else {
            "kun_in_katakana"
        }
        .to_owned(),
        expected: giusta.clone(),
    });

    Verdict::Correct { note }
}

/// Il significato primario, a scelta multipla.
pub struct MeaningFacet;

impl MeaningFacet {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kanji.meaning");
}

impl ExerciseType for MeaningFacet {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, rng: &mut dyn Rng) -> Result<Question> {
        let item = resolve(request.item, &Self::ID)?;
        let corretta = primary(&item.kanji.meanings, request.item)?.to_owned();

        // I distrattori sono i significati primari di altri kanji dell'ambito, e si
        // confrontano sul valore: due kanji possono condividere un significato, e uno
        // che fosse accettato renderebbe la domanda a due risposte buone.
        let escluse: Vec<String> = item
            .accepted(Facet::Meaning)
            .iter()
            .map(|m| fold(m))
            .collect();
        let mut viste = escluse;
        let mut options = Vec::new();

        let mut pool: Vec<&ItemId> = request.pool.iter().collect();
        pool.shuffle(rng);
        for altro in pool {
            if options.len() == request.distractors {
                break;
            }
            let Ok(altro) = resolve(altro, &Self::ID) else {
                continue;
            };
            let Some(m) = altro.kanji.meanings.first() else {
                continue;
            };
            if viste.contains(&fold(m)) {
                continue;
            }
            viste.push(fold(m));
            options.push(m.clone());
        }

        options.push(corretta);
        options.shuffle(rng);

        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(item.kanji.character.clone()),
            asks: Some("meaning".to_owned()),
            focus: None,
            format: AnswerFormat::Choice { options },
        })
    }

    fn grade(&self, id: &ItemId, answer: &Answer) -> Result<Verdict> {
        let item = resolve(id, &Self::ID)?;
        let accepted = item.accepted(Facet::Meaning);
        let dato = fold(answer.as_str());

        // Si accetta qualunque significato del kanji, non solo il primario: chi
        // risponde «be born» a 生 lo ha capito.
        Ok(if accepted.iter().any(|m| fold(m) == dato) {
            Verdict::correct()
        } else {
            Verdict::Incorrect { accepted }
        })
    }
}

/// La lettura on, digitata.
pub struct OnFacet;

impl OnFacet {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kanji.on");
}

impl ExerciseType for OnFacet {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, _rng: &mut dyn Rng) -> Result<Question> {
        let item = resolve(request.item, &Self::ID)?;
        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(item.kanji.character.clone()),
            asks: Some("on".to_owned()),
            focus: None,
            format: AnswerFormat::Input,
        })
    }

    fn grade(&self, id: &ItemId, answer: &Answer) -> Result<Verdict> {
        let item = resolve(id, &Self::ID)?;
        Ok(grade_reading(&item.accepted(Facet::On), answer, true))
    }
}

/// La lettura kun del kanji nudo, digitata.
pub struct KunFacet;

impl KunFacet {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kanji.kun");
}

impl ExerciseType for KunFacet {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, _rng: &mut dyn Rng) -> Result<Question> {
        let item = resolve(request.item, &Self::ID)?;
        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(item.kanji.character.clone()),
            asks: Some("kun".to_owned()),
            focus: None,
            format: AnswerFormat::Input,
        })
    }

    fn grade(&self, id: &ItemId, answer: &Answer) -> Result<Verdict> {
        let item = resolve(id, &Self::ID)?;
        Ok(grade_reading(&item.accepted(Facet::Kun), answer, false))
    }
}

/// La lettura di una forma scritta col suo okurigana.
pub struct OkuriganaFacet;

impl OkuriganaFacet {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kanji.okurigana");
}

impl ExerciseType for OkuriganaFacet {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, _rng: &mut dyn Rng) -> Result<Question> {
        let item = resolve(request.item, &Self::ID)?;
        if item.accepted(Facet::Okurigana).is_empty() {
            return Err(CoreError::UnknownItem {
                id: request.item.to_string(),
            });
        }

        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(item.form.clone()),
            // **Correzione da una prova d'uso.** Qui c'era scritto che l'okurigana
            // dice gia' da se' cosa si vuole, e non e' vero: visto 大いに nudo, senza
            // una riga che lo dica, non si capisce che si chiede la lettura di 大 e
            // non quella della parola. Era l'unica faccetta senza etichetta.
            asks: Some("okurigana".to_owned()),
            // La parte scritta col kanji, cioe' quella che si legge. Il resto e'
            // contesto e chi mostra la domanda lo attenua invece di stamparlo uguale.
            focus: Some(item.kanji.character.clone()),
            format: AnswerFormat::Input,
        })
    }

    fn grade(&self, id: &ItemId, answer: &Answer) -> Result<Verdict> {
        let item = resolve(id, &Self::ID)?;
        let accepted = item.accepted(Facet::Okurigana);
        if accepted.is_empty() {
            return Err(CoreError::UnknownItem { id: id.to_string() });
        }
        Ok(grade_reading(&accepted, answer, false))
    }
}

/// Il primo elemento di una lista che il dato garantisce non vuota.
fn primary<'a>(values: &'a [String], id: &ItemId) -> Result<&'a str> {
    values
        .first()
        .map(String::as_str)
        .ok_or_else(|| CoreError::UnknownItem { id: id.to_string() })
}

/// Come si confrontano due significati: senza maiuscole e senza spazi ai bordi.
fn fold(meaning: &str) -> String {
    meaning.trim().to_lowercase()
}

/// Il livello a cui si studia un item.
///
/// Non e' scritto nell'identificatore: si chiede all'indice, partendo dal kanji che
/// apre la forma. `None` se l'identificatore non e' di questa materia, o se quel kanji
/// non e' fra i joyo, ed e' il modo di riconoscere quello che non ci riguarda senza
/// doverlo risolvere per intero.
pub fn level_of(id: &ItemId) -> Option<Level> {
    id.as_str()
        .strip_prefix(NAMESPACE)
        .and_then(|r| r.strip_prefix(':'))
        .and_then(|form| form.chars().next())
        .and_then(level_of_character)
}

/// Se un item esiste ancora davvero nel contenuto.
///
/// Serve a chi rilegge lo storico: una carta scritta mesi fa nomina una forma che il
/// dato di oggi potrebbe non avere piu', per esempio se una lettura con okurigana
/// sparisse da una rigenerazione. Quella carta va **ignorata in silenzio**, non deve
/// far fallire l'avvio di un giro.
pub fn resolves(id: &ItemId, facet: Facet) -> bool {
    match resolve(id, &facet.exercise_id()) {
        Ok(item) => !item.accepted(facet).is_empty(),
        Err(_) => false,
    }
}

/// L'esercizio che sa fare una domanda, se e' di questa materia.
pub fn exercise_for(id: &ExerciseTypeId) -> Option<&'static dyn ExerciseType> {
    Facet::from_exercise(id).map(Facet::exercise)
}

/// La faccetta che un tipo di esercizio rappresenta, se e' di questa materia.
pub fn facet_of(exercise: &ExerciseTypeId) -> Option<Facet> {
    Facet::from_exercise(exercise)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::exercise::AnswerFormat;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    /// 生 sta al livello 5, e per tutta la sua ricchezza di letture e' il caso peggiore.
    fn vita() -> Level {
        Level::new(5).unwrap()
    }

    fn id(form: &str) -> ItemId {
        item_id(form)
    }

    #[test]
    fn un_kanji_diventa_piu_item_uno_per_faccetta() {
        let suoi: Vec<Item> = items(vita())
            .into_iter()
            .filter(|i| i.form.starts_with('生'))
            .collect();

        let facce: Vec<Facet> = suoi.iter().map(|i| i.facet).collect();
        assert!(facce.contains(&Facet::Meaning));
        assert!(facce.contains(&Facet::On));
        assert!(facce.contains(&Facet::Kun), "き e なま stanno da sole");

        let forme: Vec<&str> = suoi
            .iter()
            .filter(|i| i.facet == Facet::Okurigana)
            .map(|i| i.form.as_str())
            .collect();
        assert!(forme.contains(&"生きる"));
        assert!(forme.contains(&"生える"));
        assert!(
            !forme.contains(&"生ける"),
            "le forme che non si incontrano restano nella scheda, non fra le carte"
        );
    }

    #[test]
    fn le_faccette_del_kanji_nudo_condividono_l_item() {
        let suoi: Vec<Item> = items(vita())
            .into_iter()
            .filter(|i| i.form == "生")
            .collect();
        assert!(suoi.len() >= 3);
        assert!(
            suoi.iter().all(|i| i.id == suoi[0].id),
            "e' lo stesso kanji: cambia il tipo di esercizio, non l'item"
        );
    }

    #[test]
    fn ogni_item_si_risolve_da_solo() {
        for level in [Level::new(1).unwrap(), vita(), Level::new(86).unwrap()] {
            for item in items(level) {
                let esercizio = item.facet.exercise_id();
                let risolto = resolve(&item.id, &esercizio).expect("l'item si risolve");
                assert!(
                    !risolto.accepted(item.facet).is_empty(),
                    "{} senza risposte accettate",
                    item.form
                );
            }
        }
    }

    #[test]
    fn il_significato_si_chiede_a_scelta_multipla() {
        let pool: Vec<ItemId> = items(vita()).into_iter().map(|i| i.id).collect();
        let item = id("生");
        let q = MeaningFacet
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &pool,
                    distractors: 3,
                },
                &mut rng(),
            )
            .unwrap();

        assert_eq!(q.asks.as_deref(), Some("meaning"));
        assert_eq!(q.prompt, Prompt::Japanese("生".into()));
        let AnswerFormat::Choice { options } = &q.format else {
            panic!("il significato e' a scelta multipla");
        };
        assert_eq!(options.len(), 4);

        // Una sola opzione e' giusta: se un distrattore fosse accettato, la domanda
        // avrebbe due risposte buone e una sola premiabile.
        let giuste = options
            .iter()
            .filter(|o| {
                MeaningFacet
                    .grade(&item, &Answer::new(o.as_str()))
                    .unwrap()
                    .is_correct()
            })
            .count();
        assert_eq!(giuste, 1, "{options:?}");
    }

    #[test]
    fn del_significato_si_accetta_anche_quello_secondario() {
        let item = id("生");
        for risposta in ["life", "Life", " be born "] {
            assert!(
                MeaningFacet.grade(&item, &Answer::new(risposta)).unwrap().is_correct(),
                "{risposta} doveva andare bene"
            );
        }
        assert!(!MeaningFacet.grade(&item, &Answer::new("water")).unwrap().is_correct());
    }

    #[test]
    fn la_lettura_on_si_accetta_in_tutte_e_due_le_grafie() {
        let item = id("生");
        assert_eq!(
            OnFacet.grade(&item, &Answer::new("セイ")).unwrap(),
            Verdict::correct(),
            "in katakana e' anche scritta bene, quindi niente da dire"
        );

        // In hiragana e' giusta lo stesso, e il rilievo insegna la convenzione senza
        // dire a FSRS che il ricordo e' debole.
        let verdict = OnFacet.grade(&item, &Answer::new("せい")).unwrap();
        assert!(verdict.is_correct());
        assert_eq!(
            verdict,
            Verdict::Correct {
                note: Some(Note {
                    kind: "on_in_hiragana".into(),
                    expected: "セイ".into(),
                }),
            }
        );
    }

    #[test]
    fn la_lettura_kun_vuole_l_hiragana_e_lo_dice_al_contrario() {
        let item = id("生");
        assert_eq!(
            KunFacet.grade(&item, &Answer::new("なま")).unwrap(),
            Verdict::correct()
        );

        let verdict = KunFacet.grade(&item, &Answer::new("ナマ")).unwrap();
        assert!(verdict.is_correct());
        let Verdict::Correct { note: Some(note) } = verdict else {
            panic!("il rilievo ci vuole");
        };
        assert_eq!(note.kind, "kun_in_katakana");
        assert_eq!(note.expected, "なま");
    }

    #[test]
    fn una_lettura_sbagliata_dice_cosa_si_accettava() {
        let item = id("生");
        let verdict = OnFacet.grade(&item, &Answer::new("あい")).unwrap();
        assert_eq!(
            verdict,
            Verdict::Incorrect {
                accepted: vec!["セイ".into(), "ショウ".into()],
            }
        );
    }

    #[test]
    fn l_okurigana_dice_su_quale_porzione_verte() {
        let pool = vec![];
        let item = id("生きる");
        let q = OkuriganaFacet
            .question(
                QuestionRequest {
                    item: &item,
                    pool: &pool,
                    distractors: 3,
                },
                &mut rng(),
            )
            .unwrap();

        assert_eq!(q.prompt, Prompt::Japanese("生きる".into()));
        assert_eq!(q.asks.as_deref(), Some("okurigana"));
        assert_eq!(
            q.focus.as_deref(),
            Some("生"),
            "si legge la parte col kanji, il きる e' li' solo a dire quale lettura vale"
        );
        assert_eq!(q.format, AnswerFormat::Input);
        assert!(
            OkuriganaFacet.grade(&item, &Answer::new("い")).unwrap().is_correct(),
            "si chiede la lettura della parte kanji"
        );
    }

    #[test]
    fn l_okurigana_visibile_non_si_ridigita() {
        // La domanda mostra 大きい, quindi il きい e' gia' sotto gli occhi: chiederlo
        // vorrebbe dire farlo ricopiare. Si risponde おお, e おおきい e' sbagliato.
        let item = id("大きい");
        assert!(OkuriganaFacet.grade(&item, &Answer::new("おお")).unwrap().is_correct());
        assert!(
            !OkuriganaFacet
                .grade(&item, &Answer::new("おおきい"))
                .unwrap()
                .is_correct(),
            "la parola intera comprende quello che la domanda gia' mostra"
        );
    }

    #[test]
    fn una_forma_con_due_letture_le_accetta_entrambe() {
        // 行く si legge sia いく sia ゆく, cioe' la parte kanji e' い oppure ゆ:
        // due risposte buone alla stessa domanda.
        let item = id("行く");
        for lettura in ["い", "ゆ"] {
            assert!(
                OkuriganaFacet.grade(&item, &Answer::new(lettura)).unwrap().is_correct(),
                "{lettura} e' una lettura di 行く"
            );
        }
    }

    #[test]
    fn un_identificatore_estraneo_e_un_errore_diverso_da_uno_inesistente() {
        assert!(matches!(
            OnFacet.grade(&ItemId::new("kana:hiragana:か"), &Answer::new("ka")),
            Err(CoreError::ItemNotSupported { .. })
        ));
        assert!(matches!(
            OnFacet.grade(&id("X"), &Answer::new("セイ")),
            Err(CoreError::UnknownItem { .. })
        ));

        // Un identificatore della vecchia forma, col livello dentro. Non e' piu'
        // leggibile, ed e' voluto: nessun database pubblicato ne contiene, perche' la
        // versione che scriveva quegli identificatori non e' mai uscita.
        assert!(matches!(
            OnFacet.grade(&ItemId::new("kanji:1:生"), &Answer::new("セイ")),
            Err(CoreError::UnknownItem { .. })
        ));
    }

    #[test]
    fn la_faccetta_si_ritrova_dal_tipo_di_esercizio() {
        for facet in [Facet::Meaning, Facet::On, Facet::Kun, Facet::Okurigana] {
            assert_eq!(facet_of(&facet.exercise_id()), Some(facet));
        }
        assert_eq!(facet_of(&ExerciseTypeId::new("kana.input")), None);
    }
}
