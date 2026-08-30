//! I kanji joyo, per livello di apprendimento.
//!
//! # Da dove viene il dato
//!
//! Da **kanjium** (`data/kanjidb.sqlite`), che rispetto a KANJIDIC2 porta i composti,
//! la segmentazione delle loro letture e un ordine di apprendimento. Il file resta
//! input di build e non si imbarca: le tabelle qui dentro le produce
//! `crates/core/data/kanji/generate.py`, che va rilanciato per rigenerarle.
//!
//! kanjium e' **CC BY-SA 4.0** e contiene EDICT, KANJIDIC e KRADFILE dell'EDRDG,
//! anch'essi CC BY-SA 4.0: le due attribuzioni si sommano e queste tabelle non sono
//! MIT come il resto del codice. Vedi `crates/core/data/kanji/ATTRIBUTION.md`.
//!
//! # Perche' i livelli e non gli anni di scuola
//!
//! Perche' l'anno di scuola giapponese non e' un ordine di difficolta' per chi non e'
//! madrelingua, e un anno da 328 kanji non si finisce mai.
//!
//! L'ordine e' **calcolato**, non importato: un ordinamento topologico sui componenti
//! che kanjium dichiara in `kanji_parts`, cosi' che un kanji non venga mai prima dei
//! pezzi di cui e' fatto, con i pareggi rotti per numero di tratti e poi per
//! frequenza. Ne escono **86 livelli** da venticinque, tutti della stessa forma:
//! niente discontinuita' a meta' percorso.
//!
//! Che sia calcolato e non letto da una lista e' una differenza che conta anche
//! fuori dal codice: le liste di progressione dei servizi commerciali non sono dati
//! liberi, mentre i componenti e i tratti arrivano dalle stesse fonti CC BY-SA di
//! tutto il resto.
//!
//! # Perche' i file stanno nel binario
//!
//! Tutti insieme fanno **1,1 MB** dentro un APK che ne pesa 17, e il caricamento resta
//! **pigro e per livello**: studiando il livello 3 si analizzano 17 kB, non un mega.
//! Il 40% del peso sono gli esempi coi loro significati, ed e' la leva da tirare se un
//! domani il peso desse fastidio.
//!
//! **Questa e' pero' la fine della strada per i dati imbarcati**: il vocabolario da
//! JMdict e' due ordini di grandezza piu' grande e non seguira' questa via.
//!
//! # Il dato e' fedele, la selezione la fa l'esercizio
//!
//! Le letture arrivano come le da' kanjium, comprese quelle rare, e le forme con
//! l'okurigana ci sono tutte con un segno che dice quali sono parole che si
//! incontrano davvero ([`Okurigana::common`]). Qui non si taglia niente: il contenuto
//! dice cosa **e'** un kanji, l'esercizio sceglie cosa chiederne.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Quanti livelli conta il percorso.
pub const LEVELS: u8 = 86;

/// Un livello del percorso, da 1 a [`LEVELS`].
///
/// I livelli sono tutti della stessa forma, venticinque kanji l'uno, e l'ordine viene
/// da un topologico sui componenti: vedi la testa del modulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
// In uscita e' un numero e basta; in entrata passa da `TryFrom`, che rifiuta quelli
// fuori scala. Senza, un livello 999 arrivato dal confine sfonderebbe l'indice delle
// tabelle, e la validazione di `new` varrebbe solo per chi la chiama a mano.
#[serde(into = "u8", try_from = "u8")]
pub struct Level(u8);

impl From<Level> for u8 {
    fn from(l: Level) -> Self {
        l.0
    }
}

impl TryFrom<u8> for Level {
    type Error = String;

    fn try_from(n: u8) -> std::result::Result<Self, Self::Error> {
        Self::new(n).ok_or_else(|| format!("livello {n} fuori da 1..={LEVELS}"))
    }
}

