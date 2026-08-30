//! Gli esercizi sui kanji.
//!
//! # La domanda non e' «che lettura ha questo kanji»
//!
//! Quella domanda non ha una risposta: 生 ha due letture on e diciotto kun. E il verso
//! opposto e' peggio, perche' コウ e' la lettura on di 41 kanji delle medie: chiedere
//! «quale kanji si legge コウ» non e' una domanda, e' un elenco. Quindi **si va solo da
//! kanji a lettura**, che e' anche il verso che serve a leggere davvero.
//!
//! # L'unita' di studio e' la coppia (forma scritta, famiglia)
//!
//! | famiglia | cosa si vede | cosa si accetta |
//! |---|---|---|
//! | [`Family::On`] | 生, dicendo che si vuole la lettura on | セイ, ショウ |
//! | [`Family::Kun`] | 生, dicendo che si vuole la lettura kun | き, なま, うまれ |
//! | [`Family::Okurigana`] | 生きる, che non ha bisogno di dire altro | いきる |
//!
//! Due meccanismi diversi tolgono l'ambiguita', e insieme la coprono tutta:
//!
//! - **L'okurigana e' gia' il disambiguatore.** 生 ha diciotto letture kun, 生きる ne
//!   ha una. Non e' un espediente: e' l'informazione che l'okurigana porta quando si
//!   legge un testo vero.
//! - **Dire la famiglia risolve il resto.** Dentro una famiglia le letture rimaste sono
//!   alternative genuine, e «quale delle due» non e' una domanda con una risposta: セイ
//!   e ショウ sono entrambe letture on di 生. Si accetta qualunque lettura della
//!   famiglia.
//!
//! # Cosa il dizionario porta e l'esercizio non chiede
//!
//! Le tabelle restano fedeli a KANJIDIC2 (vedi [`super::data`]), quindi e' qui che si
//! scarta cio' che non sta in piedi da solo:
//!
//! - **le letture col trattino**, 58 nel solo primo anno: `-り` di 人 e `なま-` di 生
//!   esistono solo dentro un composto. Le 199 che portano anche il punto, come
//!   `-ちが.える`, sono varianti da composto di forme che ci sono gia' senza;
//! - **le due letture kun scritte in katakana**, `シリング` di 志 e `デシメートル` di
//!   粉, che sono unita' di misura storiche e non letture.
//!
//! Nessun kanji resta senza item: 込 ha sia `-こ.む` sia `こ.む`, e un test lo verifica
//! su tutti e 2.136.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::features::kanji::data::{GRADES, Grade, Kanji, table};
use crate::shared::error::{CoreError, Result};
use crate::shared::exercise::{
    Answer, AnswerFormat, ExerciseType, ExerciseTypeId, ItemId, Prompt, Question, QuestionRequest,
    Verdict,
};
use crate::shared::text::normalize_reading;

/// Prefisso degli identificatori prodotti da questa feature.
const NAMESPACE: &str = "kanji";

/// Quale lettura di un kanji si sta allenando.
///
/// E' l'ambito, come le famiglie per i kana: si sceglie un anno di scuola e una o piu'
/// famiglie, e il giro copre quello.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    /// La lettura di origine cinese, che il dizionario scrive in katakana.
    On,
    /// La lettura giapponese del kanji da solo, senza okurigana.
    Kun,
    /// La lettura di una forma scritta col suo okurigana: 生きる, non 生.
    Okurigana,
}

/// Le tre famiglie, nell'ordine in cui si presentano.
pub const FAMILIES: [Family; 3] = [Family::On, Family::Kun, Family::Okurigana];

impl Family {
    /// La chiave che finisce negli identificatori e attraversa il confine.
    ///
    /// `okurigana` diventa `oku` perche' un identificatore si scrive in mezzo milione
    /// di righe di storico e la brevita' li' vale piu' della simmetria.
    pub fn key(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Kun => "kun",
            Self::Okurigana => "oku",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "on" => Some(Self::On),
            "kun" => Some(Self::Kun),
            "oku" => Some(Self::Okurigana),
            _ => None,
        }
    }
}

fn grade_key(grade: Grade) -> &'static str {
    match grade {
        Grade::First => "first",
        Grade::Second => "second",
        Grade::Third => "third",
        Grade::Fourth => "fourth",
        Grade::Fifth => "fifth",
        Grade::Sixth => "sixth",
        Grade::Secondary => "secondary",
    }
}

