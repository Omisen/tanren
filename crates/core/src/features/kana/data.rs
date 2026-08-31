//! La tabella dei kana.
//!
//! Il contenuto non sta nel codice: vive in `crates/core/data/kana/*.json`, si
//! modifica senza ricompilare la logica e porta un numero di versione. I file sono
//! inclusi nel binario perche' pesano pochi kilobyte, ma restano dati versionati, non
//! costanti scritte a mano. I dataset grossi che arriveranno dopo, kanji e
//! vocabolario, non seguiranno questa strada.
//!
//! Il caricamento e' pigro e per sillabario: se una sessione allena solo l'hiragana,
//! il file dei katakana non viene mai letto ne' analizzato.
//!
//! I due file sono generati da `crates/core/data/kana/generate.py`, che scrive
//! l'hiragana e ne deriva il katakana. Le due tabelle sono quindi allineate voce per
//! voce, e un test lo verifica.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// Quale dei due sillabari.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Syllabary {
    Hiragana,
    Katakana,
}

/// A quale famiglia appartiene un kana.
///
/// Serve a proporre un allenamento per gradi: prima i 46 segni di base, poi le
/// sonorizzazioni, infine le combinazioni.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KanaGroup {
    /// I 46 segni del gojuon.
    Base,
    /// Sonorizzati con il segno `゛`: が, ざ, だ, ば.
    Dakuten,
    /// Semi sonorizzati con il segno `゜`: ぱ.
    Handakuten,
    /// Combinazioni con ゃ, ゅ, ょ: きゃ, しゅ, りょ.
    Yoon,
    /// I 外来音, i suoni presi da altre lingue: ファ, ヴ, ティ, チェ.
    ///
    /// **Esiste solo in katakana.** Non e' una scelta di presentazione: l'hiragana
    /// questi suoni non li scrive, quindi chiederla sull'hiragana non da' una
    /// famiglia vuota per caso, la da' vuota per definizione.
    Gairaion,
}

/// Un singolo segno del sillabario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kana {
    /// Il segno. E' una stringa e non un `char` perche' le combinazioni valgono due
    /// caratteri: きゃ.
    pub character: String,
    /// Le trascrizioni accettate. La prima e' quella canonica, da mostrare.
    pub romaji: Vec<String>,
    pub group: KanaGroup,
    /// La riga del gojuon a cui appartiene, sonorizzazione compresa: か e きゃ stanno
    /// in `ka`, が in `ga`.
    pub row: String,
}

/// La tabella di un sillabario, con la versione del contenuto che l'ha prodotta.
#[derive(Debug, Clone, Deserialize)]
pub struct KanaTable {
    version: u32,
    syllabary: Syllabary,
    entries: Vec<Kana>,
}

impl KanaTable {
    /// Versione del file di contenuto, utile quando il dato cambiera' sotto i piedi
    /// di progressi gia' registrati.
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn syllabary(&self) -> Syllabary {
        self.syllabary
    }

    /// Tutti i segni del sillabario, nell'ordine tradizionale.
    pub fn all(&self) -> &[Kana] {
        &self.entries
    }

    /// I soli segni di una famiglia.
    pub fn group(&self, group: KanaGroup) -> impl Iterator<Item = &Kana> {
        self.entries.iter().filter(move |k| k.group == group)
    }

    /// I soli segni di una riga del gojuon.
    pub fn row<'a>(&'a self, row: &'a str) -> impl Iterator<Item = &'a Kana> {
        self.entries.iter().filter(move |k| k.row == row)
    }
}

static HIRAGANA: LazyLock<KanaTable> = LazyLock::new(|| {
    parse(
        include_str!("../../../data/kana/hiragana.json"),
        Syllabary::Hiragana,
    )
});

static KATAKANA: LazyLock<KanaTable> = LazyLock::new(|| {
    parse(
        include_str!("../../../data/kana/katakana.json"),
        Syllabary::Katakana,
    )
});

/// La tabella di un sillabario. Il file viene letto e analizzato alla prima
/// richiesta, poi resta in memoria.
pub fn table(syllabary: Syllabary) -> &'static KanaTable {
    match syllabary {
        Syllabary::Hiragana => &HIRAGANA,
        Syllabary::Katakana => &KATAKANA,
    }
}

