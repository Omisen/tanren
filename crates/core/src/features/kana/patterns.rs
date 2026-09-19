//! Le due regole di scrittura che si mostrano e non si chiedono.
//!
//! Il **sokuon** raddoppia la consonante che segue (`っ` piu' `k` vale `kk`) e le
//! **vocali lunghe** allungano. Non sono segni con una lettura isolata: sono regole che
//! si applicano dentro le parole, e la domanda dell'esercizio, «che lettura ha questo
//! segno», a queste non si puo' porre.
//!
//! # Perche' stanno in un modulo e in un file loro
//!
//! Perche' devono restare **fuori dal motore**, e il modo di garantirlo non e'
//! ricordarselo: e' non dargli un tipo che ci possa entrare. Una famiglia dell'ambito e'
//! un [`KanaGroup`](super::data::KanaGroup), che vive dentro `Scope::groups` e guida
//! `Scope::items()`; se queste due fossero due varianti di quell'enum diventerebbero
//! selezionabili e serializzabili nell'ambito, e con la regola «lista vuota significa
//! tutte» finirebbero dentro una sessione.
//!
//! Con un tipo separato l'isolamento regge su due livelli indipendenti:
//!
//! - **sul tipo**: `Scope::groups` e' un `Vec<KanaGroup>`, e [`PatternGroup`] non lo e',
//!   quindi non ci puo' finire nemmeno per errore di chi chiama;
//! - **sul dato**: `Scope::items()` filtra la tabella del sillabario, che っ e ー non li
//!   contiene affatto, quindi anche un ambito corrotto non avrebbe niente da pescare.
//!
//! Quando si decidera' come si insegnano davvero, sara' un esercizio nuovo con le sue
//! domande, non una famiglia in piu' da spuntare.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::features::kana::data::{Syllabary, Vowel};

/// Quale delle due regole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternGroup {
    /// Il sokuon: っ raddoppia la consonante che segue.
    Double,
    /// Le vocali lunghe.
    Long,
}

/// Una casella da mostrare: cosa si scrive e come si legge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternCell {
    /// Quello che si vede. Non e' per forza un segno solo, e nel sokuon non e' nemmeno
    /// tutto giapponese: `っ+k` dice la regola meglio di qualunque carattere, perche' il
    /// sokuon non e' un carattere che si legge.
    pub character: String,
    pub romaji: String,
    /// In quale colonna della tavola va messa, e `None` quando non ne ha una.
    ///
    /// **Qui la colonna e' scritta nel dato e non ricavata dal romaji**, al contrario
    /// delle tabelle: li' la colonna e' l'ultima lettera della trascrizione ed e' vero
    /// per costruzione, qui no. えい e' una **e** lunga ma la sua trascrizione finisce
    /// per `i`, e おう e' una **o** lunga che finisce per `u`: ricavarla le metterebbe
    /// nelle colonne sbagliate. Il sokuon invece una colonna non ce l'ha proprio.
    pub column: Option<Vowel>,
}

/// Una regola, con le sue caselle disposte in righe.
///
/// Le righe non portano il nome di una riga del gojuon, perche' non ne hanno una: sono
/// solo il modo in cui la regola si dispone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub group: PatternGroup,
    pub rows: Vec<Vec<PatternCell>>,
}

#[derive(Debug, Deserialize)]
struct PatternFile {
    version: u32,
    hiragana: Vec<Pattern>,
    katakana: Vec<Pattern>,
}

static PATTERNS: LazyLock<PatternFile> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/kana/patterns.json"))
        .unwrap_or_else(|e| panic!("tabella delle regole non leggibile: {e}"))
});

/// Versione del file di contenuto.
pub fn version() -> u32 {
    PATTERNS.version
}