fn grade_from_key(key: &str) -> Option<Grade> {
    GRADES.into_iter().find(|&g| grade_key(g) == key)
}

/// Costruisce l'identificatore di un item, nella forma `kanji:first:on:生`.
///
/// # Perche' dentro c'e' anche il grado
///
/// Perche' dice **in quale tabella cercare**, ed e' lo stesso mestiere che fa
/// `hiragana` dentro `kana:hiragana:か`. Senza, risalire da un identificatore al suo
/// kanji vorrebbe dire scorrere tutti e sette i file, e il caricamento pigro per grado
/// non servirebbe piu' a niente: basterebbe una domanda per analizzarli tutti.
///
/// Il costo e' che il grado entra nell'identita' dell'item: se KANJIDIC2 spostasse un
/// kanji di anno, lo storico di quel kanji resterebbe attaccato al vecchio
/// identificatore. I gradi pero' sono fissati per decreto, e un test ne verifica i
/// conteggi.
pub fn item_id(grade: Grade, family: Family, form: &str) -> ItemId {
    ItemId::new(format!(
        "{NAMESPACE}:{}:{}:{form}",
        grade_key(grade),
        family.key()
    ))
}

/// Un item risolto: cosa mostrare e cosa accettare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub family: Family,
    /// La forma scritta da mostrare: il kanji, o il kanji col suo okurigana.
    pub form: String,
    /// Le letture accettate, nella grafia del dizionario.
    ///
    /// La prima e' quella che si mostra come opzione giusta nella scelta multipla.
    pub readings: Vec<String>,
}

/// Gli item di un grado, per le famiglie chieste.
///
/// L'ordine e' quello della tabella, cioe' per frequenza: chi lo usa lo mescola.
pub fn items(grade: Grade, families: &[Family]) -> Vec<Item> {
    let wanted = |f: Family| families.is_empty() || families.contains(&f);
    let mut out = Vec::new();

    for kanji in table(grade).all() {
        if wanted(Family::On) {
            let readings = on_readings(kanji);
            if !readings.is_empty() {
                out.push(Item {
                    family: Family::On,
                    form: kanji.character.clone(),
                    readings,
                });
            }
        }

        if wanted(Family::Kun) {
            let readings = bare_kun(kanji);
            if !readings.is_empty() {
                out.push(Item {
                    family: Family::Kun,
                    form: kanji.character.clone(),
                    readings,
                });
            }
        }

        if wanted(Family::Okurigana) {
            for (form, readings) in okurigana(kanji) {
                out.push(Item {
                    family: Family::Okurigana,
                    form,
                    readings,
                });
            }
        }
    }

    out
}

/// Le letture on, tolte le quattro che esistono solo come suffisso (`-ネン` di 縁).
fn on_readings(kanji: &Kanji) -> Vec<String> {
    kanji
        .on
        .iter()
        .filter(|r| !r.contains('-'))
        .cloned()
        .collect()
}

/// Le letture kun del kanji da solo: senza okurigana, senza affissi, non in katakana.
fn bare_kun(kanji: &Kanji) -> Vec<String> {
    kanji
        .kun
        .iter()
        .filter(|r| usable_kun(r) && !r.contains('.'))
        .cloned()
        .collect()
}

/// Le forme scritte con okurigana, ognuna con le sue letture.
///
/// Una stessa forma puo' avere piu' letture, ed e' il dizionario a dirlo: 生す si legge
/// sia なす sia むす. Non sono due item, e' un item con due risposte buone.
fn okurigana(kanji: &Kanji) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();

    for reading in kanji.kun.iter().filter(|r| usable_kun(r)) {
        let Some((head, tail)) = reading.split_once('.') else {
            continue;
        };
        let form = format!("{}{tail}", kanji.character);
        let full = format!("{head}{tail}");

        match out.iter_mut().find(|(f, _)| *f == form) {
            Some((_, readings)) => {
                if !readings.contains(&full) {
                    readings.push(full);
                }
            }
            None => out.push((form, vec![full])),
        }
    }

    out
}

/// Una lettura kun utilizzabile in un esercizio.
///
/// Fuori gli affissi e le due voci in katakana: vedi la nota in cima al modulo.
fn usable_kun(reading: &str) -> bool {
    !reading.contains('-') && !reading.chars().any(is_katakana)
}

fn is_katakana(c: char) -> bool {
    ('\u{30A1}'..='\u{30FA}').contains(&c)
}

