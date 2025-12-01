#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};
use strum::Display;

#[cfg_attr(feature = "python", pyclass(str, module = "asic_rs"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
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
}
