//! I passi brevi con cui una carta nuova entra in circolo.
//!
//! # Perche' non basta FSRS
//!
//! Perche' FSRS non li produce, e non e' un difetto suo: misurato sullo scheduler
//! vero, una carta mai vista sbagliata torna dopo **305 minuti** e una risposta Good
//! la manda a 2,31 giorni. Sono intervalli giusti per una carta che si sta
//! consolidando e sbagliati per una che si sta conoscendo adesso, dove il ricordo si
//! costruisce a minuti. E' lo stesso motivo per cui i kana FSRS non lo usano affatto.
//!
//! Gli step brevi sono quindi uno **strato sopra** l'algoritmo, come in Anki: finche'
//! la carta e' in apprendimento comandano loro, dopo comanda il motore. Lo stato di
//! memoria pero' si aggiorna **fin dalla prima risposta**, cosi' quando la carta
//! gradua la stabilita' e' gia' quella vera e non un numero inventato al volo.
//!
//! # Quando una carta e' ancora in apprendimento
//!
//! Finche' **non e' mai stata indovinata**. Non serve una colonna nuova ne' uno stato
//! esplicito del ciclo di vita: `cards` porta gia' `reps` e `lapses`, e la differenza
//! fra i due e' quante volte quella carta e' stata azzeccata. A zero si e' ancora
//! nella fase in cui il ricordo si forma.
//!
//! # Perche' i passi sono due e non tre
//!
//! Perche' due sono i momenti che contano: **sbagliare**, e **indovinare mentre si sta
//! ancora imparando**. `Hard` e `Good` durante l'apprendimento tornano insieme, e la
//! differenza fra i due non va persa: arriva a FSRS come voto e si vede dopo, negli
//! intervalli che il motore calcolera'. `Easy` invece gradua subito, perche' dire
//! «questa la so» e poi rivederla fra dieci minuti sarebbe non aver ascoltato.

use std::ops::RangeInclusive;

use chrono::{DateTime, TimeDelta, Utc};

use crate::shared::error::{CoreError, Result};
use crate::shared::srs::Grade;
use crate::shared::storage::{Card, Database};

/// Quanto aspettare, in minuti, dopo una risposta sbagliata.
pub const AGAIN_RANGE: RangeInclusive<i64> = 1..=30;
/// Quanto aspettare, in minuti, dopo una risposta giusta su una carta nuova.
pub const GOOD_RANGE: RangeInclusive<i64> = 1..=120;

const AGAIN_KEY: &str = "flashcard.step_again";
const GOOD_KEY: &str = "flashcard.step_good";

/// Il default dopo un errore: la carta deve tornare dentro questa sessione.
pub const AGAIN_DEFAULT: i64 = 1;
/// Il default dopo la prima risposta giusta.
pub const GOOD_DEFAULT: i64 = 10;

/// I due passi brevi, in minuti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Steps {
    pub again: i64,
    pub good: i64,
}

impl Default for Steps {
    fn default() -> Self {
        Self {
            again: AGAIN_DEFAULT,
            good: GOOD_DEFAULT,
        }
    }
}

