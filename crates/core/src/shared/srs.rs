//! Pianificazione delle ripetizioni con FSRS.
//!
//! Il calcolo di quando rivedere una carta vive qui e non nella UI: il frontend si
//! limita a comunicare com'e' andata la risposta.
//!
//! # Perche' il crate `fsrs` e non `rs-fsrs`
//!
//! `fsrs` e' l'implementazione ufficiale del progetto open-spaced-repetition, e' alla
//! versione 6 dell'algoritmo, ha una licenza dichiarata e comprende anche
//! l'ottimizzatore, che un domani permette di ricalcolare i parametri sullo storico
//! di questa persona invece di tenere quelli medi. Fino alla versione 5 si portava
//! dietro `burn`, un framework di calcolo pesante; dalla 6 `burn` e' rimasto solo tra
//! le dipendenze di sviluppo, quindi il peso e' paragonabile all'alternativa.
//!
//! # Il modello in due numeri
//!
//! FSRS riassume la memoria in [`MemoryState`]: la **stabilita'**, cioe' quanti giorni
//! il ricordo regge, e la **difficolta'**, cioe' quanto quell'elemento e' faticoso per
//! questa persona. A ogni risposta i due numeri si aggiornano e da lì esce la prossima
//! scadenza.

use chrono::{DateTime, TimeDelta, Utc};
use fsrs::{DEFAULT_PARAMETERS, FSRS};
use serde::{Deserialize, Serialize};

use crate::shared::error::{CoreError, Result};

/// Quanto vogliamo ricordare al momento del ripasso.
///
/// Piu' e' alto, piu' i ripassi sono fitti. `0.9` e' il valore consigliato da FSRS:
/// nove volte su dieci il segno viene ricordato.
pub const DEFAULT_RETENTION: f32 = 0.9;

/// Com'e' andata una risposta, nella scala a quattro gradini di FSRS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    /// Non ricordata.
    Again = 1,
    /// Ricordata a fatica.
    Hard = 2,
    /// Ricordata.
    Good = 3,
    /// Ricordata subito e senza sforzo.
    Easy = 4,
}

impl Grade {
    /// Traduce l'esito grezzo di un esercizio nella scala di FSRS.
    ///
    /// Gli esercizi sui kana producono solo giusto o sbagliato, quindi qui si usano
    /// due gradini su quattro. Gli altri due restano disponibili per quando la UI
    /// permettera' di dire "l'ho ricordata a fatica".
    pub fn from_correct(correct: bool) -> Self {
        if correct { Self::Good } else { Self::Again }
    }

    pub fn as_i64(self) -> i64 {
        self as i64
    }

    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Again),
            2 => Some(Self::Hard),
            3 => Some(Self::Good),
            4 => Some(Self::Easy),
            _ => None,
        }
    }
}

/// Lo stato di memoria di una carta secondo FSRS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryState {
    /// Per quanti giorni il ricordo regge.
    pub stability: f32,
    /// Quanto l'elemento e' faticoso per questa persona.
    pub difficulty: f32,
}

/// Il risultato di una pianificazione.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduled {
    pub memory: MemoryState,
    /// Quando ripresentare la carta.
    pub due_at: DateTime<Utc>,
    /// L'intervallo scelto, in giorni. Puo' essere minore di uno: una carta sbagliata
    /// torna dopo pochi minuti, non il giorno dopo.
    pub interval_days: f32,
}

/// Decide quando ripresentare una carta.
#[derive(Debug, Clone)]
pub struct Scheduler {
    fsrs: FSRS,
    desired_retention: f32,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION).expect("i parametri di default di FSRS sono validi")
    }
}

impl Scheduler {
    /// Costruisce uno scheduler con i parametri medi di FSRS.
    ///
    /// I parametri personalizzati, ricavati dallo storico delle risposte, sono un
    /// passo successivo: qui si parte da quelli che valgono per tutti.
    pub fn new(desired_retention: f32) -> Result<Self> {
        let fsrs = FSRS::new(&DEFAULT_PARAMETERS).map_err(|e| CoreError::Scheduling {
            message: format!("{e:?}"),
        })?;

        Ok(Self {
            fsrs,
            desired_retention,
        })
    }