/// Le regole da mostrare per un sillabario.
///
/// Il file e' uno solo e si legge tutto insieme, a differenza delle tabelle, che sono
/// per sillabario e si leggono pigramente: li' il caricamento pigro serve perche' un
/// file pesa sedici kilobyte, qui ce ne sono due in tutto.
///
/// **Il katakana non si deriva dall'hiragana**, come invece succede per le tabelle: la
/// vocale lunga in katakana si scrive col choonpu ー e non raddoppiando, quindi アア
/// sarebbe ortograficamente sbagliato. Il dato dichiara i due sillabari per esteso.
pub fn patterns(syllabary: Syllabary) -> &'static [Pattern] {
    match syllabary {
        Syllabary::Hiragana => &PATTERNS.hiragana,
        Syllabary::Katakana => &PATTERNS.katakana,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::kana::data::table;

    const SILLABARI: [Syllabary; 2] = [Syllabary::Hiragana, Syllabary::Katakana];

    #[test]
    fn il_file_si_legge_e_porta_le_due_regole() {
        assert_eq!(version(), 1);
        for s in SILLABARI {
            let p = patterns(s);
            assert_eq!(p.len(), 2, "sillabario {s:?}");
            assert_eq!(p[0].group, PatternGroup::Double);
            assert_eq!(p[1].group, PatternGroup::Long);
            assert!(
                p.iter().all(|g| g.rows.iter().all(|r| !r.is_empty())),
                "una riga vuota non si disegna"
            );
        }
    }

    #[test]
    fn il_sokuon_usa_il_segno_del_proprio_sillabario() {
        let cella = |s| patterns(s)[0].rows[0][0].character.clone();
        assert_eq!(cella(Syllabary::Hiragana), "っ+k");
        assert_eq!(cella(Syllabary::Katakana), "ッ+k");

        // Nessuna colonna: il sokuon non sta in nessuna colonna della tavola.
        for s in SILLABARI {
            assert!(patterns(s)[0].rows[0].iter().all(|c| c.column.is_none()));
        }
    }

    #[test]
    fn le_vocali_lunghe_in_katakana_usano_il_choonpu() {
        // **Il test che conta di questo modulo.** Un gruppo che esiste per fare da
        // riferimento grammaticale non puo' mostrare una forma sbagliata, e アア lo
        // sarebbe: in katakana la vocale lunga si scrive col ー.
        let kata = &patterns(Syllabary::Katakana)[1];
        assert_eq!(kata.rows.len(), 1, "えい e おう sono dell'hiragana");
        for c in &kata.rows[0] {
            assert!(
                c.character.ends_with('ー'),
                "{} non usa il choonpu",
                c.character
            );
        }

        // L'hiragana invece raddoppia, e ha la seconda riga.
        let hira = &patterns(Syllabary::Hiragana)[1];
        assert_eq!(hira.rows.len(), 2);
        assert!(hira.rows.iter().flatten().all(|c| !c.character.contains('ー')));
    }

    #[test]
    fn la_colonna_delle_vocali_lunghe_non_segue_il_romaji() {
        // Se si ricavasse dalla trascrizione come nelle tabelle, えい finirebbe nella
        // colonna i e おう nella u, cioe' nel posto sbagliato.
        let hira = &patterns(Syllabary::Hiragana)[1];
        let seconda = &hira.rows[1];

        assert_eq!(seconda[0].romaji, "ei");
        assert_eq!(seconda[0].column, Some(Vowel::E));
        assert_eq!(seconda[1].romaji, "ou");
        assert_eq!(seconda[1].column, Some(Vowel::O));
    }

    #[test]
    fn niente_di_tutto_questo_sta_nelle_tabelle_dell_esercizio() {
        // L'invariante vera: quello che si mostra qui non puo' finire in una sessione,
        // perche' la tabella che l'ambito filtra non lo contiene affatto. E' la meta'
        // «sul dato» dell'isolamento descritto in cima al modulo; l'altra meta', quella
        // «sul tipo», la garantisce il compilatore e non si puo' scrivere come test.
        for s in SILLABARI {
            let segni: Vec<&str> = table(s).all().iter().map(|k| k.character.as_str()).collect();
            for g in patterns(s) {
                for c in g.rows.iter().flatten() {
                    assert!(
                        !segni.contains(&c.character.as_str()),
                        "{} e' finito anche nella tabella dell'esercizio",
                        c.character
                    );
                }
            }
        }
    }
}
