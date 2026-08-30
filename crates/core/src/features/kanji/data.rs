//! Le tabelle dei kanji joyo.
//!
//! Come per i kana, il contenuto non sta nel codice: vive in
//! `crates/core/data/kanji/*.json`, porta un numero di versione e si rigenera invece
//! di modificarsi a mano. A differenza dei kana, pero', **non e' scritto da noi**:
//! deriva da KANJIDIC2, quindi ogni file dichiara anche da quale edizione del
//! dizionario e' uscito, nel campo [`Source`]. Senza, non si saprebbe piu' quale
//! versione del dato si sta studiando.
//!
//! # Perche' i file stanno nel binario
//!
//! Perche' sono stati **misurati**: tutti e sette insieme fanno 315 kB, dentro un APK
//! che ne pesa 17.000, e il primo anno da solo ne fa 12. La regola di tenere i dataset
//! fuori dal binario nasceva dal peso su mobile, e qui il peso non c'e'. Il caricamento
//! resta comunque **pigro e per grado**: allenando il primo anno, gli altri sei file
//! non vengono mai analizzati, ed e' li' che sta il costo vero, non nei byte a riposo.
//!
//! La regola resta valida dov'era pensata, cioe' per il vocabolario: JMdict e' due
//! ordini di grandezza piu' grande e non seguira' questa strada.
//!
//! # Perche' il dato e' fedele e non ripulito
//!
//! Le letture arrivano come le da' KANJIDIC2, punti e trattini compresi, e con le
//! stranezze che il dizionario si porta dietro (vedi [`Kanji::kun`]). Ripulirle qui
//! vorrebbe dire decidere nel modulo del contenuto come si formula una domanda, che e'
//! mestiere dell'esercizio: il contenuto dice cosa **e'** un kanji, l'esercizio sceglie
//! cosa chiederne.
//!
//! # Licenza
//!
//! KANJIDIC2 e' dell'EDRDG, in CC BY-SA 4.0, e la licenza si estende ai dati derivati:
//! queste tabelle non sono MIT come il resto del codice. Vedi
//! `crates/core/data/kanji/ATTRIBUTION.md`.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// L'anno di scuola in cui un kanji si insegna, secondo KANJIDIC2.
///
/// Da [`Grade::First`] a [`Grade::Sixth`] sono i sei anni della scuola elementare, i
/// kyoiku kanji. [`Grade::Secondary`] sono i joyo restanti, insegnati alle medie e alle
/// superiori: e' il grado 8 di KANJIDIC2, ed e' di gran lunga il piu' numeroso.
///
/// I gradi 9 e 10, i jinmeiyo ammessi solo nei nomi di persona, non entrano nelle
/// tabelle: non si studiano per leggere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Secondary,
}

/// I sette gradi, dal primo anno in avanti.
pub const GRADES: [Grade; 7] = [
    Grade::First,
    Grade::Second,
    Grade::Third,
    Grade::Fourth,
    Grade::Fifth,
    Grade::Sixth,
    Grade::Secondary,
];

impl Grade {
    /// L'anno di scuola elementare, da 1 a 6. `None` per i kanji delle medie.
    pub fn year(self) -> Option<u8> {
        match self {
            Self::First => Some(1),
            Self::Second => Some(2),
            Self::Third => Some(3),
            Self::Fourth => Some(4),
            Self::Fifth => Some(5),
            Self::Sixth => Some(6),
            Self::Secondary => None,
        }
    }
}

