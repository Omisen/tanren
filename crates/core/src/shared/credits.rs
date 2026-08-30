//! Chi ha fatto cosa, e sotto quale licenza lo ridistribuiamo.
//!
//! # Perche' sta nel core e non nell'interfaccia
//!
//! Perche' una parte non e' testo fisso: l'edizione dei dati che l'app spedisce la sa
//! solo il dato stesso, e scriverla a mano di la' vorrebbe dire vederla divergere alla
//! prima rigenerazione. Il campo `source` dentro le tabelle esiste anche per questo.
//!
//! # Perche' non basta ringraziare
//!
//! La CC BY-SA e' **ShareAlike**: obbliga a dichiarare sotto quale licenza si
//! ridistribuisce, non solo da chi si e' preso. Ed e' un obbligo che vive **nel mezzo
//! in cui l'opera viaggia**, cioe' dentro l'APK: il README della repo non lo assolve,
//! perche' chi installa l'app il README non lo vede.

use serde::Serialize;

/// Una fonte, con quello che serve a darle credito.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub name: String,
    /// Che cosa di quello che si vede nell'app viene da qui.
    pub covers: String,
    /// La frase esatta che quella fonte chiede di riportare, dove la chiede.
    ///
    /// Non e' parafrasabile: se una licenza detta le parole, sono quelle.
    pub notice: Option<String>,
    pub licence: String,
    pub licence_url: String,
    /// Il testo della licenza imbarcato nell'app, se c'e'.
    ///
    /// `None` quando si puo' offrire solo il link. Le due cose non sono equivalenti ma
    /// sono entrambe ammesse: la licenza dell'EDRDG dice che dove il pacchetto non
    /// permette di includere i file basta rimandarci.
    pub licence_file: Option<String>,
    pub source_url: Option<String>,
    /// Quale edizione esatta di quella fonte l'app sta spedendo.
    pub edition: Option<String>,
}

/// Il font e la licenza del progetto stesso.
///
/// I dati di studio non stanno qui: li dichiara la materia che li usa, perche' e'
/// l'unica a sapere da quale edizione vengono.
pub fn app() -> Vec<Credit> {
    vec![
        Credit {
            name: "M PLUS Rounded 1c".to_owned(),
            covers: "The Japanese typeface, subset to the characters the app shows".to_owned(),
            notice: None,
            licence: "SIL Open Font License 1.1".to_owned(),
            licence_url: "https://openfontlicense.org".to_owned(),
            licence_file: Some("/fonts/OFL.txt".to_owned()),
            source_url: Some("https://github.com/google/fonts/tree/main/ofl/mplusrounded1c".to_owned()),
            edition: None,
        },
        Credit {
            name: "Tanren".to_owned(),
            // La meta' che quasi tutti dimenticano: lo ShareAlike obbliga a dire sotto
            // quale licenza **tu** ridistribuisci, non solo da chi hai preso.
            covers: "The app itself. Code and data are separate works, and the licences \
                     differ: the share-alike travels with the data, not with the code."
                .to_owned(),
            notice: Some(
                "Code: MIT, © 2026 Omisen. Kanji data: CC BY-SA 4.0, because it is \
                 derived from sources under that licence. The level ordering is computed \
                 from their composition data and is a derived work, so it is CC BY-SA 4.0 \
                 too."
                    .to_owned(),
            ),
            licence: "MIT for the code, CC BY-SA 4.0 for the kanji data".to_owned(),
            licence_url: "https://github.com/Omisen/tanren/blob/main/LICENSE".to_owned(),
            licence_file: Some("/licences/CC-BY-SA-4.0.txt".to_owned()),
            source_url: Some("https://github.com/Omisen/tanren".to_owned()),
            edition: None,
        },
    ]
}
