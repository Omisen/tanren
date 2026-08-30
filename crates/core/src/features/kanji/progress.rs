//! A che punto e' l'apprendimento, e quando si puo' andare avanti.
//!
//! # Due misure diverse che non vanno mescolate
//!
//! **Quanto sei consolidato** lo dice FSRS, e lo dice questo modulo: quante faccette
//! hanno superato la soglia, quanto reggono adesso quelle che stai portando avanti,
//! quando un livello e' finito. **Com'e' andata la pratica di oggi** e' un'altra cosa,
//! vive dentro la sessione di Drill e non arriva mai qui: il Drill non tocca lo
//! scheduling, quindi non puo' spostare questa misura.
//!
//! # Perche' sta nella feature e non in `shared`
//!
//! Perche' parla di livelli e di faccette, che sono cose dei kanji. La regola del
//! progetto e' di non generalizzare prima di avere due casi: quando una seconda materia
//! avra' bisogno di maturita' e sblocco si vedra' cosa e' davvero comune.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::features::kanji::facets::{Facet, Item, items};
use crate::features::kanji::levels::{Level, table};
use crate::shared::error::Result;
use crate::shared::srs::retrievability;
use crate::shared::storage::{Card, CardFilter, Database};

/// I numeri che governano il ritmo.
///
/// Sono **tutti tarabili**: sono scelte prudenti su cui non abbiamo ancora dati veri,
/// e il punto di tenerli qui insieme e' poterli cambiare in un posto solo quando i
/// dati arriveranno.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pacing {
    /// Quanti item mettere in un giro di Drill.
    ///
    /// Il Drill e' pratica a richiesta: senza un tetto un giro sarebbe lungo quanto
    /// tutto quello che si e' imparato, cioe' impraticabile.
    pub drill_size: usize,
    /// Quanti kanji nuovi al giorno, al massimo.
    ///
    /// Cinque e' volutamente prudente: il carico associativo di un kanji e' alto,
    /// perche' non e' un ricordo ma tre o quattro.
    pub daily_new: usize,
    /// Quanto devono reggere in media le faccette che si stanno portando avanti
    /// perche' si possa introdurre altro. E' il freno principale.
    pub min_retrievability: f32,
    /// Quanto deve passare fra due introduzioni.
    ///
    /// Serve a non ingozzarsi tutto in un'ora: il cap giornaliero da solo non lo
    /// impedirebbe.
    pub floor: TimeDelta,
    /// Da quanti giorni di stabilita' una faccetta si considera matura.
    pub mature_days: f32,
    /// Quanta parte di un livello deve essere matura perche' sblocchi il successivo.
    ///
    /// Non il 100%: pochi item ostinati bloccherebbero il percorso per sempre.
    pub unlock_ratio: f32,
}

impl Default for Pacing {
    fn default() -> Self {
        Self {
            drill_size: 20,
            daily_new: 5,
            min_retrievability: 0.75,
            floor: TimeDelta::hours(4),
            mature_days: 21.0,
            unlock_ratio: 0.9,
        }
    }
}

/// In che stato e' un kanji.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Mai introdotto.
    New,
    /// Introdotto, con almeno una faccetta ancora acerba.
    Learning,
    /// Tutte le faccette attive hanno superato la soglia.
    Mature,
}

/// A che punto e' un livello.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelProgress {
    pub level: Level,
    pub total: usize,
    pub new: usize,
    pub learning: usize,
    pub mature: usize,
    /// La quota di kanji maturi, da 0 a 1.
    pub ratio: f32,
    /// Se il livello e' abbastanza consolidato da aprire il successivo.
    pub complete: bool,
}

/// Perche' non si puo' imparare altro adesso.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Blocked {
    /// Quello che c'e' gia' non regge abbastanza. E' il freno che conta.
    Consolidate { current: f32, needed: f32 },
    /// Si e' introdotto troppo di recente.
    TooSoon { until: DateTime<Utc> },
    /// La quota di oggi e' finita.
    DailyCap { done: usize, cap: usize },
    /// Non c'e' piu' niente di nuovo in questo livello.
    NothingNew,
}

