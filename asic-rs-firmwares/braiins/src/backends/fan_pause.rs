//! Pause cooldown fan mode for the REST Braiins OS backends (25.07 and newer).
//!
//! The REST API has no way to change only the pause mode: `PUT cooling/mode`
//! takes a whole cooling mode, and switching modes resets every field that is
//! not sent. So the current cooling configuration is read first and sent back
//! unchanged, apart from `pause_mode`.

use asic_rs_core::{
    config::fan::{FanPause, FanPauseRuntime},
    traits::miner::WebAPIClient,
};
use reqwest::Method;
use semver::Version;
use serde_json::{Value, json};

use super::util::parse_bos_version;

/// First Braiins OS release with the pause cooldown fan mode.
const MIN_VERSION: Version = Version::new(26, 1, 0);
/// First Braiins OS release whose API exposes the pause runtime
/// (`fan_pause_runtime`).
const RUNTIME_VERSION: Version = Version::new(26, 4, 0);

/// `FAN_PAUSE_RUNTIME_*` values of `ManualPauseMode.fan_pause_runtime`.
const FAN_PAUSE_RUNTIME_LIMITED: i32 = 1;
const FAN_PAUSE_RUNTIME_INDEFINITE: i32 = 2;

/// Cooling mode keys accepted by `PUT cooling/mode`.
const SETTABLE_MODES: [&str; 4] = ["auto", "manual", "immersion", "hydro"];

pub(crate) async fn set_fan_pause(
    web: &impl WebAPIClient,
    pause: FanPause,
) -> anyhow::Result<bool> {
    pause.validate()?;

    let details = web
        .send_command("miner/details", false, None, Method::GET)
        .await?;
    let version = details
        .pointer("/bos_version/current")
        .and_then(Value::as_str)
        .and_then(parse_bos_version)
        .ok_or_else(|| {
            anyhow::anyhow!("Could not determine the Braiins OS version; the pause cooldown fan mode requires Braiins OS 26.01 or later")
        })?;
    if version < MIN_VERSION {
        anyhow::bail!(
            "The pause cooldown fan mode requires Braiins OS 26.01 or later (found {}.{:02})",
            version.major,
            version.minor
        );
    }

    let pause_mode = pause_mode_json(web, pause, &version).await?;

    let configuration = web
        .send_command("configuration/miner", false, None, Method::GET)
        .await?;
    let (mode_key, mut mode) = current_cooling_mode(&configuration)?;
    mode.as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Cooling mode '{mode_key}' is not an object"))?
        .insert("pause_mode".to_string(), pause_mode);

    web.send_command(
        "cooling/mode",
        true,
        Some(json!({ mode_key: mode })),
        Method::PUT,
    )
    .await?;
    Ok(true)
}

async fn pause_mode_json(
    web: &impl WebAPIClient,
    pause: FanPause,
    version: &Version,
) -> anyhow::Result<Value> {
    let FanPause::Manual { fan_speed, runtime } = pause else {
        return Ok(json!({ "mode": { "auto": {} } }));
    };

    if *version < RUNTIME_VERSION {
        anyhow::bail!(
            "A manual pause cooldown needs the pause runtime, which requires Braiins OS 26.04 or later (found {}.{:02})",
            version.major,
            version.minor
        );
    }

    let runtime_code = match runtime {
        FanPauseRuntime::Indefinite => FAN_PAUSE_RUNTIME_INDEFINITE,
        FanPauseRuntime::Limited(requested) => {
            // The API only exposes a firmware constant for the limited runtime,
            // so a different duration cannot be honoured.
            let constraints = web
                .send_command("configuration/constraints", false, None, Method::GET)
                .await?;
            let firmware_secs = constraints
                .pointer("/cooling_constraints/fan_pause_runtime_limited_duration_s")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "The miner does not report its limited pause runtime, so the requested duration cannot be verified"
                    )
                })?;
            if requested.as_secs_f64() != firmware_secs as f64 {
                anyhow::bail!(
                    "The limited pause runtime is fixed by the firmware at {firmware_secs}s and cannot be set to {}s",
                    requested.as_secs_f64()
                );
            }
            FAN_PAUSE_RUNTIME_LIMITED
        }
    };

    Ok(json!({
        "mode": { "manual": {
            "pause_cooldown_fan_speed_ratio": fan_speed,
            "fan_pause_runtime": runtime_code,
        } }
    }))
}