/// I file sono inclusi nel binario, quindi un errore qui non e' un dato sbagliato
/// dell'utente ma un difetto della build: meglio accorgersene subito e rumorosamente.
/// I test coprono entrambi i file, cosi' il problema emerge prima di arrivare a un
/// dispositivo.
fn parse(raw: &str, expected: Syllabary) -> KanaTable {
    let table: KanaTable = serde_json::from_str(raw)
        .unwrap_or_else(|e| panic!("tabella kana {expected:?} non leggibile: {e}"));
    assert_eq!(
        table.syllabary, expected,
        "il file dichiara un sillabario diverso da quello atteso"
    );
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::text::normalize_reading;

    const SILLABARI: [Syllabary; 2] = [Syllabary::Hiragana, Syllabary::Katakana];

    #[test]
    fn entrambe_le_tabelle_si_leggono() {
        for s in SILLABARI {
            let t = table(s);
            assert_eq!(t.version(), 1);
            assert_eq!(t.syllabary(), s);
            // Il katakana ne ha 19 in piu': i 外来音, che l'hiragana non ha.
            let attese = match s {
                Syllabary::Hiragana => 107,
                Syllabary::Katakana => 126,
            };
            assert_eq!(t.all().len(), attese, "sillabario {s:?}");
        }
    }

    #[test]
    fn le_famiglie_hanno_le_dimensioni_attese() {
        for s in SILLABARI {
            let t = table(s);
            assert_eq!(t.group(KanaGroup::Base).count(), 46);
            assert_eq!(t.group(KanaGroup::Dakuten).count(), 20);
            assert_eq!(t.group(KanaGroup::Handakuten).count(), 5);
            assert_eq!(t.group(KanaGroup::Yoon).count(), 36);
        }
    }

    #[test]
    fn i_gairaion_stanno_solo_nel_katakana() {
        assert_eq!(
            table(Syllabary::Hiragana)
                .group(KanaGroup::Gairaion)
                .count(),
            0,
            "l'hiragana non scrive i suoni presi da fuori"
        );
        assert_eq!(
            table(Syllabary::Katakana)
                .group(KanaGroup::Gairaion)
                .count(),
            19
        );
    }

    #[test]
    fn nessun_segno_e_ripetuto() {
        for s in SILLABARI {
            let mut visti = std::collections::HashSet::new();
            for k in table(s).all() {
                assert!(visti.insert(&k.character), "segno doppio: {}", k.character);
            }
        }
    }

    #[test]
    fn ogni_segno_ha_almeno_una_trascrizione() {
        for s in SILLABARI {
            for k in table(s).all() {
                assert!(!k.romaji.is_empty(), "{} senza romaji", k.character);
                for r in &k.romaji {
                    assert!(
                        r.chars().all(|c| c.is_ascii_lowercase()),
                        "romaji fuori formato: {r}"
                    );
                }
            }
        }
    }

    #[test]
    fn ogni_segno_sta_nel_proprio_sillabario() {
        for k in table(Syllabary::Hiragana).all() {
            for c in k.character.chars() {
                assert!(
                    ('\u{3041}'..='\u{3096}').contains(&c),
                    "{c} non e' hiragana"
                );
            }
        }
        for k in table(Syllabary::Katakana).all() {
            for c in k.character.chars() {
                assert!(
                    ('\u{30A1}'..='\u{30F6}').contains(&c),
                    "{c} non e' katakana"
                );
            }
        }
    }

    #[test]
    fn le_due_tabelle_sono_allineate_sulle_famiglie_condivise() {
        // Le quattro famiglie condivise **sono** la stessa tabella in due grafie, e qui
        // si verifica voce per voce. I 外来音 no, e non e' l'invariante che si allenta
        // per far tornare il test: e' il suo perimetro detto per intero. Un suono che
        // l'hiragana non scrive non ha un hiragana a cui corrispondere, quindi
        // pretendere qui una coppia vorrebbe dire pretendere che esista.
        let hira = table(Syllabary::Hiragana).all();
        let kata: Vec<_> = table(Syllabary::Katakana)
            .all()
            .iter()
            .filter(|k| k.group != KanaGroup::Gairaion)
            .collect();
        assert_eq!(hira.len(), kata.len());

        for (h, k) in hira.iter().zip(kata) {
            assert_eq!(h.romaji, k.romaji);
            assert_eq!(h.group, k.group);
            assert_eq!(h.row, k.row);
            // Il katakana normalizzato deve ricadere sull'hiragana corrispondente:
            // qui la tabella e la normalizzazione dello step precedente si
            // controllano a vicenda.
            assert_eq!(normalize_reading(&k.character), h.character);
        }
    }

    #[test]
    fn si_puo_chiedere_una_singola_riga() {
        let ka: Vec<_> = table(Syllabary::Hiragana).row("ka").collect();
        // か き く け こ piu' le tre combinazioni きゃ きゅ きょ.
        assert_eq!(ka.len(), 8);
        assert!(ka.iter().any(|k| k.character == "か"));
        assert!(ka.iter().any(|k| k.character == "きゃ"));
        // が sta nella riga ga, non qui.
        assert!(ka.iter().all(|k| k.character != "が"));
    }
}