/// Se si puo' introdurre roba nuova, e altrimenti perche' no.
///
/// Il motivo non e' un dettaglio da inghiottire: dire «consolida quello che hai» e
/// dire «torna fra quattro ore» sono due consigli diversi, e chi studia ha diritto di
/// sapere quale dei due vale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Gate {
    Open { room: usize },
    Closed(Blocked),
}

/// Le carte di un livello, prese in blocco.
async fn cards_of(db: &Database, level: Level) -> Result<(Vec<Item>, Vec<Card>)> {
    let elenco = items(level);
    let ids: Vec<String> = {
        let mut v: Vec<String> = elenco.iter().map(|i| i.id.as_str().to_owned()).collect();
        v.sort();
        v.dedup();
        v
    };

    let cards = db
        .cards(CardFilter {
            items: Some(&ids),
            exercise_type: None,
        })
        .await?;

    Ok((elenco, cards))
}

fn card_for<'a>(cards: &'a [Card], item: &Item) -> Option<&'a Card> {
    let exercise = item.facet.exercise_id();
    cards
        .iter()
        .find(|c| c.item_id == item.id.as_str() && c.exercise_type == exercise.as_str())
}

/// Se una faccetta ha superato la soglia di stabilita'.
fn is_mature(card: Option<&Card>, pacing: &Pacing) -> bool {
    card.and_then(|c| c.stability)
        .is_some_and(|s| s >= pacing.mature_days)
}

/// In che stato e' ogni kanji del livello, **nell'ordine della tabella**.
///
/// Un kanji e' maturo quando **tutte** le sue faccette attive lo sono: sapere il
/// significato di 生 e non sapere come si legge non e' saperlo.
pub async fn standings(
    db: &Database,
    level: Level,
    pacing: &Pacing,
) -> Result<Vec<(String, Standing)>> {
    let (elenco, cards) = cards_of(db, level).await?;

    let mut per_kanji: std::collections::HashMap<String, Vec<&Item>> = Default::default();
    for item in &elenco {
        per_kanji.entry(kanji_of(item)).or_default().push(item);
    }

    // L'ordine e' quello della tabella, cioe' per frequenza, e non quello di una mappa:
    // la griglia si guarda, e un ordine che cambia da un giro all'altro sarebbe
    // illeggibile.
    Ok(table(level)
        .all()
        .iter()
        .map(|k| {
            let facce = per_kanji.get(&k.character);
            let stato = match facce {
                None => Standing::New,
                Some(facce) => {
                    let carte: Vec<Option<&Card>> =
                        facce.iter().map(|i| card_for(&cards, i)).collect();
                    if carte.iter().all(Option::is_none) {
                        Standing::New
                    } else if carte.iter().all(|c| is_mature(*c, pacing)) {
                        Standing::Mature
                    } else {
                        Standing::Learning
                    }
                }
            };
            (k.character.clone(), stato)
        })
        .collect())
}

/// A che punto e' un livello.
pub async fn level_progress(
    db: &Database,
    level: Level,
    pacing: &Pacing,
) -> Result<LevelProgress> {
    let stati = standings(db, level, pacing).await?;

    let conta = |cercato: Standing| stati.iter().filter(|(_, s)| *s == cercato).count();
    let new = conta(Standing::New);
    let learning = conta(Standing::Learning);
    let mature = conta(Standing::Mature);
    let total = stati.len();
    let ratio = if total > 0 {
        mature as f32 / total as f32
    } else {
        0.0
    };

    Ok(LevelProgress {
        level,
        total,
        new,
        learning,
        mature,
        ratio,
        complete: total > 0 && ratio >= pacing.unlock_ratio,
    })
}

/// Il kanji a cui appartiene un item: la forma intera per le faccette del kanji nudo,
/// il primo carattere per quelle con l'okurigana.
fn kanji_of(item: &Item) -> String {
    match item.facet {
        Facet::Okurigana => item.form.chars().take(1).collect(),
        _ => item.form.clone(),
    }
}