/// Risale dall'identificatore all'item.
///
/// Un identificatore di un'altra materia non e' un dato corrotto: e' semplicemente
/// qualcosa di cui questo esercizio non si occupa, e viene distinto dal caso di un
/// kanji o di una forma che non esistono.
fn resolve(id: &ItemId, exercise: &ExerciseTypeId) -> Result<Item> {
    let not_supported = || CoreError::ItemNotSupported {
        exercise: exercise.to_string(),
        id: id.to_string(),
    };
    let unknown = || CoreError::UnknownItem { id: id.to_string() };

    let rest = id
        .as_str()
        .strip_prefix(NAMESPACE)
        .and_then(|r| r.strip_prefix(':'))
        .ok_or_else(not_supported)?;
    let (grade, rest) = rest.split_once(':').ok_or_else(not_supported)?;
    let (family, form) = rest.split_once(':').ok_or_else(not_supported)?;

    let grade = grade_from_key(grade).ok_or_else(not_supported)?;
    let family = Family::from_key(family).ok_or_else(not_supported)?;

    // Il kanji e' il primo carattere della forma scritta: da solo per le letture on e
    // kun, seguito dall'okurigana per le altre.
    let character = form.chars().next().ok_or_else(unknown)?;
    let entry = table(grade)
        .all()
        .iter()
        .find(|k| k.character.chars().eq(std::iter::once(character)))
        .ok_or_else(unknown)?;

    let readings = match family {
        Family::On => on_readings(entry),
        Family::Kun => bare_kun(entry),
        Family::Okurigana => okurigana(entry)
            .into_iter()
            .find(|(f, _)| f == form)
            .map(|(_, r)| r)
            .unwrap_or_default(),
    };

    if readings.is_empty() {
        return Err(unknown());
    }

    Ok(Item {
        family,
        form: form.to_owned(),
        readings,
    })
}

/// Mostra la forma scritta, si sceglie la lettura.
pub struct KanjiRecognition;

impl KanjiRecognition {
    pub const ID: ExerciseTypeId = ExerciseTypeId::new("kanji.recognition");
}

impl ExerciseType for KanjiRecognition {
    fn id(&self) -> ExerciseTypeId {
        Self::ID
    }

    fn question(&self, request: QuestionRequest<'_>, rng: &mut dyn Rng) -> Result<Question> {
        let item = resolve(request.item, &Self::ID)?;
        let corretta = item.readings[0].clone();

        let mut options = distractors(&request, &item, rng);
        options.push(corretta);
        options.shuffle(rng);

        Ok(Question {
            exercise_type: Self::ID,
            item: request.item.clone(),
            prompt: Prompt::Japanese(item.form.clone()),
            // L'okurigana dice gia' da solo cosa si vuole; il kanji nudo no.
            asks: match item.family {
                Family::Okurigana => None,
                family => Some(family.key().to_owned()),
            },
            format: AnswerFormat::Choice { options },
        })
    }

    fn grade(&self, id: &ItemId, answer: &Answer) -> Result<Verdict> {
        let item = resolve(id, &Self::ID)?;
        let dato = normalize_reading(answer.as_str());

        // Si giudica sulla lettura e non sulla grafia: il dizionario scrive le letture
        // on in katakana, ma pretendere che si digitino in katakana proverebbe la
        // tastiera e non la conoscenza. `normalize_reading` ripiega tutto sull'hiragana.
        Ok(
            if item.readings.iter().any(|r| normalize_reading(r) == dato) {
                Verdict::Correct
            } else {
                Verdict::Incorrect {
                    accepted: item.readings,
                }
            },
        )
    }
}

