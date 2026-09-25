//! Leggere un mazzo da un CSV, e dire cosa non va prima di scrivere niente.
//!
//! # Perche' il parse sta qui e non nel frontend
//!
//! Perche' **quali colonne ci sono, in che ordine e quali sono obbligatorie** e' sapere
//! sulla forma di una carta, cioe' dominio, e il principio 4 dice che la UI non ne
//! contiene. Sta anche accanto alla validazione, che e' gia' qui: se il parse vivesse di
//! la', «questa riga e' buona» finirebbe per essere deciso in due posti, e due copie di
//! una regola si sganciano al primo ritocco.
//!
//! # Cosa attraversa il confine, e perche' e' una stringa
//!
//! Una `String` gia' decodificata, non dei byte. Il frontend legge il file a byte e lo
//! decodifica con un decodificatore **severo**, perche' e' li' che si risponde alla
//! domanda «questo file e' UTF-8?»: un CSV in Shift-JIS letto come UTF-8 non fallisce,
//! restituisce mojibake dove c'e' il giapponese e testo giusto dove c'e' il latino, che
//! e' il modo peggiore in cui un difetto puo' presentarsi. Misurato sul dispositivo.
//!
//! Quando quella stringa arriva qui e' UTF-8 **per costruzione del tipo**, quindi questo
//! modulo riceve una garanzia invece di una speranza e non ha niente da ricontrollare.
//!
//! # Gli errori sono dati, non frasi
//!
//! Il core dice quale riga e quale problema; «riga 14: il campo giapponese e' vuoto» lo
//! scrive la schermata. E' la stessa regola di `asks` e di `Gate::Closed`: la lingua
//! dell'interfaccia non entra nel core.

use serde::Serialize;

use super::deck::{self, Content, MAX_ALTERNATIVES};

/// Le colonne fisse, nell'ordine in cui stanno nel file.
///
/// Il furigana sta in terza e non in seconda perche' le due obbligatorie vengono prima:
/// chi compila il template a mano riempie le prime due e puo' fermarsi li'.
const FIXED: [&str; 3] = ["japanese", "meaning", "furigana"];

/// Come si chiama la colonna dell'alternativo numero `n`, contando da 1.
fn alt(n: usize) -> String {
    format!("alt_{n}")
}

/// L'intestazione che il template scrive e che l'import si aspetta.
///
/// **Quante colonne `alt_` ci siano lo dice [`MAX_ALTERNATIVES`]**, non un otto scritto
/// a mano: il tetto e' una decisione di prodotto che puo' cambiare, e il giorno che
/// cambia il template deve seguirlo da solo.
pub fn columns() -> Vec<String> {
    FIXED
        .iter()
        .map(|c| (*c).to_owned())
        .chain((1..=MAX_ALTERNATIVES).map(alt))
        .collect()
}

/// Il file da scaricare per sapere cosa scrivere: la sola riga di intestazione.
///
/// **Senza riga d'esempio**, ed e' una scelta. Una riga d'esempio si spiegherebbe
/// meglio, ma se chi compila si dimentica di cancellarla diventa **una carta vera**:
/// l'import non ha deduplica e non e' parziale, quindi entrerebbe senza che niente lo
/// segnali. Spiegare come si compila e' compito della schermata, che e' dove si e'
/// quando si scarica il file.
pub fn template() -> String {
    format!("{}\n", columns().join(","))
}

/// Una riga del file, gia' divisa nei campi ma non ancora ripulita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    japanese: String,
    meaning: String,
    furigana: String,
    alternatives: Vec<String>,
}

impl Row {
    /// La riga nella forma che il resto della materia sa scrivere.
    pub fn content(&self) -> Content<'_> {
        Content {
            japanese: &self.japanese,
            meaning: &self.meaning,
            alternatives: &self.alternatives,
            furigana: &self.furigana,
        }
    }
}

/// Una riga che non va, e perche'.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowError {
    /// La riga **del file**, non l'indice della carta.
    ///
    /// E' il numero che si legge nel foglio di calcolo, intestazione compresa, perche'
    /// e' li' che chi corregge deve andare. Lo conta il lettore CSV e non noi, cosi'
    /// resta giusto anche quando un campo fra virgolette contiene un a capo.
    pub line: u64,
    pub problem: Problem,
}