impl Level {
    /// `None` fuori da 1..=[`LEVELS`].
    pub fn new(n: u8) -> Option<Self> {
        (1..=LEVELS).contains(&n).then_some(Self(n))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    /// Tutti i livelli, dal primo all'ultimo.
    pub fn all() -> impl Iterator<Item = Self> {
        (1..=LEVELS).map(Self)
    }

    /// Il livello successivo, se non e' l'ultimo.
    pub fn next(self) -> Option<Self> {
        Self::new(self.0 + 1)
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Una forma scritta col suo okurigana: 生きる, che si legge いきる.
///
/// **Una forma puo' avere piu' letture ed e' una cosa sola**: 行く si legge sia いく
/// sia ゆく. Sono due risposte buone alla stessa domanda, non due domande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Okurigana {
    /// Il kanji col suo okurigana.
    pub form: String,
    pub readings: Vec<String>,
    /// Se quella forma e' una parola che si incontra davvero.
    ///
    /// Lo dicono due segnali indipendenti: il corpus di Wikipedia la contiene, oppure
    /// kanjium la classifica come comune. Serve a chi costruisce l'esercizio per
    /// decidere cosa chiedere; qui non si taglia niente.
    pub common: bool,
}

/// Un composto in cui il kanji compare, da mostrare come esempio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Example {
    pub word: String,
    pub reading: String,
    pub meaning: String,
}

/// Un kanji, con tutto quello che serve a impararlo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kanji {
    pub character: String,
    pub strokes: u8,
    /// Il livello in cui si studia. Sta dentro la voce e non solo nell'intestazione
    /// del file perche' serve a costruire l'identificatore di un item, che deve dire
    /// in quale tabella cercare.
    pub level: Level,
    /// Il rango di frequenza di kanjium. `None` per sei joyo che non ce l'hanno.
    pub frequency: Option<u16>,
    /// Quanto quel kanji ricorre **da solo** invece che dentro un composto, da 0 a 1.
    ///
    /// Misurato su un corpus di ventimila parole con conteggi veri: 私 sta a 0,52 e
    /// 生 a 0,005. `None` per i 205 joyo che il corpus non contiene.
    ///
    /// Non e' un dato di kanjium, che ha solo classi testuali di frequenza: e'
    /// derivato, e la derivazione sta nel generatore.
    pub alone_ratio: Option<f32>,
    /// I significati, **il primo e' il primario**.
    ///
    /// Che sia il primo e' un'assunzione, non un dato: kanjium non marca il
    /// significato principale, e l'ordine e' l'unico segnale che offre.
    pub meanings: Vec<String>,
    /// Le letture on, in katakana come le scrive il dizionario.
    pub on: Vec<String>,
    /// Le letture on che si incontrano di rado, tenute fuori dalle altre.
    #[serde(default)]
    pub on_rare: Vec<String>,
    /// La lettura on che pesa di piu' nei composti veri.
    ///
    /// Derivata pesando i composti sulla loro frequenza, non contandoli: per 生 da'
    /// セイ, mentre contarli darebbe ショウ. `None` se il kanji non ha letture on.
    pub primary_on: Option<String>,
    /// Le letture kun del kanji nudo, senza okurigana.
    pub kun: Vec<String>,
    /// La lettura kun con cui il kanji si legge piu' spesso **da solo**.
    ///
    /// Non e' derivata come [`Self::primary_on`], e non poteva esserlo: quella pesa i
    /// composti, ma dentro un composto una kun prende la forma legata, che e' un'altra
    /// dalla lettura da isolato. Verificato che sbaglia: direbbe いな per 稲, che da solo
    /// e' いね. Qui si guarda la voce di dizionario che e' il kanji da solo.
    ///
    /// `None` quasi sempre, e per due ragioni diverse: perche' di kun nuda ce n'e' una
    /// sola e non c'e' niente da scegliere (553 kanji su 595), o perche' il dato non
    /// discrimina. **Ne restano quattro marcati**: 雌, 雄, 女, 羽.
    pub primary_kun: Option<String>,
    #[serde(default)]
    pub kun_rare: Vec<String>,
    pub okurigana: Vec<Okurigana>,
    /// Le letture che il kanji prende nei nomi propri. Si mostrano, non si chiedono.
    pub nanori: Vec<String>,
    pub examples: Vec<Example>,
}

