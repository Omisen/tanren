//! Normalizzazione del testo giapponese.
//!
//! La validazione di una risposta confronta letture in kana, mai romaji. Prima del
//! confronto l'input dell'utente e la lettura attesa passano entrambi da qui, cosi'
//! due scritture equivalenti della stessa lettura risultano uguali.
//!
//! # Cosa viene uniformato
//!
//! - **Katakana verso hiragana.** カタカナ e かたかな sono la stessa lettura: la
//!   forma canonica scelta e' l'hiragana.
//! - **Larghezza piena verso larghezza normale**, e katakana a mezza larghezza verso
//!   la forma piena. Un IME puo' produrre `Ａ` invece di `A` e `ﾆﾎﾝ` invece di
//!   ニホン, a seconda della modalita' in cui si trova.
//! - **Segni di sonorizzazione staccati.** `か` seguito dal segno combinante `゙`
//!   diventa `が`, che e' come l'utente lo intendeva.
//! - **Spazi.** Vengono tolti tutti, non solo quelli ai bordi: dentro una lettura
//!   sono sempre rumore, e alcuni IME lasciano uno spazio dopo la conversione.
//!
//! # Due livelli di normalizzazione
//!
//! [`normalize_input`] fa la pulizia Unicode ma **non tocca il sillabario**: serve
//! quando la domanda e' scritta in un sillabario preciso, come nell'esercizio in cui
//! si chiede di scrivere un segno in katakana. Se ripiegasse tutto sull'hiragana,
//! rispondere か a una domanda su カ risulterebbe corretto.
//!
//! [`normalize_reading`] aggiunge la conversione verso l'hiragana: serve quando conta
//! la lettura e non la grafia, come per le letture di un kanji.
//!
//! # Cosa viene lasciato stare
//!
//! - Il prolungamento `ー`, che appartiene alla lettura e distingue `らーめん` da
//!   `らめん`.
//! - I kana piccoli: `きょう` e `きよう` sono letture diverse.
//! - Il punto mediano `・` e la punteggiatura in generale.

use unicode_normalization::UnicodeNormalization;

/// Riduce una lettura alla sua forma canonica, pronta per essere confrontata.
///
/// La funzione e' idempotente: applicarla a un risultato gia' normalizzato non lo
/// cambia.
///
/// ```
/// use tanren_core::shared::text::normalize_reading;
///
/// assert_eq!(normalize_reading("ニホン"), "にほん");
/// assert_eq!(normalize_reading(" ﾆﾎﾝ "), "にほん");
/// ```
pub fn normalize_reading(input: &str) -> String {
    normalize(input).map(to_hiragana).collect()
}

/// Ripulisce il testo lasciando intatto il sillabario in cui e' scritto.
///
/// Fa la stessa pulizia di [`normalize_reading`] ma senza la conversione verso
/// l'hiragana, quindi `カ` resta `カ` e non diventa `か`.
///
/// ```
/// use tanren_core::shared::text::normalize_input;
///
/// assert_eq!(normalize_input(" ﾆﾎﾝ "), "ニホン");
/// assert_ne!(normalize_input("カ"), normalize_input("か"));
/// ```
pub fn normalize_input(input: &str) -> String {
    normalize(input).collect()
}

/// La parte comune: NFKC e via gli spazi.
///
/// NFKC fa tre cose in un colpo solo: porta la larghezza piena a quella normale,
/// ricompone i katakana a mezza larghezza con il loro segno di sonorizzazione, e
/// unisce i segni combinanti alla sillaba che li precede.
fn normalize(input: &str) -> impl Iterator<Item = char> + '_ {
    input.nfkc().filter(|c| !c.is_whitespace())
}

/// Porta un singolo carattere katakana al corrispondente hiragana.
///
/// I due sillabari sono allineati nella tabella Unicode a distanza fissa, quindi la
/// conversione e' una sottrazione. Vale pero' solo dove la corrispondenza esiste
/// davvero, e i limiti vanno rispettati alla lettera.
fn to_hiragana(c: char) -> char {
    /// Distanza tra un katakana e il suo hiragana nella tabella Unicode.
    const SHIFT: u32 = 0x60;

    match c {
        // Da ァ a ヶ ogni katakana ha il suo hiragana, ゔ e ゖ compresi.
        // Sopra ヶ ci sono ヷヸヹヺ, che un hiragana non ce l'hanno: restano come
        // sono. Subito dopo viene ・, che e' punteggiatura, e ー, che appartiene
        // alla lettura: nessuno dei due va toccato.
        'ァ'..='ヶ' => shift_down(c, SHIFT),
        // I segni di iterazione ヽ e ヾ hanno invece il loro equivalente ゝ e ゞ.
        'ヽ'..='ヾ' => shift_down(c, SHIFT),
        _ => c,
    }
}