/// Cosa c'e' che non va in una riga.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    /// La prima riga non porta i nomi delle colonne che servono.
    Header,
    /// Un campo che deve avere del testo dentro e' arrivato vuoto.
    EmptyField { field: String },
    /// Sono arrivati piu' valori di quanti se ne accettino.
    TooManyValues { field: String, max: usize },
    /// La riga ha piu' colonne dell'intestazione.
    ///
    /// Quasi sempre vuol dire una virgola dentro un campo senza virgolette, cioe' il
    /// caso che una divisione fatta a mano sbaglierebbe in silenzio.
    TooManyColumns { found: usize, expected: usize },
    /// Il file non si legge affatto: virgolette non chiuse, o simili.
    Malformed,
}

/// Cosa dice un file: le carte che ne verrebbero, oppure le righe da correggere.
///
/// **Lo stesso tipo serve al controllo e all'import**, e le due cose sono diverse solo
/// in cosa conta `cards`: guardando e' quante carte **verrebbero**, importando e'
/// quante ne sono state **scritte**. Un tipo per ciascuno sarebbe stato due tipi
/// identici con due nomi.
///
/// **Un rifiuto non e' un errore del ponte.** E' un esito, come lo e' una risposta
/// sbagliata: il file e' arrivato e si e' capito benissimo cosa c'e' dentro, ed e' che
/// non si puo' usare. Farne un `CoreError` vorrebbe anche dire far conoscere al livello
/// condiviso cosa sia una riga di CSV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Review {
    /// Il file si puo' usare.
    Ready { cards: usize },
    /// Il file non si usa, e **non se ne scrive nessuna riga**: o tutto o niente.
    Rejected { errors: Vec<RowError> },
}

/// Guarda un file senza scrivere niente.
pub fn review(csv: &str) -> Review {
    match read(csv) {
        Ok(righe) => Review::Ready { cards: righe.len() },
        Err(errors) => Review::Rejected { errors },
    }
}

/// Legge il file e restituisce le righe, **oppure** tutto quello che non va.
///
/// # O tutto o niente
///
/// Non c'e' una via di mezzo, e non e' pigrizia: un import parziale lascerebbe chi
/// studia a indovinare cosa sia entrato, e con due soli campi obbligatori e il template
/// gia' intestato sbagliare e' difficile. Quindi o il file e' usabile per intero, o si
/// torna indietro con l'elenco delle righe da correggere.
///
/// # Cosa si perdona
///
/// Le **righe vuote** si saltano invece di contarle errori: un foglio di calcolo le
/// lascia in fondo di continuo, e rifiutare un file per una riga che non dice niente
/// sarebbe pedanteria. Si salta una riga in cui **tutti** i campi sono vuoti, non una a
/// cui manca solo il giapponese, che e' un errore vero.
pub fn read(csv: &str) -> Result<Vec<Row>, Vec<RowError>> {
    let attese = columns();

    let mut lettore = csv::ReaderBuilder::new()
        // Le righe possono avere meno campi dell'intestazione: chi compila spesso si
        // ferma alle prime colonne, e le altre semplicemente non ci sono.
        .flexible(true)
        .from_reader(csv.as_bytes());

    // L'intestazione si controlla per prima: senza, la prima carta verrebbe mangiata
    // come se fosse l'intestazione, e sparirebbe senza che niente lo dica.
    match lettore.headers() {
        Ok(trovata) => {
            let trovata: Vec<String> = trovata.iter().map(|c| c.trim().to_lowercase()).collect();
            if trovata != attese {
                return Err(vec![RowError {
                    line: 1,
                    problem: Problem::Header,
                }]);
            }
        }
        Err(_) => {
            return Err(vec![RowError {
                line: 1,
                problem: Problem::Malformed,
            }]);
        }
    }

    let mut righe = Vec::new();
    let mut errori = Vec::new();

    for record in lettore.records() {
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                errori.push(RowError {
                    line: e.position().map_or(0, csv::Position::line),
                    problem: Problem::Malformed,
                });
                continue;
            }
        };

        let line = record.position().map_or(0, csv::Position::line);

        if record.iter().all(|campo| campo.trim().is_empty()) {
            continue;
        }

        if record.len() > attese.len() {
            errori.push(RowError {
                line,
                problem: Problem::TooManyColumns {
                    found: record.len(),
                    expected: attese.len(),
                },
            });
            continue;
        }

        let campo = |i: usize| record.get(i).unwrap_or_default().to_owned();
        let riga = Row {
            japanese: campo(0),
            meaning: campo(1),
            furigana: campo(2),
            alternatives: (FIXED.len()..attese.len()).map(campo).collect(),
        };

        // La validazione e' quella della creazione manuale, chiamata e non ricopiata.
        match deck::clean(riga.content()) {
            Ok(_) => righe.push(riga),
            Err(e) => errori.push(RowError {
                line,
                problem: problem(e),
            }),
        }
    }

    if errori.is_empty() {
        Ok(righe)
    } else {
        Err(errori)
    }
}