/// Fin dove si e' arrivati: il primo livello non ancora completato.
///
/// I livelli precedenti restano aperti, quelli successivi si possono guardare ma non
/// esercitare.
pub async fn current_level(db: &Database, pacing: &Pacing) -> Result<Level> {
    for level in Level::all() {
        if !level_progress(db, level, pacing).await?.complete {
            return Ok(level);
        }
    }
    Ok(Level::new(crate::features::kanji::levels::LEVELS).expect("l'ultimo livello esiste"))
}

/// Quanto reggono adesso, in media, le faccette che si stanno portando avanti.
///
/// Contano solo quelle **introdotte e non ancora mature**: quelle mature reggono per
/// definizione e le tirerebbero su, nascondendo proprio il carico che si vuole
/// misurare. `None` quando non ce n'e' nessuna, che non e' zero: non c'e' niente da
/// misurare, non c'e' un carico basso.
pub async fn load(
    db: &Database,
    level: Level,
    pacing: &Pacing,
    now: DateTime<Utc>,
) -> Result<Option<f32>> {
    let (elenco, cards) = cards_of(db, level).await?;

    let attive: Vec<f32> = elenco
        .iter()
        .filter_map(|item| card_for(&cards, item))
        .filter(|c| !is_mature(Some(c), pacing))
        .filter_map(|c| Some(retrievability(c.memory()?, c.last_reviewed_at?, now)))
        .collect();

    Ok((!attive.is_empty()).then(|| attive.iter().sum::<f32>() / attive.len() as f32))
}

/// Se si puo' introdurre un kanji nuovo, e quanti.
///
/// Le tre condizioni si guardano in quest'ordine, e l'ordine e' il messaggio: prima si
/// chiede se stai reggendo, poi se hai appena studiato, poi se hai gia' fatto la tua
/// parte per oggi.
pub async fn learning_gate(
    db: &Database,
    level: Level,
    pacing: &Pacing,
    now: DateTime<Utc>,
) -> Result<Gate> {
    let progress = level_progress(db, level, pacing).await?;
    if progress.new == 0 {
        return Ok(Gate::Closed(Blocked::NothingNew));
    }

    if let Some(current) = load(db, level, pacing, now).await?
        && current < pacing.min_retrievability
    {
        return Ok(Gate::Closed(Blocked::Consolidate {
            current,
            needed: pacing.min_retrievability,
        }));
    }

    // Una carta nasce quando il kanji viene introdotto, e ogni kanji ne ha una sola di
    // significato: contare quelle e' contare i kanji, non le faccette.
    let meaning = Facet::Meaning.exercise_id();
    if let Some(last) = db.last_card_created(meaning.as_str()).await?
        && now < last + pacing.floor
    {
        return Ok(Gate::Closed(Blocked::TooSoon {
            until: last + pacing.floor,
        }));
    }

    let oggi = db
        .cards_created_since(meaning.as_str(), start_of_day(now))
        .await?
        .max(0) as usize;
    if oggi >= pacing.daily_new {
        return Ok(Gate::Closed(Blocked::DailyCap {
            done: oggi,
            cap: pacing.daily_new,
        }));
    }

    Ok(Gate::Open {
        room: (pacing.daily_new - oggi).min(progress.new),
    })
}