/// Sposta un carattere indietro di `amount` posizioni nella tabella Unicode.
///
/// Se il risultato non fosse un carattere valido restituisce l'originale, cosi' un
/// errore nei limiti degrada in un mancato confronto invece che in un panico.
fn shift_down(c: char, amount: u32) -> char {
    char::from_u32(c as u32 - amount).unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converte_i_katakana_in_hiragana() {
        assert_eq!(normalize_reading("カタカナ"), "かたかな");
        assert_eq!(normalize_reading("ニホンゴ"), "にほんご");
    }

    #[test]
    fn lascia_intatto_cio_che_e_gia_hiragana() {
        assert_eq!(normalize_reading("にほんご"), "にほんご");
    }

    #[test]
    fn tiene_distinti_i_kana_piccoli() {
        // きょう e きよう sono letture diverse: la normalizzazione non deve
        // appiattirle.
        assert_ne!(normalize_reading("キョウ"), normalize_reading("キヨウ"));
        assert_eq!(normalize_reading("キョウ"), "きょう");
    }

    #[test]
    fn conserva_il_prolungamento() {
        assert_eq!(normalize_reading("ラーメン"), "らーめん");
        assert_ne!(normalize_reading("ラーメン"), normalize_reading("ラメン"));
    }

    #[test]
    fn converte_i_katakana_a_mezza_larghezza() {
        assert_eq!(normalize_reading("ﾆﾎﾝ"), "にほん");
        assert_eq!(normalize_reading("ﾃﾞｰﾀ"), "でーた");
    }

    #[test]
    fn ricompone_i_segni_di_sonorizzazione_staccati() {
        // か seguito dal segno combinante di sonorizzazione vale が.
        assert_eq!(normalize_reading("か\u{3099}"), "が");
        // Stessa cosa per il semi sonoro: は piu' ゚ vale ぱ.
        assert_eq!(normalize_reading("は\u{309A}"), "ぱ");
    }

    #[test]
    fn porta_la_larghezza_piena_a_quella_normale() {
        assert_eq!(normalize_reading("ＡＢＣ１２３"), "ABC123");
    }

    #[test]
    fn toglie_gli_spazi_ovunque() {
        assert_eq!(normalize_reading("  にほん  "), "にほん");
        assert_eq!(normalize_reading("に ほん"), "にほん");
        // Spazio ideografico a larghezza piena.
        assert_eq!(normalize_reading("に\u{3000}ほん"), "にほん");
        assert_eq!(normalize_reading("\tにほん\n"), "にほん");
    }

    #[test]
    fn non_tocca_la_punteggiatura_al_confine_del_sillabario() {
        // ・ sta appena dopo i katakana convertibili: se il limite fosse sbagliato
        // diventerebbe un carattere qualunque.
        assert_eq!(normalize_reading("ア・イ"), "あ・い");
    }

    #[test]
    fn converte_i_kana_ai_bordi_dell_intervallo() {
        assert_eq!(normalize_reading("ァ"), "ぁ");
        assert_eq!(normalize_reading("ヶ"), "ゖ");
        assert_eq!(normalize_reading("ヴ"), "ゔ");
    }

    #[test]
    fn lascia_stare_i_katakana_senza_equivalente() {
        // ヷ ヸ ヹ ヺ non hanno una forma hiragana: vanno lasciati come sono.
        assert_eq!(normalize_reading("ヷ"), "ヷ");
        assert_eq!(normalize_reading("ヺ"), "ヺ");
    }

    #[test]
    fn converte_i_segni_di_iterazione() {
        assert_eq!(normalize_reading("ヽ"), "ゝ");
        assert_eq!(normalize_reading("ヾ"), "ゞ");
    }

    #[test]
    fn regge_i_casi_limite() {
        assert_eq!(normalize_reading(""), "");
        assert_eq!(normalize_reading("   "), "");
    }

    #[test]
    fn la_normalizzazione_dell_input_non_tocca_il_sillabario() {
        // La domanda "scrivi カ" non deve accettare か.
        assert_eq!(normalize_input("カ"), "カ");
        assert_ne!(normalize_input("カ"), normalize_input("か"));
        // La pulizia Unicode pero' la fa lo stesso.
        assert_eq!(normalize_input(" ﾆﾎﾝ "), "ニホン");
        assert_eq!(normalize_input("ｶ\u{FF9E}"), "ガ");
    }

    #[test]
    fn e_idempotente() {
        for input in ["ﾆﾎﾝ", "カタカナ", " ラーメン ", "か\u{3099}", "ＡＢＣ"] {
            let once = normalize_reading(input);
            assert_eq!(once, normalize_reading(&once), "reading: {input}");

            let once = normalize_input(input);
            assert_eq!(once, normalize_input(&once), "input: {input}");
        }
    }
}