    /// Calcola il nuovo stato di una carta dopo una risposta.
    ///
    /// `current` e `last_reviewed_at` sono `None` la prima volta che l'elemento viene
    /// studiato.
    pub fn schedule(
        &self,
        current: Option<MemoryState>,
        last_reviewed_at: Option<DateTime<Utc>>,
        grade: Grade,
        now: DateTime<Utc>,
    ) -> Result<Scheduled> {
        // FSRS ragiona in giorni interi trascorsi dall'ultimo ripasso. Piu' di un
        // ripasso nello stesso giorno da' zero, che e' proprio quello che serve.
        let days_elapsed = last_reviewed_at
            .map(|last| (now - last).num_days().max(0) as u32)
            .unwrap_or(0);

        let states = self
            .fsrs
            .next_states(
                current.map(Into::into),
                self.desired_retention,
                days_elapsed,
            )
            .map_err(|e| CoreError::Scheduling {
                message: format!("{e:?}"),
            })?;

        let next = match grade {
            Grade::Again => states.again,
            Grade::Hard => states.hard,
            Grade::Good => states.good,
            Grade::Easy => states.easy,
        };

        // L'intervallo arriva in giorni con la virgola. Convertirlo in secondi invece
        // di arrotondarlo ai giorni conserva i ripassi ravvicinati di una carta appena
        // sbagliata.
        let seconds = f64::from(next.interval) * 86_400.0;

        Ok(Scheduled {
            memory: next.memory.into(),
            due_at: now + TimeDelta::milliseconds((seconds * 1_000.0) as i64),
            interval_days: next.interval,
        })
    }
}

impl From<MemoryState> for fsrs::MemoryState {
    fn from(s: MemoryState) -> Self {
        Self {
            stability: s.stability,
            difficulty: s.difficulty,
        }
    }
}

impl From<fsrs::MemoryState> for MemoryState {
    fn from(s: fsrs::MemoryState) -> Self {
        Self {
            stability: s.stability,
            difficulty: s.difficulty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> Scheduler {
        Scheduler::default()
    }

    #[test]
    fn una_carta_mai_studiata_riceve_uno_stato_iniziale() {
        let now = Utc::now();
        let s = scheduler().schedule(None, None, Grade::Good, now).unwrap();

        assert!(s.memory.stability > 0.0);
        assert!(s.memory.difficulty > 0.0);
        assert!(s.due_at > now, "il ripasso deve cadere nel futuro");
    }

    #[test]
    fn la_valutazione_decide_quanto_aspettare() {
        let now = Utc::now();
        let s = scheduler();
        let intervallo = |g| s.schedule(None, None, g, now).unwrap().interval_days;

        let again = intervallo(Grade::Again);
        let hard = intervallo(Grade::Hard);
        let good = intervallo(Grade::Good);
        let easy = intervallo(Grade::Easy);

        assert!(
            again < hard && hard < good && good < easy,
            "gli intervalli devono crescere con la valutazione: {again} {hard} {good} {easy}"
        );
    }

    #[test]
    fn una_carta_non_ricordata_torna_nel_giro_di_ore() {
        let now = Utc::now();
        let s = scheduler().schedule(None, None, Grade::Again, now).unwrap();

        // Meno di un giorno: e' il motivo per cui l'intervallo non viene arrotondato
        // ai giorni interi.
        assert!(
            s.interval_days < 1.0,
            "intervallo troppo lungo per una carta sbagliata: {} giorni",
            s.interval_days
        );
        assert!(s.due_at > now);
    }

    #[test]
    fn ricordare_di_nuovo_allontana_il_ripasso() {
        let s = scheduler();
        let t0 = Utc::now();

        let primo = s.schedule(None, None, Grade::Good, t0).unwrap();
        let t1 = primo.due_at;
        let secondo = s
            .schedule(Some(primo.memory), Some(t0), Grade::Good, t1)
            .unwrap();

        assert!(secondo.memory.stability > primo.memory.stability);
        assert!(secondo.interval_days > primo.interval_days);
    }

    #[test]
    fn volere_ricordare_di_piu_infittisce_i_ripassi() {
        let t0 = Utc::now();
        let rilassato = Scheduler::new(0.8).unwrap();
        let esigente = Scheduler::new(0.95).unwrap();

        let a = rilassato.schedule(None, None, Grade::Good, t0).unwrap();
        let b = esigente.schedule(None, None, Grade::Good, t0).unwrap();

        assert!(
            b.interval_days < a.interval_days,
            "chiedere piu' memoria deve accorciare l'attesa: {} contro {}",
            b.interval_days,
            a.interval_days
        );
    }

    #[test]
    fn un_esito_binario_usa_due_gradini_su_quattro() {
        assert_eq!(Grade::from_correct(true), Grade::Good);
        assert_eq!(Grade::from_correct(false), Grade::Again);
    }

    #[test]
    fn la_valutazione_sopravvive_al_passaggio_dal_database() {
        for g in [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy] {
            assert_eq!(Grade::from_i64(g.as_i64()), Some(g));
        }
        // Un valore che non conosciamo non viene inventato.
        assert_eq!(Grade::from_i64(0), None);
        assert_eq!(Grade::from_i64(5), None);
    }
}