/// L'inizio della giornata in cui cade un istante.
///
/// In UTC e non nel fuso locale: il core non sa dove si trovi chi studia, e usare un
/// fuso indovinato sposterebbe il confine della giornata di ore.
fn start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("mezzanotte esiste")
        .and_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::kanji::facets::item_id;
    use crate::features::kanji::levels::table;
    use crate::shared::exercise::ItemId;
    use crate::shared::srs::{Grade, MemoryState, Scheduled};
    use crate::shared::storage::{NewAnswer, Scheduling};

    fn primo() -> Level {
        Level::new(1).unwrap()
    }

    async fn db() -> Database {
        Database::in_memory().await.unwrap()
    }

    /// Scrive una carta con la stabilita' e l'eta' che servono alla prova.
    ///
    /// Passa dall'archivio invece di inventare righe: cosi' la prova esercita la
    /// stessa strada che percorre una risposta vera.
    async fn carta(db: &Database, item: &ItemId, facet: Facet, stability: f32, quando: DateTime<Utc>) {
        let esercizio = facet.exercise_id();
        db.record_answer(NewAnswer {
            item_id: item.as_str(),
            exercise_type: esercizio.as_str(),
            correct: true,
            answer: "x",
            answered_at: quando,
            response_time_ms: None,
            scheduling: Some(Scheduling {
                grade: Grade::Good,
                next: Scheduled {
                    memory: MemoryState {
                        stability,
                        difficulty: 5.0,
                    },
                    due_at: quando + TimeDelta::days(stability as i64),
                    interval_days: stability,
                },
            }),
        })
        .await
        .unwrap();
    }

    /// Tutte le faccette di un kanji, portate alla stabilita' chiesta.
    async fn impara(db: &Database, level: Level, kanji: &str, stability: f32, quando: DateTime<Utc>) {
        for item in items(level).into_iter().filter(|i| kanji_of(i) == kanji) {
            carta(db, &item.id, item.facet, stability, quando).await;
        }
    }

    #[tokio::test]
    async fn un_livello_intatto_e_tutto_da_fare() {
        let db = db().await;
        let p = level_progress(&db, primo(), &Pacing::default()).await.unwrap();

        assert_eq!(p.total, table(primo()).all().len());
        assert_eq!(p.new, p.total);
        assert_eq!(p.learning, 0);
        assert_eq!(p.mature, 0);
        assert_eq!(p.ratio, 0.0);
        assert!(!p.complete);
    }

    #[tokio::test]
    async fn un_kanji_e_maturo_solo_se_lo_sono_tutte_le_sue_faccette() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let uno = &table(primo()).all()[0].character.clone();

        // Una faccetta sola, e per giunta gia' matura: il kanji resta in mezzo al guado.
        let solo_significato = item_id(primo(), uno);
        carta(&db, &solo_significato, Facet::Meaning, 40.0, now).await;

        let p = level_progress(&db, primo(), &pacing).await.unwrap();
        assert_eq!(p.learning, 1, "sapere il significato non e' sapere il kanji");
        assert_eq!(p.mature, 0);

        // Con tutte le faccette sopra soglia diventa maturo.
        impara(&db, primo(), uno, 40.0, now).await;
        let p = level_progress(&db, primo(), &pacing).await.unwrap();
        assert_eq!(p.mature, 1);
        assert_eq!(p.learning, 0);
    }

    #[tokio::test]
    async fn la_soglia_di_maturita_e_quella_dichiarata() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let uno = &table(primo()).all()[0].character.clone();

        impara(&db, primo(), uno, pacing.mature_days - 0.1, now).await;
        assert_eq!(level_progress(&db, primo(), &pacing).await.unwrap().mature, 0);

        impara(&db, primo(), uno, pacing.mature_days, now).await;
        assert_eq!(level_progress(&db, primo(), &pacing).await.unwrap().mature, 1);
    }

    #[tokio::test]
    async fn un_livello_si_completa_al_novanta_per_cento_e_non_al_cento() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let kanji: Vec<String> = table(primo())
            .all()
            .iter()
            .map(|k| k.character.clone())
            .collect();

        // Novanta su cento: qualche item ostinato non deve poter bloccare il percorso
        // per sempre.
        let quanti = (kanji.len() as f32 * 0.9).ceil() as usize;
        for k in &kanji[..quanti] {
            impara(&db, primo(), k, 40.0, now).await;
        }

        let p = level_progress(&db, primo(), &pacing).await.unwrap();
        assert!(p.complete, "quota {}", p.ratio);
        assert!(p.mature < p.total, "e non serve averli tutti");
    }

    #[tokio::test]
    async fn la_griglia_tiene_l_ordine_della_tabella() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let tabella = table(primo()).all();

        impara(&db, primo(), &tabella[1].character.clone(), 40.0, now).await;

        let stati = standings(&db, primo(), &pacing).await.unwrap();
        assert_eq!(stati.len(), tabella.len());
        // Una griglia che si riordina a ogni risposta non si potrebbe guardare.
        for (i, (kanji, _)) in stati.iter().enumerate() {
            assert_eq!(kanji, &tabella[i].character);
        }
        assert_eq!(stati[0].1, Standing::New);
        assert_eq!(stati[1].1, Standing::Mature);
    }

    #[tokio::test]
    async fn si_e_arrivati_al_primo_livello_non_finito() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        assert_eq!(current_level(&db, &pacing).await.unwrap(), primo());

        for k in table(primo()).all() {
            impara(&db, primo(), &k.character, 40.0, now).await;
        }
        assert_eq!(
            current_level(&db, &pacing).await.unwrap(),
            Level::new(2).unwrap()
        );
    }

    #[tokio::test]
    async fn a_mani_vuote_si_puo_cominciare() {
        let db = db().await;
        let pacing = Pacing::default();
        let gate = learning_gate(&db, primo(), &pacing, Utc::now()).await.unwrap();
        assert_eq!(gate, Gate::Open { room: pacing.daily_new });
    }

    #[tokio::test]
    async fn quello_che_non_regge_chiude_la_porta_prima_di_tutto() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let uno = &table(primo()).all()[0].character.clone();

        // Introdotto dieci giorni fa e mai piu' rivisto, con una stabilita' bassa:
        // adesso non regge quasi niente.
        impara(&db, primo(), uno, 1.0, now - TimeDelta::days(10)).await;

        let carico = load(&db, primo(), &pacing, now).await.unwrap().unwrap();
        assert!(carico < pacing.min_retrievability, "carico {carico}");

        let gate = learning_gate(&db, primo(), &pacing, now).await.unwrap();
        let Gate::Closed(Blocked::Consolidate { current, needed }) = gate else {
            panic!("doveva chiedere di consolidare, non {gate:?}");
        };
        assert_eq!(needed, pacing.min_retrievability);
        assert!(current < needed);
    }

    #[tokio::test]
    async fn quello_che_e_maturo_non_conta_come_carico() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let uno = &table(primo()).all()[0].character.clone();

        // Maturo e vecchio: reggerebbe poco, ma non e' il carico che si sta portando
        // avanti, e contarlo direbbe di consolidare cio' che e' gia' consolidato.
        impara(&db, primo(), uno, 40.0, now - TimeDelta::days(200)).await;
        assert_eq!(load(&db, primo(), &pacing, now).await.unwrap(), None);
    }

    #[tokio::test]
    async fn dopo_aver_introdotto_si_aspetta() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();
        let uno = &table(primo()).all()[0].character.clone();

        // Maturo, cosi' non e' il carico a chiudere la porta ma l'orologio.
        impara(&db, primo(), uno, 40.0, now).await;

        let gate = learning_gate(&db, primo(), &pacing, now + TimeDelta::hours(1))
            .await
            .unwrap();
        assert!(
            matches!(gate, Gate::Closed(Blocked::TooSoon { .. })),
            "{gate:?}"
        );

        let gate = learning_gate(&db, primo(), &pacing, now + pacing.floor)
            .await
            .unwrap();
        assert!(matches!(gate, Gate::Open { .. }), "passato il floor si riapre");
    }

    #[tokio::test]
    async fn la_quota_di_oggi_si_esaurisce() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();

        for k in table(primo()).all().iter().take(pacing.daily_new) {
            impara(&db, primo(), &k.character, 40.0, now).await;
        }

        let gate = learning_gate(&db, primo(), &pacing, now + pacing.floor)
            .await
            .unwrap();
        assert_eq!(
            gate,
            Gate::Closed(Blocked::DailyCap {
                done: pacing.daily_new,
                cap: pacing.daily_new,
            })
        );
    }

    #[tokio::test]
    async fn finito_il_livello_non_c_e_piu_niente_da_introdurre() {
        let db = db().await;
        let pacing = Pacing::default();
        let now = Utc::now();

        for k in table(primo()).all() {
            impara(&db, primo(), &k.character, 40.0, now).await;
        }

        let gate = learning_gate(&db, primo(), &pacing, now + TimeDelta::days(1))
            .await
            .unwrap();
        assert_eq!(gate, Gate::Closed(Blocked::NothingNew));
    }
}