/// Un kanji, come lo descrive KANJIDIC2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kanji {
    /// Il segno. E' una stringa per la stessa ragione dei kana, cioe' per non
    /// promettere che stia in un `char`, anche se qui in pratica ci sta sempre.
    pub character: String,
    pub strokes: u8,
    /// Il rango di frequenza sui giornali, da 1 a 2501. `None` sui joyo che non
    /// rientrano nei duemilacinquecento piu' comuni.
    pub frequency: Option<u16>,
    /// Le letture on, quelle di origine cinese, in katakana.
    ///
    /// Quattro di loro cominciano con un trattino (`-ネン` di 縁): sono letture che
    /// compaiono solo in coda a un composto.
    pub on: Vec<String>,
    /// Le letture kun, quelle giapponesi, in hiragana.
    ///
    /// Portano due segni che non fanno parte della lettura:
    /// - il **punto** separa la parte scritta col kanji dall'okurigana: in `い.きる` il
    ///   kanji copre solo `い`;
    /// - il **trattino** segna prefissi e suffissi: `-り` di 人, `なま-` di 生.
    ///
    /// Due voci sono in katakana e non in hiragana, ed e' il dizionario a dirlo: 志 ha
    /// `シリング` (lo scellino) e 粉 ha `デシメートル`. Sono usi storici da unita' di
    /// misura, e chi costruisce un esercizio sulle letture fara' bene a scartarli.
    pub kun: Vec<String>,
    /// I significati in inglese.
    ///
    /// **Oggi nessuna schermata li mostra.** KANJIDIC2 li da' in inglese, spagnolo,
    /// francese e portoghese: l'italiano, che e' la lingua dell'interfaccia, non c'e'.
    /// Restano nel dato perche' fanno parte di cosa e' un kanji; usarli vorra' dire
    /// prima decidere in che lingua si mostrano.
    pub meanings: Vec<String>,
}

/// Da quale edizione di KANJIDIC2 viene una tabella.
///
/// Viaggia dentro il dato e non nel codice perche' e' una proprieta' del dato: due
/// tabelle rigenerate a mesi di distanza sono due cose diverse, e questo campo e'
/// l'unico posto in cui la differenza si vede. Serve anche alla schermata delle fonti,
/// che la licenza dell'EDRDG richiede.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub dataset: String,
    /// La versione del dizionario, nella forma `anno-progressivo`.
    pub database_version: String,
    pub created: String,
    pub licence: String,
    pub url: String,
}

/// I kanji di un grado, con la versione del contenuto e la fonte.
#[derive(Debug, Clone, Deserialize)]
pub struct KanjiTable {
    version: u32,
    grade: Grade,
    source: Source,
    entries: Vec<Kanji>,
}

impl KanjiTable {
    /// Versione del formato del file, non del dizionario: quella sta in
    /// [`KanjiTable::source`].
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn grade(&self) -> Grade {
        self.grade
    }

    /// Da quale edizione di KANJIDIC2 e' uscita questa tabella.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Tutti i kanji del grado, dai piu' frequenti ai meno.
    ///
    /// L'ordine e' quello del file, cioe' per rango di frequenza con in coda quelli
    /// che un rango non ce l'hanno. Cosi' "i primi N di questo grado" e' un prefisso
    /// di questa lista, non un filtro da scrivere.
    pub fn all(&self) -> &[Kanji] {
        &self.entries
    }
}

macro_rules! table {
    ($name:ident, $grade:expr, $file:literal) => {
        static $name: LazyLock<KanjiTable> =
            LazyLock::new(|| parse(include_str!(concat!("../../../data/kanji/", $file)), $grade));
    };
}

table!(FIRST, Grade::First, "first.json");
table!(SECOND, Grade::Second, "second.json");
table!(THIRD, Grade::Third, "third.json");
table!(FOURTH, Grade::Fourth, "fourth.json");
table!(FIFTH, Grade::Fifth, "fifth.json");
table!(SIXTH, Grade::Sixth, "sixth.json");
table!(SECONDARY, Grade::Secondary, "secondary.json");

/// La tabella di un grado. Il file viene analizzato alla prima richiesta, poi resta
/// in memoria: allenando un solo anno, gli altri sei non si toccano.
pub fn table(grade: Grade) -> &'static KanjiTable {
    match grade {
        Grade::First => &FIRST,
        Grade::Second => &SECOND,
        Grade::Third => &THIRD,
        Grade::Fourth => &FOURTH,
        Grade::Fifth => &FIFTH,
        Grade::Sixth => &SIXTH,
        Grade::Secondary => &SECONDARY,
    }
}