/// Pesca i distrattori dal pool, **dalla stessa famiglia** dell'item chiesto.
///
/// Non e' una preferenza come per i kana, e' un requisito: il dizionario scrive le
/// letture on in katakana e le kun in hiragana, quindi una kun in mezzo a tre on si
/// riconosce dalla forma delle lettere senza sapere niente del kanji, e la domanda si
/// risponde da sola.
///
/// I candidati vengono confrontati **sul valore normalizzato**, non sulla provenienza:
/// ショウ e' la lettura on di cinque kanji del primo anno, quindi un distrattore pescato
/// da un altro kanji puo' essere una risposta giusta per quello mostrato. Lo stesso
/// confronto scarta i doppioni fra distrattori.
fn distractors(request: &QuestionRequest<'_>, item: &Item, rng: &mut dyn Rng) -> Vec<String> {
    let escluse: Vec<String> = item.readings.iter().map(|r| normalize_reading(r)).collect();

    let mut candidati: Vec<&ItemId> = request
        .pool
        .iter()
        .filter(|id| id.as_str().contains(&format!(":{}:", item.family.key())))
        .collect();
    candidati.shuffle(rng);

    let mut presi: Vec<String> = Vec::new();
    let mut viste = escluse;

    for id in candidati {
        if presi.len() == request.distractors {
            break;
        }
        let Ok(altro) = resolve(id, &KanjiRecognition::ID) else {
            continue;
        };
        let lettura = altro.readings[0].clone();
        let chiave = normalize_reading(&lettura);
        if viste.contains(&chiave) {
            continue;
        }
        viste.push(chiave);
        presi.push(lettura);
    }

    presi
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    fn ids(grade: Grade, families: &[Family]) -> Vec<ItemId> {
        items(grade, families)
            .into_iter()
            .map(|i| item_id(grade, i.family, &i.form))
            .collect()
    }

    #[test]
    fn le_famiglie_del_primo_anno_hanno_le_dimensioni_attese() {
        assert_eq!(items(Grade::First, &[Family::On]).len(), 80);
        assert_eq!(items(Grade::First, &[Family::Kun]).len(), 69);
        assert_eq!(items(Grade::First, &[Family::Okurigana]).len(), 77);
        assert_eq!(items(Grade::First, &[]).len(), 226, "vuoto significa tutte");
    }

    #[test]
    fn nessun_kanji_resta_senza_un_item() {
        // Scartare affissi e katakana non deve far sparire un kanji dall'esercizio:
        // 込 sopravvive perche' ha sia `-こ.む` sia `こ.む`.
        for grade in GRADES {
            let coperti: std::collections::HashSet<char> = items(grade, &[])
                .iter()
                .filter_map(|i| i.form.chars().next())
                .collect();

            for kanji in table(grade).all() {
                let c = kanji.character.chars().next().unwrap();
                assert!(coperti.contains(&c), "{} non ha nessun item", kanji.character);
            }
        }
    }

    #[test]
    fn ogni_item_ha_almeno_una_lettura_e_un_identificatore_valido() {
        for grade in GRADES {
            for item in items(grade, &[]) {
                assert!(!item.readings.is_empty(), "{} senza letture", item.form);
                let id = item_id(grade, item.family, &item.form);
                assert_eq!(
                    resolve(&id, &KanjiRecognition::ID).unwrap(),
                    item,
                    "l'identificatore non torna all'item da cui e' nato"
                );
            }
        }
    }

    #[test]
    fn le_letture_scartate_non_diventano_item() {
        // シリング (lo scellino) e デシメートル sono kun nel dizionario, ma sono unita'
        // di misura storiche e non letture. La lettura on シ di 志, che e' katakana per
        // sua natura, deve invece restare.
        let quinto = items(Grade::Fifth, &[]);
        let letture: Vec<&String> = quinto.iter().flat_map(|i| &i.readings).collect();
        assert!(!letture.iter().any(|r| r.as_str() == "シリング"));
        assert!(!letture.iter().any(|r| r.as_str() == "デシメートル"));
        assert!(
            letture.iter().any(|r| r.as_str() == "シ"),
            "la lettura on di 志 non c'entra niente con lo scellino"
        );

        // Gli affissi col trattino non compaiono da nessuna parte.
        for grade in GRADES {
            for item in items(grade, &[]) {
                for reading in &item.readings {
                    assert!(!reading.contains('-'), "{reading} e' un affisso");
                    assert!(!reading.contains('.'), "{reading} ha ancora il punto");
                }
            }
        }
    }

    #[test]
    fn l_okurigana_toglie_l_ambiguita() {
        let primo = items(Grade::First, &[Family::Okurigana]);

        let vivere = primo.iter().find(|i| i.form == "生きる").unwrap();
        assert_eq!(vivere.readings, ["いきる"], "una forma, una lettura");

        // 生 da solo ne avrebbe diciotto: e' la ragione per cui l'item e' la forma
        // scritta e non il kanji.
        let nudo = items(Grade::First, &[Family::Kun])
            .into_iter()
            .find(|i| i.form == "生")
            .unwrap();
        assert!(nudo.readings.len() > 1);
    }

    #[test]
    fn una_forma_con_due_letture_le_accetta_entrambe() {
        // 生す si legge sia なす sia むす: non sono due item, e' un item con due
        // risposte buone.
        let nasu = items(Grade::First, &[Family::Okurigana])
            .into_iter()
            .find(|i| i.form == "生す")
            .expect("生す esiste");

        assert_eq!(nasu.readings, ["なす", "むす"]);
    }

    #[test]
    fn la_domanda_dice_cosa_chiede_solo_quando_serve() {
        let mut rng = rng();
        let pool = ids(Grade::First, &[]);

        let on = item_id(Grade::First, Family::On, "生");
        let q = KanjiRecognition
            .question(
                QuestionRequest {
                    item: &on,
                    pool: &pool,
                    distractors: 3,
                },
                &mut rng,
            )
            .unwrap();
        assert_eq!(q.asks.as_deref(), Some("on"));
        assert_eq!(q.prompt, Prompt::Japanese("生".into()));

        let oku = item_id(Grade::First, Family::Okurigana, "生きる");
        let q = KanjiRecognition
            .question(
                QuestionRequest {
                    item: &oku,
                    pool: &pool,
                    distractors: 3,
                },
                &mut rng,
            )
            .unwrap();
        assert_eq!(q.asks, None, "l'okurigana parla da solo");
    }

    #[test]
    fn i_distrattori_sono_della_stessa_famiglia_e_mai_giusti() {
        let mut rng = rng();
        let pool = ids(Grade::First, &[]);
        let tutte: Vec<Item> = items(Grade::First, &[]);

        for item in &tutte {
            let id = item_id(Grade::First, item.family, &item.form);
            let q = KanjiRecognition
                .question(
                    QuestionRequest {
                        item: &id,
                        pool: &pool,
                        distractors: 3,
                    },
                    &mut rng,
                )
                .unwrap();

            let AnswerFormat::Choice { options } = &q.format else {
                panic!("il riconoscimento e' a scelta multipla");
            };
            assert_eq!(options.len(), 4, "{}", item.form);

            // Nessun doppione, e una sola opzione giusta: se un distrattore fosse
            // accettato da `grade`, la domanda avrebbe due risposte buone e una sola
            // premiabile.
            let giuste = options
                .iter()
                .filter(|o| {
                    KanjiRecognition
                        .grade(&id, &Answer::new(o.as_str()))
                        .unwrap()
                        .is_correct()
                })
                .count();
            assert_eq!(giuste, 1, "{}: {options:?}", item.form);

            let mut viste = std::collections::HashSet::new();
            for o in options {
                assert!(viste.insert(normalize_reading(o)), "opzione doppia in {options:?}");
            }
        }
    }

    #[test]
    fn le_letture_on_si_possono_scrivere_in_hiragana() {
        let id = item_id(Grade::First, Family::On, "生");

        // Il dizionario scrive セイ, ma chi digita せい ha risposto alla domanda: qui
        // conta la lettura, non la grafia.
        for risposta in ["セイ", "せい", "ショウ", "しょう"] {
            assert!(
                KanjiRecognition
                    .grade(&id, &Answer::new(risposta))
                    .unwrap()
                    .is_correct(),
                "{risposta} doveva andare bene"
            );
        }

        let sbagliata = KanjiRecognition.grade(&id, &Answer::new("なま")).unwrap();
        assert_eq!(
            sbagliata,
            Verdict::Incorrect {
                accepted: vec!["セイ".into(), "ショウ".into()],
            },
            "なま e' una lettura di 生, ma non e' una lettura on"
        );
    }

    #[test]
    fn un_identificatore_estraneo_e_un_errore_diverso_da_uno_inesistente() {
        let altra_materia = ItemId::new("kana:hiragana:か");
        assert!(matches!(
            KanjiRecognition.grade(&altra_materia, &Answer::new("ka")),
            Err(CoreError::ItemNotSupported { .. })
        ));

        let inesistente = item_id(Grade::First, Family::On, "X");
        assert!(matches!(
            KanjiRecognition.grade(&inesistente, &Answer::new("セイ")),
            Err(CoreError::UnknownItem { .. })
        ));

        // Un kanji vero, ma nel grado sbagliato: la tabella del primo anno non lo ha.
        let fuori_grado = item_id(Grade::First, Family::On, "違");
        assert!(matches!(
            KanjiRecognition.grade(&fuori_grado, &Answer::new("イ")),
            Err(CoreError::UnknownItem { .. })
        ));
    }
}
