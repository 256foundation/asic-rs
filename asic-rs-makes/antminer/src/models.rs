use std::str::FromStr;

use asic_rs_core::errors::ModelSelectionError;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize, Display)]
pub enum AntMinerModel {
    #[serde(alias = "ANTMINER D3")]
    D3,
    #[serde(alias = "ANTMINER HS3")]
    HS3,
    #[serde(alias = "ANTMINER L3+")]
    L3Plus,
    #[serde(alias = "ANTMINER KA3")]
    KA3,
    #[serde(alias = "ANTMINER KS3")]
    KS3,
    #[serde(alias = "ANTMINER DR5")]
    DR5,
    #[serde(alias = "ANTMINER KS5")]
    KS5,
    #[serde(alias = "ANTMINER KS5 PRO")]
    KS5Pro,
    #[serde(alias = "ANTMINER L7")]
    L7,
    #[serde(alias = "ANTMINER K7")]
    K7,
    #[serde(alias = "ANTMINER D7")]
    D7,
    #[serde(alias = "ANTMINER E9 PRO")]
    E9Pro,
    #[serde(alias = "ANTMINER D9")]
    D9,
    #[serde(alias = "ANTMINER S9")]
    S9,
    #[serde(alias = "ANTMINER S9I")]
    S9i,
    #[serde(alias = "ANTMINER S9J")]
    S9j,
    #[serde(alias = "ANTMINER T9")]
    T9,
    #[serde(alias = "ANTMINER L9")]
    L9,
    #[serde(alias = "ANTMINER Z15")]
    Z15,
    #[serde(alias = "ANTMINER Z15 PRO")]
    Z15Pro,
    #[serde(alias = "ANTMINER S17")]
    S17,
    #[serde(alias = "ANTMINER S17+")]
    S17Plus,
    #[serde(alias = "ANTMINER S17 PRO")]
    S17Pro,
    #[serde(alias = "ANTMINER S17E")]
    S17e,
    #[serde(alias = "ANTMINER T17")]
    T17,
    #[serde(alias = "ANTMINER T17+")]
    T17Plus,
    #[serde(alias = "ANTMINER T17E")]
    T17e,
    #[serde(alias = "ANTMINER S19")]
    S19,
    #[serde(alias = "ANTMINER S19L")]
    S19L,
    #[serde(alias = "ANTMINER S19 PRO")]
    S19Pro,
    #[serde(alias = "ANTMINER S19J")]
    S19j,
    #[serde(alias = "ANTMINER S19I")]
    S19i,
    #[serde(alias = "ANTMINER S19+")]
    S19Plus,
    #[serde(alias = "ANTMINER S19J88NOPIC")]
    S19jNoPIC,
    #[serde(alias = "ANTMINER S19PRO+")]
    S19ProPlus,
    #[serde(alias = "ANTMINER S19J PRO")]
    S19jPro,
    #[serde(alias = "ANTMINER S19J PRO+")]
    S19jProPlus,
    #[serde(alias = "ANTMINER S19 XP")]
    S19XP,
    #[serde(alias = "ANTMINER S19A")]
    S19a,
    #[serde(alias = "ANTMINER S19A PRO")]
    S19aPro,
    #[serde(alias = "ANTMINER S19 HYDRO")]
    S19Hydro,
    #[serde(alias = "ANTMINER S19 PRO HYD.")]
    S19ProHydro,
    #[serde(alias = "ANTMINER S19 PRO+ HYD.")]
    S19ProPlusHydro,
    #[serde(alias = "ANTMINER S19K PRO")]
    S19KPro,
    #[serde(alias = "ANTMINER S19J XP")]
    S19jXP,
    #[serde(alias = "ANTMINER T19")]
    T19,
    #[serde(alias = "ANTMINER S21")]
    #[serde(alias = "ANTMINER BHB68601")]
    #[serde(alias = "ANTMINER BHB68606")]
    S21,
    #[serde(alias = "ANTMINER S21 PRO")]
    S21Pro,
    #[serde(alias = "ANTMINER S21 XP")]
    S21XP,
    #[serde(alias = "ANTMINER S21+")]
    S21Plus,
    #[serde(alias = "ANTMINER S21 HYD.")]
    S21Hydro,
    #[serde(alias = "ANTMINER S21+ HYD.")]
    S21PlusHydro,
    #[serde(alias = "ANTMINER S21E XP HYD.")]
    S21eXPHydro,
    #[serde(alias = "ANTMINER T21")]
    T21,
    #[strum(to_string = "{0}")]
    Unknown(String),
}

impl FromStr for AntMinerModel {
    type Err = ModelSelectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .or_else(|_| Ok(Self::Unknown(s.to_string())))
    }
}

impl asic_rs_core::traits::model::MinerModel for AntMinerModel {
    fn make_name(&self) -> String {
        "Antminer".to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn known_model_parses() {
        // Act
        let result = AntMinerModel::from_str("ANTMINER S21").unwrap();

        // Assert
        assert_eq!(result, AntMinerModel::S21);
    }

    #[test]
    fn unknown_model_falls_back() {
        // Act
        let result = AntMinerModel::from_str("ANTMINER S99").unwrap();

        // Assert
        assert_eq!(result, AntMinerModel::Unknown("ANTMINER S99".to_string()));
    }
}