/// Traduce il rifiuto della validazione nel motivo che attraversa il confine.
fn problem(e: crate::shared::error::CoreError) -> Problem {
    use crate::shared::error::CoreError;
    match e {
        CoreError::EmptyField { field } => Problem::EmptyField { field },
        CoreError::TooManyValues { field, max } => Problem::TooManyValues { field, max },
        // `clean` non ne produce altri. Se un domani ne producesse, questo dice
        // «la riga non va» invece di far finta che vada.
        _ => Problem::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un file buono, scritto come lo scriverebbe chi compila il template.
    fn file() -> String {
        format!(
            "{}\n日本語,japanese,にほんご,the japanese language,,,,,,,\n猫,cat,,,,,,,,,\n",
            columns().join(",")
        )
    }

    fn solo_errori(csv: &str) -> Vec<RowError> {
        read(csv).expect_err("doveva essere rifiutato")
    }

    #[test]
    fn guardare_un_file_buono_dice_quante_carte_ne_verrebbero() {
        assert_eq!(review(&file()), Review::Ready { cards: 2 });
    }

    #[test]
    fn guardare_un_file_rotto_dice_le_righe_e_non_un_conteggio() {
        let csv = format!("{}\n,orphan,,,,,,,,,\n", columns().join(","));
        let Review::Rejected { errors } = review(&csv) else {
            panic!("doveva essere rifiutato");
        };
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn il_template_porta_le_colonne_in_ordine_e_niente_altro() {
        let t = template();
        assert_eq!(t, "japanese,meaning,furigana,alt_1,alt_2,alt_3,alt_4,alt_5,alt_6,alt_7,alt_8\n");
        // Una riga sola: nessun esempio che possa entrare per sbaglio.
        assert_eq!(t.lines().count(), 1);
    }

    #[test]
    fn le_colonne_alternative_le_conta_il_dominio() {
        assert_eq!(columns().len(), FIXED.len() + MAX_ALTERNATIVES);
        assert_eq!(columns().last().unwrap(), &alt(MAX_ALTERNATIVES));
    }

    #[test]
    fn un_file_buono_da_le_sue_righe_nell_ordine_del_file() {
        let righe = read(&file()).unwrap();
        assert_eq!(righe.len(), 2);
        assert_eq!(righe[0].japanese, "日本語");
        assert_eq!(righe[1].japanese, "猫");
    }

    #[test]
    fn il_giapponese_arriva_intero() {
        let righe = read(&file()).unwrap();
        let (japanese, meaning, alternatives, furigana) =
            deck::clean(righe[0].content()).unwrap();
        assert_eq!(japanese, "日本語");
        assert_eq!(meaning, "japanese");
        assert_eq!(furigana.as_deref(), Some("にほんご"));
        assert_eq!(alternatives, ["the japanese language"]);
    }

    #[test]
    fn le_caselle_vuote_non_diventano_risposte() {
        let righe = read(&file()).unwrap();
        // La seconda riga ha otto caselle alternative tutte vuote.
        let (_, _, alternatives, furigana) = deck::clean(righe[1].content()).unwrap();
        assert!(alternatives.is_empty());
        assert_eq!(furigana, None);
    }

    /// E' il motivo per cui si usa un lettore CSV vero invece di dividere sulle virgole.
    #[test]
    fn una_virgola_dentro_un_campo_fra_virgolette_non_divide_niente() {
        let csv = format!("{}\nこんにちは,\"hello, there\",,,,,,,,,\n", columns().join(","));
        let righe = read(&csv).unwrap();
        assert_eq!(righe[0].meaning, "hello, there");
    }

    #[test]
    fn una_riga_senza_giapponese_dice_quale_riga_e_quale_campo() {
        let csv = format!("{}\n日本語,japanese,,,,,,,,,\n,orphan,,,,,,,,,\n", columns().join(","));
        let errori = solo_errori(&csv);
        assert_eq!(errori.len(), 1);
        assert_eq!(errori[0].line, 3, "la riga e' quella del file, intestazione compresa");
        assert_eq!(
            errori[0].problem,
            Problem::EmptyField {
                field: "japanese".to_owned()
            }
        );
    }

    #[test]
    fn una_riga_senza_significato_e_un_errore_come_l_altra() {
        let csv = format!("{}\n猫,,,,,,,,,,\n", columns().join(","));
        assert_eq!(
            solo_errori(&csv)[0].problem,
            Problem::EmptyField {
                field: "meaning".to_owned()
            }
        );
    }

    /// La regola vincolante: o tutto o niente.
    #[test]
    fn una_riga_sola_sbagliata_rifiuta_tutto_il_file() {
        let csv = format!(
            "{}\n日本語,japanese,,,,,,,,,\n猫,cat,,,,,,,,,\n,orphan,,,,,,,,,\n",
            columns().join(",")
        );
        // Due righe erano buonissime, e non serve a niente.
        assert!(read(&csv).is_err());
    }

    #[test]
    fn le_righe_vuote_si_saltano_invece_di_far_fallire() {
        let csv = format!("{}\n猫,cat,,,,,,,,,\n\n,,,,,,,,,,\n", columns().join(","));
        let righe = read(&csv).unwrap();
        assert_eq!(righe.len(), 1);
    }

    #[test]
    fn una_riga_a_cui_manca_solo_il_giapponese_non_e_una_riga_vuota() {
        let csv = format!("{}\n,cat,,,,,,,,,\n", columns().join(","));
        assert_eq!(solo_errori(&csv).len(), 1);
    }

    #[test]
    fn una_intestazione_sbagliata_si_ferma_subito() {
        let csv = "japanese,meaning\n猫,cat\n";
        let errori = solo_errori(csv);
        assert_eq!(errori.len(), 1);
        assert_eq!(errori[0].line, 1);
        assert_eq!(errori[0].problem, Problem::Header);
    }

    /// Senza il controllo, la prima carta verrebbe mangiata come intestazione.
    #[test]
    fn un_file_senza_intestazione_non_perde_la_prima_carta_in_silenzio() {
        let csv = "猫,cat,,,,,,,,,\n犬,dog,,,,,,,,,\n";
        assert_eq!(solo_errori(csv)[0].problem, Problem::Header);
    }

    #[test]
    fn l_intestazione_non_bada_a_maiuscole_e_spazi() {
        let csv = " Japanese , MEANING ,furigana,alt_1,alt_2,alt_3,alt_4,alt_5,alt_6,alt_7,alt_8\n猫,cat,,,,,,,,,\n";
        assert!(read(csv).is_ok());
    }

    /// Quasi sempre e' una virgola dentro un campo senza virgolette.
    #[test]
    fn una_riga_con_piu_colonne_dell_intestazione_si_rifiuta() {
        let csv = format!("{}\n猫,cat,,,,,,,,,,,\n", columns().join(","));
        assert_eq!(
            solo_errori(&csv)[0].problem,
            Problem::TooManyColumns {
                found: 13,
                expected: 11
            }
        );
    }

    #[test]
    fn una_riga_con_meno_colonne_va_benissimo() {
        let csv = format!("{}\n猫,cat\n", columns().join(","));
        let righe = read(&csv).unwrap();
        assert_eq!(righe[0].meaning, "cat");
        assert_eq!(righe[0].furigana, "");
    }

    /// Il numero di riga lo conta il lettore, non noi: con un a capo dentro un campo
    /// fra virgolette, contare le righe a mano sbaglierebbe.
    #[test]
    fn il_numero_di_riga_regge_un_a_capo_dentro_un_campo() {
        let csv = format!(
            "{}\n猫,\"soft\nanimal\",,,,,,,,,\n,orphan,,,,,,,,,\n",
            columns().join(",")
        );
        let errori = solo_errori(&csv);
        assert_eq!(errori.len(), 1);
        assert_eq!(errori[0].line, 4, "la riga buona ne occupa due");
    }

    /// Il BOM e' coperto **due volte e da due parti indipendenti**: sul frontend lo
    /// toglie il decodificatore, e qui lo toglie il lettore CSV. Verificato, perche'
    /// `trim()` non lo toglierebbe: `U+FEFF` non e' uno spazio.
    ///
    /// Vale la pena provarlo proprio perche' e' la forma in cui Excel salva il suo
    /// «CSV UTF-8», cioe' quella che arrivera' davvero.
    #[test]
    fn un_bom_non_da_fastidio_nemmeno_se_arriva_fin_qui() {
        let csv = format!("\u{feff}{}\n猫,cat,,,,,,,,,\n", columns().join(","));
        let righe = read(&csv).expect("il BOM non deve far fallire niente");
        assert_eq!(righe[0].japanese, "猫");
    }
}