/// Extract the active cooling mode (`{"auto": {..}}` etc.) from the miner
/// configuration, as a key and its settings.
fn current_cooling_mode(configuration: &Value) -> anyhow::Result<(&'static str, Value)> {
    let mode = configuration
        .pointer("/temperature/mode")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("The miner did not report its cooling mode"))?;
    for key in SETTABLE_MODES {
        if let Some(settings) = mode.get(key) {
            return Ok((key, settings.clone()));
        }
    }
    anyhow::bail!(
        "The current cooling mode ({}) cannot be re-applied to change the pause mode",
        mode.keys().cloned().collect::<Vec<_>>().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::Duration};

    use asic_rs_core::{
        data::command::MinerCommand,
        traits::miner::{APIClient, WebAPIClient},
    };
    use async_trait::async_trait;

    use super::*;

    /// In-memory stand-in for the miner web API; records every request.
    struct FakeWeb {
        version: &'static str,
        configuration: Value,
        constraints: Value,
        requests: Mutex<Vec<(Method, String, Option<Value>)>>,
    }

    impl FakeWeb {
        fn new(version: &'static str) -> Self {
            Self {
                version,
                configuration: json!({
                    "pool_groups": [],
                    "temperature": { "mode": { "auto": {
                        "target_temperature": { "degree_c": 65.0 },
                        "hot_temperature": { "degree_c": 80.0 },
                        "dangerous_temperature": { "degree_c": 90.0 },
                        "min_fan_speed": 20,
                        "max_fan_speed": 90,
                        "minimum_required_fans": 2,
                        "pause_mode": null
                    } } }
                }),
                constraints: json!({
                    "cooling_constraints": {
                        "default_cooling_mode": 1,
                        "fan_pause_runtime_limited_duration_s": 120
                    }
                }),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn put_bodies(&self) -> Vec<Value> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|(method, _, _)| method == Method::PUT)
                .filter_map(|(_, _, body)| body.clone())
                .collect()
        }
    }

    #[async_trait]
    impl APIClient for FakeWeb {
        async fn get_api_result(&self, _command: &MinerCommand) -> anyhow::Result<Value> {
            unimplemented!("not used")
        }
    }

    #[async_trait]
    impl WebAPIClient for FakeWeb {
        async fn send_command(
            &self,
            command: &str,
            _privileged: bool,
            parameters: Option<Value>,
            method: Method,
        ) -> anyhow::Result<Value> {
            self.requests
                .lock()
                .unwrap()
                .push((method, command.to_string(), parameters));
            Ok(match command {
                "miner/details" => json!({ "bos_version": { "current": self.version } }),
                "configuration/miner" => self.configuration.clone(),
                "configuration/constraints" => self.constraints.clone(),
                "cooling/mode" => Value::Null,
                other => anyhow::bail!("unexpected command {other}"),
            })
        }
    }

    const V26_04: &str = "2026-04-14-0-912d084c-26.04-plus";
    const V26_01: &str = "2026-01-20-0-0123abcd-26.01-plus";
    const V25_07: &str = "2025-08-06-0-3881c51d-25.07-plus";

    #[tokio::test]
    async fn manual_pause_is_sent_and_other_cooling_fields_are_preserved() -> anyhow::Result<()> {
        let web = FakeWeb::new(V26_04);
        let pause = FanPause::manual(0.4, FanPauseRuntime::Indefinite);

        assert!(set_fan_pause(&web, pause).await?);

        assert_eq!(
            web.put_bodies(),
            vec![json!({ "auto": {
                "target_temperature": { "degree_c": 65.0 },
                "hot_temperature": { "degree_c": 80.0 },
                "dangerous_temperature": { "degree_c": 90.0 },
                "min_fan_speed": 20,
                "max_fan_speed": 90,
                "minimum_required_fans": 2,
                "pause_mode": { "mode": { "manual": {
                    "pause_cooldown_fan_speed_ratio": 0.4,
                    "fan_pause_runtime": 2
                } } }
            } })]
        );
        Ok(())
    }

    #[tokio::test]
    async fn limited_runtime_matching_the_firmware_constant_is_accepted() -> anyhow::Result<()> {
        let web = FakeWeb::new(V26_04);
        let pause = FanPause::manual(1.0, FanPauseRuntime::Limited(Duration::from_secs(120)));

        set_fan_pause(&web, pause).await?;

        assert_eq!(
            web.put_bodies()[0].pointer("/auto/pause_mode/mode/manual/fan_pause_runtime"),
            Some(&json!(1))
        );
        Ok(())
    }

    #[tokio::test]
    async fn limited_runtime_differing_from_the_firmware_constant_is_rejected() {
        let web = FakeWeb::new(V26_04);
        let pause = FanPause::manual(0.5, FanPauseRuntime::Limited(Duration::from_secs(300)));

        let err = set_fan_pause(&web, pause).await.unwrap_err();

        assert!(err.to_string().contains("fixed by the firmware at 120s"));
        assert!(web.put_bodies().is_empty());
    }

    #[tokio::test]
    async fn limited_runtime_is_rejected_when_the_miner_reports_no_constant() {
        let mut web = FakeWeb::new(V26_04);
        web.constraints = json!({ "cooling_constraints": { "default_cooling_mode": 1 } });
        let pause = FanPause::manual(0.5, FanPauseRuntime::Limited(Duration::from_secs(120)));

        assert!(set_fan_pause(&web, pause).await.is_err());
        assert!(web.put_bodies().is_empty());
    }

    #[tokio::test]
    async fn auto_pause_is_sent_with_the_other_fields_preserved() -> anyhow::Result<()> {
        let web = FakeWeb::new(V26_01);

        set_fan_pause(&web, FanPause::Auto).await?;

        let body = &web.put_bodies()[0];
        assert_eq!(
            body.pointer("/auto/pause_mode"),
            Some(&json!({ "mode": { "auto": {} } }))
        );
        assert_eq!(body.pointer("/auto/max_fan_speed"), Some(&json!(90)));
        assert_eq!(
            body.pointer("/auto/target_temperature/degree_c"),
            Some(&json!(65.0))
        );
        Ok(())
    }

    #[tokio::test]
    async fn other_cooling_modes_are_reapplied_in_their_own_mode() -> anyhow::Result<()> {
        let mut web = FakeWeb::new(V26_04);
        web.configuration = json!({ "pool_groups": [], "temperature": { "mode": { "manual": {
            "fan_speed_ratio": 0.6,
            "hot_temperature": { "degree_c": 75.0 }
        } } } });

        set_fan_pause(&web, FanPause::Auto).await?;

        assert_eq!(
            web.put_bodies(),
            vec![json!({ "manual": {
                "fan_speed_ratio": 0.6,
                "hot_temperature": { "degree_c": 75.0 },
                "pause_mode": { "mode": { "auto": {} } }
            } })]
        );
        Ok(())
    }

    #[tokio::test]
    async fn firmware_before_26_01_is_rejected_with_a_clear_error() {
        let web = FakeWeb::new(V25_07);

        let err = set_fan_pause(&web, FanPause::Auto).await.unwrap_err();

        assert!(err.to_string().contains("Braiins OS 26.01 or later"));
        assert!(web.put_bodies().is_empty());
    }

    #[tokio::test]
    async fn manual_pause_before_26_04_is_rejected_because_the_runtime_is_unsupported() {
        let web = FakeWeb::new(V26_01);
        let pause = FanPause::manual(0.5, FanPauseRuntime::Indefinite);

        let err = set_fan_pause(&web, pause).await.unwrap_err();

        assert!(err.to_string().contains("26.04"));
        assert!(web.put_bodies().is_empty());
    }

    #[tokio::test]
    async fn invalid_ratio_is_rejected_before_any_request() {
        let web = FakeWeb::new(V26_04);

        for bad in [-0.1, 1.5, f64::NAN] {
            let pause = FanPause::manual(bad, FanPauseRuntime::Indefinite);
            assert!(set_fan_pause(&web, pause).await.is_err());
        }
        assert!(web.requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsettable_cooling_mode_is_rejected() {
        let mut web = FakeWeb::new(V26_04);
        web.configuration =
            json!({ "pool_groups": [], "temperature": { "mode": { "disabled": {} } } });

        assert!(set_fan_pause(&web, FanPause::Auto).await.is_err());
        assert!(web.put_bodies().is_empty());
    }
}
