pub mod v2018;
pub mod v2020;
pub mod v2023_07;

use std::net::IpAddr;

use asic_rs_core::traits::{
    miner::{Miner, MinerConstructor, Validate},
    model::MinerModel,
};
use serde_json::Value;
use v2018::AntMinerV2018;
use v2020::AntMinerV2020;
use v2023_07::AntMinerV202307;

pub struct AntMiner;

fn blink_response_matches(response: &Value, fault: bool) -> bool {
    matches!(
        (fault, response.get("code").and_then(Value::as_str)),
        (true, Some("B000")) | (false, Some("B100"))
    )
}

fn use_legacy_backend(model: &impl MinerModel, version: Option<&semver::Version>) -> bool {
    AntMinerV2018::validate(version)
        && matches!(model.to_string().as_str(), "Z11j" | "Z15" | "Z15j")
}

impl MinerConstructor for AntMiner {
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::if_same_then_else)]
    fn new(ip: IpAddr, model: impl MinerModel, version: Option<semver::Version>) -> Box<dyn Miner> {
        if use_legacy_backend(&model, version.as_ref()) {
            Box::new(AntMinerV2018::new(ip, model))
        } else if AntMinerV2020::validate(version.as_ref()) {
            Box::new(AntMinerV2020::new(ip, model))
        } else if AntMinerV202307::validate(version.as_ref()) {
            Box::new(AntMinerV202307::new(ip, model))
        } else {
            Box::new(AntMinerV202307::new(ip, model))
        }
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::{
        data::{collector::DataField, command::MinerCommand},
        traits::miner::{MinerConstructor, Validate},
    };
    use semver::Version;

    use asic_rs_makes_antminer::models::AntMinerModel;

    use super::{
        AntMiner, AntMinerV2018, AntMinerV2020, blink_response_matches, use_legacy_backend,
    };
    use serde_json::json;

    #[test]
    fn routes_build_dates_before_2021_to_v2018() {
        for version in [
            Version::new(2017, 12, 31),
            Version::new(2018, 1, 1),
            Version::new(2019, 8, 26),
            Version::new(2020, 7, 3),
            Version::new(2020, 12, 14),
            Version::new(2020, 12, 31),
        ] {
            assert!(AntMinerV2018::validate(Some(&version)));
        }

        assert!(!AntMinerV2018::validate(Some(&Version::new(2021, 1, 1))));
        assert!(!AntMinerV2018::validate(None));
    }

    #[test]
    fn json_backend_accepts_all_routed_build_dates_through_june_2023() {
        assert!(AntMinerV2020::validate(Some(&Version::new(2020, 7, 3))));
        assert!(AntMinerV2020::validate(Some(&Version::new(2020, 12, 31))));
        assert!(AntMinerV2020::validate(Some(&Version::new(2021, 1, 1))));
        assert!(AntMinerV2020::validate(Some(&Version::new(2023, 6, 30))));
        assert!(!AntMinerV2020::validate(Some(&Version::new(2023, 7, 1))));
        assert!(!AntMinerV2020::validate(None));
    }

    #[test]
    fn only_verified_legacy_models_use_form_backend_on_2020_builds() {
        let version = Version::new(2020, 7, 3);
        for model in [AntMinerModel::Z11j, AntMinerModel::Z15, AntMinerModel::Z15j] {
            assert!(use_legacy_backend(&model, Some(&version)), "{model}");
            let miner = AntMiner::new([127, 0, 0, 1].into(), model, Some(version.clone()));
            assert!(matches!(
                miner.get_locations(DataField::Uptime)[0].0,
                MinerCommand::WebAPI { .. }
            ));
        }
        for model in [AntMinerModel::S19, AntMinerModel::S19Pro] {
            assert!(!use_legacy_backend(&model, Some(&version)), "{model}");
            let miner = AntMiner::new([127, 0, 0, 1].into(), model, Some(version.clone()));
            assert!(matches!(
                miner.get_locations(DataField::Uptime)[0].0,
                MinerCommand::RPC { .. }
            ));
        }
    }

    #[test]
    fn blink_response_must_match_requested_state() {
        assert!(blink_response_matches(&json!({"code": "B000"}), true));
        assert!(blink_response_matches(&json!({"code": "B100"}), false));
        assert!(!blink_response_matches(&json!({"code": "B100"}), true));
        assert!(!blink_response_matches(&json!({"code": "B000"}), false));
        assert!(!blink_response_matches(&json!({}), true));
    }
}