impl Steps {
    /// Quando ripresentare la carta.
    ///
    /// `fsrs` e' quello che direbbe il motore. Lo si sostituisce **solo dove il motore
    /// e' fuori scala**, cioe' dopo un errore e mentre la carta e' ancora in
    /// apprendimento: in tutti gli altri casi decide lui, ed e' il punto.
    pub fn due_at(
        &self,
        grade: Grade,
        learning: bool,
        fsrs: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> DateTime<Utc> {
        match grade {
            // Vale anche su una carta matura che si e' persa: rivederla adesso e'
            // proprio quello che serve, e la stabilita' l'ha gia' pagata.
            Grade::Again => now + TimeDelta::minutes(self.again),
            Grade::Hard | Grade::Good if learning => now + TimeDelta::minutes(self.good),
            _ => fsrs,
        }
    }
}

/// Se una carta e' ancora in apprendimento, cioe' se non e' mai stata indovinata.
///
/// Una carta che non esiste ancora non e' mai stata nemmeno vista, quindi si'.
pub fn learning(card: Option<&Card>) -> bool {
    card.is_none_or(|c| c.reps - c.lapses == 0)
}

/// I passi come sono configurati adesso, non come sono nati.
///
/// Una preferenza scritta male vale come **assente** e si ricade sul default, per la
/// stessa ragione del ritmo dei kanji: un valore fuori posto non deve impedire di
/// aprire l'app, e fuori intervallo ci si puo' finire anche solo stringendo un limite
/// in una versione futura.
pub async fn steps(db: &Database) -> Result<Steps> {
    let mut steps = Steps::default();

    if let Some(grezzo) = db.setting(AGAIN_KEY).await?
        && let Ok(n) = grezzo.parse::<i64>()
        && AGAIN_RANGE.contains(&n)
    {
        steps.again = n;
    }

    if let Some(grezzo) = db.setting(GOOD_KEY).await?
        && let Ok(n) = grezzo.parse::<i64>()
        && GOOD_RANGE.contains(&n)
    {
        steps.good = n;
    }

    Ok(steps)
}

/// Cambia il passo dopo un errore.
pub async fn set_again(db: &Database, value: i64, now: DateTime<Utc>) -> Result<()> {
    write(db, AGAIN_KEY, AGAIN_RANGE, value, now).await
}

/// Cambia il passo dopo la prima risposta giusta.
pub async fn set_good(db: &Database, value: i64, now: DateTime<Utc>) -> Result<()> {
    write(db, GOOD_KEY, GOOD_RANGE, value, now).await
}

async fn write(
    db: &Database,
    key: &str,
    range: RangeInclusive<i64>,
    value: i64,
    now: DateTime<Utc>,
) -> Result<()> {
    if !range.contains(&value) {
        return Err(CoreError::SettingOutOfRange {
            setting: key.to_owned(),
            value,
            min: *range.start(),
            max: *range.end(),
        });
    }

    db.set_setting(key, &value.to_string(), now).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adesso() -> DateTime<Utc> {
        "2026-03-15T08:00:00Z".parse().expect("istante valido")
    }

    fn fra_giorni(n: i64) -> DateTime<Utc> {
        adesso() + TimeDelta::days(n)
    }

    #[test]
    fn sbagliare_riporta_la_carta_dentro_la_sessione() {
        let s = Steps::default();
        // Anche su una carta consolidata, dove il motore direbbe mezza giornata.
        for learning in [true, false] {
            assert_eq!(
                s.due_at(Grade::Again, learning, fra_giorni(1), adesso()),
                adesso() + TimeDelta::minutes(1)
            );
        }
    }

    #[test]
    fn mentre_si_impara_comandano_i_passi_brevi() {
        let s = Steps::default();
        for g in [Grade::Hard, Grade::Good] {
            assert_eq!(
                s.due_at(g, true, fra_giorni(2), adesso()),
                adesso() + TimeDelta::minutes(10),
                "il motore direbbe giorni, e su una carta nuova e' fuori scala"
            );
        }
    }

    #[test]
    fn dire_che_e_facile_gradua_subito() {
        let s = Steps::default();
        assert_eq!(
            s.due_at(Grade::Easy, true, fra_giorni(8), adesso()),
            fra_giorni(8),
            "decide il motore anche se la carta e' nuova"
        );
    }

    #[test]
    fn dopo_l_apprendimento_decide_il_motore() {
        let s = Steps::default();
        for g in [Grade::Hard, Grade::Good, Grade::Easy] {
            assert_eq!(s.due_at(g, false, fra_giorni(5), adesso()), fra_giorni(5));
        }
    }

    #[tokio::test]
    async fn senza_preferenze_valgono_i_default() {
        let db = Database::in_memory().await.unwrap();
        assert_eq!(steps(&db).await.unwrap(), Steps::default());
    }

    #[tokio::test]
    async fn una_preferenza_scritta_vale_piu_del_default() {
        let db = Database::in_memory().await.unwrap();
        set_again(&db, 5, adesso()).await.unwrap();
        set_good(&db, 20, adesso()).await.unwrap();

        assert_eq!(steps(&db).await.unwrap(), Steps { again: 5, good: 20 });
    }

    #[tokio::test]
    async fn un_valore_fuori_scala_non_entra() {
        let db = Database::in_memory().await.unwrap();
        assert!(matches!(
            set_again(&db, 0, adesso()).await,
            Err(CoreError::SettingOutOfRange { .. })
        ));
        assert!(matches!(
            set_good(&db, 1_000, adesso()).await,
            Err(CoreError::SettingOutOfRange { .. })
        ));
        assert_eq!(steps(&db).await.unwrap(), Steps::default());
    }

    #[tokio::test]
    async fn una_preferenza_illeggibile_vale_come_assente() {
        let db = Database::in_memory().await.unwrap();
        db.set_setting(AGAIN_KEY, "domani", adesso()).await.unwrap();
        assert_eq!(steps(&db).await.unwrap(), Steps::default());
    }
}