/// Da dove viene una tabella.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub dataset: String,
    /// kanjium non ha numeri di versione: la provenienza e' il commit che ha toccato
    /// per l'ultima volta il database.
    pub commit: String,
    pub committed: String,
    pub licence: String,
    pub url: String,
    /// Le fonti di terzi che kanjium si porta dentro, e la cui licenza si somma.
    pub includes: String,
}

/// I kanji di un livello.
#[derive(Debug, Clone, Deserialize)]
pub struct KanjiTable {
    version: u32,
    level: Level,
    source: Source,
    entries: Vec<Kanji>,
}

impl KanjiTable {
    /// Versione del formato del file, non della fonte: quella sta in [`Self::source`].
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn level(&self) -> Level {
        self.level
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    /// I kanji del livello, dai piu' frequenti ai meno.
    pub fn all(&self) -> &[Kanji] {
        &self.entries
    }

    /// Un kanji del livello, se c'e'.
    pub fn get(&self, character: &str) -> Option<&Kanji> {
        self.entries.iter().find(|k| k.character == character)
    }
}

/// I file, in ordine di livello. Sono costanti, quindi non costano niente finche'
/// nessuno li analizza.
static RAW: [&str; LEVELS as usize] = [
    include_str!("../../../data/kanji/levels/level-01.json"),
    include_str!("../../../data/kanji/levels/level-02.json"),
    include_str!("../../../data/kanji/levels/level-03.json"),
    include_str!("../../../data/kanji/levels/level-04.json"),
    include_str!("../../../data/kanji/levels/level-05.json"),
    include_str!("../../../data/kanji/levels/level-06.json"),
    include_str!("../../../data/kanji/levels/level-07.json"),
    include_str!("../../../data/kanji/levels/level-08.json"),
    include_str!("../../../data/kanji/levels/level-09.json"),
    include_str!("../../../data/kanji/levels/level-10.json"),
    include_str!("../../../data/kanji/levels/level-11.json"),
    include_str!("../../../data/kanji/levels/level-12.json"),
    include_str!("../../../data/kanji/levels/level-13.json"),
    include_str!("../../../data/kanji/levels/level-14.json"),
    include_str!("../../../data/kanji/levels/level-15.json"),
    include_str!("../../../data/kanji/levels/level-16.json"),
    include_str!("../../../data/kanji/levels/level-17.json"),
    include_str!("../../../data/kanji/levels/level-18.json"),
    include_str!("../../../data/kanji/levels/level-19.json"),
    include_str!("../../../data/kanji/levels/level-20.json"),
    include_str!("../../../data/kanji/levels/level-21.json"),
    include_str!("../../../data/kanji/levels/level-22.json"),
    include_str!("../../../data/kanji/levels/level-23.json"),
    include_str!("../../../data/kanji/levels/level-24.json"),
    include_str!("../../../data/kanji/levels/level-25.json"),
    include_str!("../../../data/kanji/levels/level-26.json"),
    include_str!("../../../data/kanji/levels/level-27.json"),
    include_str!("../../../data/kanji/levels/level-28.json"),
    include_str!("../../../data/kanji/levels/level-29.json"),
    include_str!("../../../data/kanji/levels/level-30.json"),
    include_str!("../../../data/kanji/levels/level-31.json"),
    include_str!("../../../data/kanji/levels/level-32.json"),
    include_str!("../../../data/kanji/levels/level-33.json"),
    include_str!("../../../data/kanji/levels/level-34.json"),
    include_str!("../../../data/kanji/levels/level-35.json"),
    include_str!("../../../data/kanji/levels/level-36.json"),
    include_str!("../../../data/kanji/levels/level-37.json"),
    include_str!("../../../data/kanji/levels/level-38.json"),
    include_str!("../../../data/kanji/levels/level-39.json"),
    include_str!("../../../data/kanji/levels/level-40.json"),
    include_str!("../../../data/kanji/levels/level-41.json"),
    include_str!("../../../data/kanji/levels/level-42.json"),
    include_str!("../../../data/kanji/levels/level-43.json"),
    include_str!("../../../data/kanji/levels/level-44.json"),
    include_str!("../../../data/kanji/levels/level-45.json"),
    include_str!("../../../data/kanji/levels/level-46.json"),
    include_str!("../../../data/kanji/levels/level-47.json"),
    include_str!("../../../data/kanji/levels/level-48.json"),
    include_str!("../../../data/kanji/levels/level-49.json"),
    include_str!("../../../data/kanji/levels/level-50.json"),
    include_str!("../../../data/kanji/levels/level-51.json"),
    include_str!("../../../data/kanji/levels/level-52.json"),
    include_str!("../../../data/kanji/levels/level-53.json"),
    include_str!("../../../data/kanji/levels/level-54.json"),
    include_str!("../../../data/kanji/levels/level-55.json"),
    include_str!("../../../data/kanji/levels/level-56.json"),
    include_str!("../../../data/kanji/levels/level-57.json"),
    include_str!("../../../data/kanji/levels/level-58.json"),
    include_str!("../../../data/kanji/levels/level-59.json"),
    include_str!("../../../data/kanji/levels/level-60.json"),
    include_str!("../../../data/kanji/levels/level-61.json"),
    include_str!("../../../data/kanji/levels/level-62.json"),
    include_str!("../../../data/kanji/levels/level-63.json"),
    include_str!("../../../data/kanji/levels/level-64.json"),
    include_str!("../../../data/kanji/levels/level-65.json"),
    include_str!("../../../data/kanji/levels/level-66.json"),
    include_str!("../../../data/kanji/levels/level-67.json"),
    include_str!("../../../data/kanji/levels/level-68.json"),
    include_str!("../../../data/kanji/levels/level-69.json"),
    include_str!("../../../data/kanji/levels/level-70.json"),
    include_str!("../../../data/kanji/levels/level-71.json"),
    include_str!("../../../data/kanji/levels/level-72.json"),
    include_str!("../../../data/kanji/levels/level-73.json"),
    include_str!("../../../data/kanji/levels/level-74.json"),
    include_str!("../../../data/kanji/levels/level-75.json"),
    include_str!("../../../data/kanji/levels/level-76.json"),
    include_str!("../../../data/kanji/levels/level-77.json"),
    include_str!("../../../data/kanji/levels/level-78.json"),
    include_str!("../../../data/kanji/levels/level-79.json"),
    include_str!("../../../data/kanji/levels/level-80.json"),
    include_str!("../../../data/kanji/levels/level-81.json"),
    include_str!("../../../data/kanji/levels/level-82.json"),
    include_str!("../../../data/kanji/levels/level-83.json"),
    include_str!("../../../data/kanji/levels/level-84.json"),
    include_str!("../../../data/kanji/levels/level-85.json"),
    include_str!("../../../data/kanji/levels/level-86.json"),
];

/// Le tabelle gia' analizzate. Ognuna si costruisce alla prima richiesta e poi resta.
static PARSED: [OnceLock<KanjiTable>; LEVELS as usize] =
    [const { OnceLock::new() }; LEVELS as usize];

/// La tabella di un livello. Il file viene analizzato alla prima richiesta.
pub fn table(level: Level) -> &'static KanjiTable {
    let i = usize::from(level.get() - 1);
    PARSED[i].get_or_init(|| parse(RAW[i], level))
}