/// I file sono inclusi nel binario, quindi un errore qui non e' un dato sbagliato
/// dell'utente ma un difetto della build: meglio accorgersene subito e rumorosamente.
fn parse(raw: &str, expected: Grade) -> KanjiTable {
    let table: KanjiTable = serde_json::from_str(raw)
        .unwrap_or_else(|e| panic!("tabella kanji {expected:?} non leggibile: {e}"));
    assert_eq!(
        table.grade, expected,
        "il file dichiara un grado diverso da quello atteso"
    );
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Quanti kanji ha ogni grado. Sono numeri che non cambiano a caso: la lista dei
    /// joyo e' fissata per decreto, quindi se uno di questi si muove e' successo
    /// qualcosa che va guardato, non aggiustato.
    const ATTESI: [(Grade, usize); 7] = [
        (Grade::First, 80),
        (Grade::Second, 160),
        (Grade::Third, 200),
        (Grade::Fourth, 202),
        (Grade::Fifth, 193),
        (Grade::Sixth, 191),
        (Grade::Secondary, 1110),
    ];

    #[test]
    fn tutte_le_tabelle_si_leggono() {
        for (grade, quanti) in ATTESI {
            let t = table(grade);
            assert_eq!(t.version(), 1);
            assert_eq!(t.grade(), grade);
            assert_eq!(t.all().len(), quanti, "grado {grade:?}");
        }
    }

    #[test]
    fn i_joyo_sono_duemilacentotrentasei() {
        let totale: usize = GRADES.iter().map(|&g| table(g).all().len()).sum();
        assert_eq!(totale, 2136, "la lista ufficiale dei joyo");
    }

    #[test]
    fn nessun_kanji_sta_in_due_gradi() {
        let mut visti = HashSet::new();
        for grade in GRADES {
            for k in table(grade).all() {
                assert!(visti.insert(&k.character), "kanji doppio: {}", k.character);
            }
        }
    }

    #[test]
    fn ogni_kanji_ha_almeno_una_lettura() {
        for grade in GRADES {
            for k in table(grade).all() {
                assert!(
                    !k.on.is_empty() || !k.kun.is_empty(),
                    "{} senza letture: non ci si puo' costruire una domanda",
                    k.character
                );
                assert!(k.strokes > 0, "{} senza tratti", k.character);
            }
        }
    }

    #[test]
    fn tutte_le_tabelle_vengono_dalla_stessa_edizione() {
        // Mischiare due edizioni di KANJIDIC2 nello stesso binario vorrebbe dire
        // studiare due dati diversi senza saperlo: si rigenera tutto insieme.
        let atteso = table(Grade::First).source();
        assert_eq!(atteso.dataset, "KANJIDIC2");
        assert_eq!(atteso.licence, "CC BY-SA 4.0");
        for grade in GRADES {
            assert_eq!(table(grade).source(), atteso, "grado {grade:?}");
        }
    }

    #[test]
    fn le_letture_arrivano_come_le_da_il_dizionario() {
        let t = table(Grade::First);
        let vita = t.all().iter().find(|k| k.character == "生").unwrap();

        // Il punto separa il kanji dall'okurigana, il trattino segna i suffissi:
        // restano nel dato perche' sono informazione, e sara' l'esercizio a decidere
        // cosa mostrarne.
        assert!(vita.kun.contains(&"い.きる".to_owned()));
        assert!(vita.kun.contains(&"なま-".to_owned()));
        assert_eq!(vita.on, ["セイ", "ショウ"]);
    }

    #[test]
    fn i_kanji_sono_ordinati_per_frequenza() {
        for grade in GRADES {
            let voci = table(grade).all();
            let ranghi: Vec<_> = voci.iter().map(|k| k.frequency).collect();

            // Prima quelli con un rango, in ordine crescente, poi quelli senza.
            let con_rango = ranghi.iter().take_while(|f| f.is_some()).count();
            assert!(
                ranghi[con_rango..].iter().all(|f| f.is_none()),
                "grado {grade:?}: un rango dopo i senza rango"
            );
            assert!(
                ranghi[..con_rango].windows(2).all(|w| w[0] <= w[1]),
                "grado {grade:?}: ranghi non crescenti"
            );
        }
        // Il primo anno comincia dal kanji piu' comune della lingua.
        assert_eq!(table(Grade::First).all()[0].character, "日");
    }

    #[test]
    fn il_grado_dice_l_anno_di_scuola() {
        assert_eq!(Grade::First.year(), Some(1));
        assert_eq!(Grade::Sixth.year(), Some(6));
        // I kanji delle medie non stanno in un anno solo, quindi non ne hanno uno.
        assert_eq!(Grade::Secondary.year(), None);
    }
}