/// I file sono inclusi nel binario, quindi un errore qui non e' un dato sbagliato
/// dell'utente ma un difetto della build: meglio accorgersene subito e rumorosamente.
fn parse(raw: &str, expected: Level) -> KanjiTable {
    let table: KanjiTable = serde_json::from_str(raw)
        .unwrap_or_else(|e| panic!("tabella del livello {expected} non leggibile: {e}"));
    assert_eq!(
        table.level, expected,
        "il file dichiara un livello diverso da quello atteso"
    );
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Quanti joyo ci sono, e non cambiano: la lista e' fissata per decreto.
    const JOYO: usize = 2136;

    #[test]
    fn tutti_i_livelli_si_leggono() {
        for level in Level::all() {
            let t = table(level);
            assert_eq!(t.version(), 1);
            assert_eq!(t.level(), level);
            assert!(!t.all().is_empty(), "livello {level} vuoto");
        }
    }

    #[test]
    fn i_livelli_coprono_i_joyo_una_volta_sola() {
        let mut visti = HashSet::new();
        for level in Level::all() {
            for k in table(level).all() {
                assert!(visti.insert(&k.character), "kanji doppio: {}", k.character);
                assert_eq!(k.level, level, "{} dichiara un livello diverso", k.character);
            }
        }
        assert_eq!(visti.len(), JOYO);
    }

    #[test]
    fn ogni_kanji_ha_di_che_fare_una_domanda() {
        for level in Level::all() {
            for k in table(level).all() {
                assert!(!k.meanings.is_empty(), "{} senza significati", k.character);
                assert!(
                    !k.on.is_empty() || !k.kun.is_empty() || !k.okurigana.is_empty(),
                    "{} senza nessuna lettura",
                    k.character
                );
                assert!(k.strokes > 0, "{} senza tratti", k.character);
            }
        }
    }

    #[test]
    fn la_lettura_primaria_e_una_delle_sue() {
        for level in Level::all() {
            for k in table(level).all() {
                match (&k.primary_on, k.on.is_empty()) {
                    (Some(p), false) => assert!(
                        k.on.contains(p),
                        "{}: la primaria {p} non e' fra le sue letture on",
                        k.character
                    ),
                    (None, true) => {}
                    _ => panic!("{}: primaria e letture on non si accordano", k.character),
                }
            }
        }
    }

    #[test]
    fn la_quota_di_occorrenza_singola_e_una_proporzione() {
        let mut misurati = 0;
        for level in Level::all() {
            for k in table(level).all() {
                if let Some(r) = k.alone_ratio {
                    assert!((0.0..=1.0).contains(&r), "{}: quota fuori scala {r}", k.character);
                    misurati += 1;
                }
            }
        }
        // Il corpus non contiene tutti i joyo, e va bene che lo dica invece di
        // inventare uno zero: uno zero vorrebbe dire "non ricorre mai da solo", che e'
        // un'altra cosa da "non lo so".
        assert_eq!(misurati, 1931, "i joyo che il corpus contiene");
    }

    #[test]
    fn le_letture_rare_stanno_fuori_dalle_altre() {
        let t = table(Level::new(9).unwrap());
        let ku = t.get("行").expect("行 sta al livello 9");
        assert_eq!(ku.on, ["コウ", "ギョウ"]);
        assert_eq!(ku.on_rare, ["アン"], "アン si incontra di rado");
        assert_eq!(ku.primary_on.as_deref(), Some("コウ"));
    }

    #[test]
    fn una_forma_con_due_letture_e_una_forma_sola() {
        let t = table(Level::new(9).unwrap());
        let ku = t.get("行").unwrap();
        let iku = ku.okurigana.iter().find(|o| o.form == "行く").unwrap();
        assert_eq!(iku.readings, ["いく", "ゆく"]);
    }

    #[test]
    fn la_lettura_on_primaria_pesa_i_composti_invece_di_contarli() {
        // 生 ha piu' composti con ショウ ma quelli che si leggono davvero (生活,
        // 学生, 先生) sono con セイ. Contare darebbe la risposta sbagliata.
        let vita = table(Level::new(5).unwrap()).get("生").unwrap();
        assert_eq!(vita.primary_on.as_deref(), Some("セイ"));
        assert_eq!(vita.on, ["セイ", "ショウ"]);
    }

    #[test]
    fn chi_vive_nei_composti_si_distingue_da_chi_sta_da_solo() {
        let vita = table(Level::new(5).unwrap()).get("生").unwrap();
        let io = table(Level::new(14).unwrap()).get("私").expect("私 sta al livello 14");
        assert!(vita.alone_ratio.unwrap() < 0.05, "生 vive nei composti");
        assert!(io.alone_ratio.unwrap() > 0.4, "私 sta quasi sempre da solo");
    }

    #[test]
    fn le_forme_comuni_sono_marcate() {
        let vita = table(Level::new(5).unwrap()).get("生").unwrap();
        let comune = |f: &str| vita.okurigana.iter().find(|o| o.form == f).unwrap().common;
        assert!(comune("生きる"));
        assert!(comune("生える"), "il corpus non la contiene ma kanjium la dice comune");
        assert!(!comune("生ける"));
    }

    #[test]
    fn tutte_le_tabelle_vengono_dalla_stessa_edizione() {
        let atteso = table(Level::new(1).unwrap()).source();
        assert_eq!(atteso.dataset, "kanjium");
        assert_eq!(atteso.licence, "CC BY-SA 4.0");
        assert!(atteso.includes.contains("EDRDG"), "l'attribuzione dell'EDRDG si somma");
        for level in Level::all() {
            assert_eq!(table(level).source(), atteso, "livello {level}");
        }
    }

    #[test]
    fn un_livello_fuori_scala_non_attraversa_il_confine() {
        assert!(serde_json::from_str::<Level>("3").is_ok());
        for fuori in ["0", "87", "255"] {
            assert!(
                serde_json::from_str::<Level>(fuori).is_err(),
                "{fuori} non e' un livello e non deve entrare"
            );
        }
    }

    #[test]
    fn i_livelli_hanno_tutti_la_stessa_misura_tranne_l_ultimo() {
        for level in Level::all() {
            let quanti = table(level).all().len();
            if level.get() < LEVELS {
                assert_eq!(quanti, 25, "livello {level}");
            } else {
                assert!((1..=25).contains(&quanti), "l'ultimo livello e' un resto");
            }
        }
    }

    #[test]
    fn la_difficolta_sale_lungo_il_percorso() {
        // L'ordine viene da un topologico sui componenti coi pareggi rotti per numero
        // di tratti: la conseguenza visibile e' che i segni si complicano andando
        // avanti. Se un domani il criterio cambiasse di nascosto, questo se ne
        // accorgerebbe.
        let medi = |level: Level| {
            let voci = table(level).all();
            voci.iter().map(|k| u32::from(k.strokes)).sum::<u32>() as f32 / voci.len() as f32
        };

        let primo = medi(Level::new(1).unwrap());
        let mezzo = medi(Level::new(43).unwrap());
        let ultimo = medi(Level::new(LEVELS).unwrap());

        assert!(primo < mezzo, "{primo} tratti contro {mezzo}");
        assert!(mezzo < ultimo, "{mezzo} tratti contro {ultimo}");
        assert!(primo < 4.0, "il primo livello e' fatto di segni semplici: {primo}");
    }

    #[test]
    fn i_livelli_stanno_dentro_i_loro_estremi() {
        assert_eq!(Level::new(0), None);
        assert_eq!(Level::new(LEVELS + 1), None);
        assert_eq!(Level::new(1).unwrap().next().unwrap().get(), 2);
        assert_eq!(Level::new(LEVELS).unwrap().next(), None);
        assert_eq!(Level::all().count(), usize::from(LEVELS));
    }
}
