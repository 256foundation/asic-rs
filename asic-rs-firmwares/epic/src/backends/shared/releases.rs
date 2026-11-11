use std::{future::Future, time::Duration};

use anyhow::{Context, bail};
use asic_rs_core::data::{firmware::FirmwareStats, miner::MinerData};
use asic_rs_makes_antminer::hardware::AntMinerControlBoard;
use asic_rs_makes_epic::hardware::EPicControlBoard;
use asic_rs_makes_volcminer::make::VolcMinerMake;
use semver::Version;
use serde_json::Value;
use tokio::{sync::Mutex, time::Instant};

const UMC: &str = "powerplay-antminer";
const STOCK: &str = "umcos-antminer";
const RELEASE_TTL: Duration = Duration::from_secs(5 * 60);
const FAILURE_TTL: Duration = Duration::from_secs(60);

struct CachedRelease {
    result: Result<Value, String>,
    expires_at: Instant,
}

#[derive(Default)]
struct ReleaseCache {
    umc: Mutex<Option<CachedRelease>>,
    stock: Mutex<Option<CachedRelease>>,
}

static RELEASE_CACHE: ReleaseCache = ReleaseCache {
    umc: Mutex::const_new(None),
    stock: Mutex::const_new(None),
};

impl ReleaseCache {
    async fn get<F, Fut>(&self, repository: &str, fetch: F) -> anyhow::Result<Value>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<Value>>,
    {
        let entry = match repository {
            UMC => &self.umc,
            STOCK => &self.stock,
            _ => bail!("Unknown ePIC release repository {repository:?}"),
        };
        // Hold only this repository's lock across the request. Concurrent callers
        // share its result; the other repository can refresh independently.
        let mut entry = entry.lock().await;
        if let Some(cached) = entry.as_ref()
            && Instant::now() < cached.expires_at
        {
            return cached.result.clone().map_err(anyhow::Error::msg);
        }

        let result = fetch().await.and_then(|release| {
            // Validate before caching, so malformed release data backs off too.
            stats(&release, None)?;
            Ok(release)
        });
        let ttl = if result.is_ok() {
            RELEASE_TTL
        } else {
            FAILURE_TTL
        };
        let result = result.map_err(|error| format!("{error:#}"));
        *entry = Some(CachedRelease {
            result: result.clone(),
            expires_at: Instant::now() + ttl,
        });
        result.map_err(anyhow::Error::msg)
    }
}

pub(crate) fn repository(
    platform: Option<&str>,
    cpu: Option<&str>,
    make: &str,
) -> anyhow::Result<&'static str> {
    if let Some(platform) = platform.filter(|s| !s.trim().is_empty()) {
        if EPicControlBoard::parse(platform).is_some() {
            return Ok(UMC);
        }
        if AntMinerControlBoard::parse(platform).is_some() {
            return Ok(STOCK);
        }
        bail!("No ePIC release repository for control board platform {platform:?}");
    }

    // Older firmware reports only cpuHardware. Do not use the telemetry
    // parser's default-to-UMC fallback when selecting a release family.
    let cpu = cpu.unwrap_or_default().to_ascii_uppercase();
    // Legacy VolcMiner firmware omits platform and shares the Xilinx CPU label
    // with AntMiner boards, but cannot use the AntMiner release family.
    if cpu.contains("XILINX") && make == VolcMinerMake::default().to_string() {
        bail!("No ePIC release repository for legacy VolcMiner Xilinx control boards");
    }
    if cpu.contains("STM32") {
        Ok(UMC)
    } else if cpu.contains("AMLOGIC")
        || cpu.contains("XILINX")
        || cpu.contains("GENERIC AM33XX")
        || cpu.contains("CVITEK")
    {
        Ok(STOCK)
    } else {
        bail!("Cannot identify control board for ePIC firmware update check")
    }
}

pub(crate) fn snapshot_update_target(
    data: &MinerData,
) -> anyhow::Result<(&'static str, Option<Version>)> {
    let board = data
        .control_board_version
        .as_ref()
        .context("Snapshot is missing control board metadata")?;
    let repository = repository(
        board.platform.as_deref(),
        board.cpu.as_deref(),
        &data.device_info.make,
    )?;
    let current_version = data.firmware_version.as_deref().and_then(parse_version);
    Ok((repository, current_version))
}

pub(crate) fn parse_version(value: &str) -> Option<Version> {
    Version::parse(value.split_whitespace().last()?.trim_start_matches('v')).ok()
}

pub(crate) async fn check(
    repository: &str,
    current_version: Option<Version>,
) -> anyhow::Result<FirmwareStats> {
    let release = RELEASE_CACHE
        .get(repository, || fetch_release(repository))
        .await?;
    // Installed versions belong to individual miners, never to the cache.
    stats(&release, current_version)
}

async fn fetch_release(repository: &str) -> anyhow::Result<Value> {
    // Use a separate client: miner credentials must never be sent to GitHub.
    reqwest::Client::builder()
        .user_agent(concat!("asic-rs/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()?
        .get(format!(
            "https://api.github.com/repos/epicblockchain/{repository}/releases/latest"
        ))
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .context("Failed to query ePIC GitHub releases")?
        .error_for_status()?
        .json::<Value>()
        .await
        .context("Failed to decode ePIC GitHub release")
}

fn stats(release: &Value, current_version: Option<Version>) -> anyhow::Result<FirmwareStats> {
    let latest_version = release["tag_name"]
        .as_str()
        .and_then(parse_version)
        .context("ePIC GitHub release has no valid version tag")?;
    Ok(FirmwareStats {
        current_version,
        latest_version: Some(latest_version),
        // Releases contain multiple board/model images. Version discovery does
        // not yet resolve which asset is compatible with this miner.
        firmware: None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn concurrent_devices_share_release_but_keep_installed_versions() -> anyhow::Result<()> {
        let cache = Arc::new(ReleaseCache::default());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = tokio::task::JoinSet::new();
        for patch in 0..64 {
            let cache = Arc::clone(&cache);
            let calls = Arc::clone(&calls);
            tasks.spawn(async move {
                let release = cache
                    .get(UMC, || async {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::task::yield_now().await;
                        Ok(json!({"tag_name": "v1.26.3"}))
                    })
                    .await?;
                let current = Version::new(1, 0, patch);
                let result = stats(&release, Some(current.clone()))?;
                assert_eq!(result.current_version, Some(current));
                assert_eq!(result.latest_version, Some(Version::new(1, 26, 3)));
                anyhow::Ok(())
            });
        }
        while let Some(result) = tasks.join_next().await {
            result??;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test]
    async fn repositories_refresh_independently_and_expired_entries_reload() -> anyhow::Result<()> {
        let cache = ReleaseCache::default();
        let barrier = tokio::sync::Barrier::new(2);
        let (umc, stock) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(
                cache.get(UMC, || async {
                    barrier.wait().await;
                    Ok(json!({"tag_name": "v1.26.3"}))
                }),
                cache.get(STOCK, || async {
                    barrier.wait().await;
                    Ok(json!({"tag_name": "v1.27.0"}))
                })
            )
        })
        .await?;
        assert_eq!(umc?["tag_name"], "v1.26.3");
        assert_eq!(stock?["tag_name"], "v1.27.0");

        cache.umc.lock().await.as_mut().unwrap().expires_at = Instant::now();
        let refreshed = cache
            .get(UMC, || async { Ok(json!({"tag_name": "v1.28.0"})) })
            .await?;
        assert_eq!(refreshed["tag_name"], "v1.28.0");
        let stock = cache
            .get(STOCK, || async {
                panic!("Refreshing UMC must not invalidate the stock-board cache");
            })
            .await?;
        assert_eq!(stock["tag_name"], "v1.27.0");
        Ok(())
    }

    #[tokio::test]
    async fn failures_back_off_then_retry_instead_of_poisoning_the_cache() -> anyhow::Result<()> {
        let cache = ReleaseCache::default();
        let calls = AtomicUsize::new(0);
        for _ in 0..2 {
            let error = cache
                .get(UMC, || async {
                    calls.fetch_add(1, Ordering::SeqCst);
                    anyhow::bail!("GitHub unavailable")
                })
                .await
                .unwrap_err();
            assert!(error.to_string().contains("GitHub unavailable"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let cached = cache.umc.lock().await;
        assert!(cached.as_ref().unwrap().expires_at <= Instant::now() + FAILURE_TTL);
        drop(cached);
        cache.umc.lock().await.as_mut().unwrap().expires_at = Instant::now();
        let recovered = cache
            .get(UMC, || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(json!({"tag_name": "v1.26.3"}))
            })
            .await?;
        assert_eq!(recovered["tag_name"], "v1.26.3");
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        assert!(
            cache
                .get(STOCK, || async { Ok(json!({"tag_name": "not-a-version"})) })
                .await
                .is_err()
        );
        assert!(
            cache
                .get(STOCK, || async {
                    panic!("Malformed release responses must back off too");
                })
                .await
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn compares_installed_umc_labels_with_github_tags() -> anyhow::Result<()> {
        let release = json!({"tag_name": "v1.26.3"});
        for (installed, expected) in [
            ("PowerPlay-BMS v1.9.0", true),
            ("PowerPlay-BMS v1.26.3", false),
            ("PowerPlay-BMS v1.38.3", false),
            ("PowerPlay-BMS v1.26.3-rc.1", true),
            ("PowerPlay-BMS v1.26.3+build", false),
        ] {
            let current = parse_version(installed).unwrap();
            let result = stats(&release, Some(current))?;
            assert_eq!(result.update_available(), expected, "{installed}");
        }
        let result = stats(&release, parse_version("unknown"))?;
        assert!(!result.update_available());
        Ok(())
    }

    #[test]
    fn selects_release_family() {
        for platform in ["ePIC UMC", "UMC"] {
            assert_eq!(repository(Some(platform), None, "AntMiner").unwrap(), UMC);
        }
        for platform in ["AMLogic", "Xilinx", "BeagleBoneBlack", "CVITek"] {
            assert_eq!(repository(Some(platform), None, "AntMiner").unwrap(), STOCK);
        }
        for (cpu, expected) in [
            ("STM32 (Device Tree Support)", UMC),
            ("AMLOGIC", STOCK),
            ("Xilinx Zynq", STOCK),
            ("Generic AM33XX", STOCK),
        ] {
            assert_eq!(repository(None, Some(cpu), "AntMiner").unwrap(), expected);
        }
        assert!(repository(None, None, "AntMiner").is_err());
        assert!(repository(None, Some("unknown"), "AntMiner").is_err());
        assert!(repository(Some("TVXilinx"), Some("STM32"), "AntMiner").is_err());
    }

    #[test]
    fn rejects_legacy_volcminer_xilinx_boards() {
        let make = VolcMinerMake::default().to_string();
        for platform in [None, Some(""), Some(" "), Some("TVXilinx")] {
            assert!(repository(platform, Some("Xilinx Zynq"), &make).is_err());
        }
        assert_eq!(
            repository(None, Some("Xilinx Zynq"), "AntMiner").unwrap(),
            STOCK
        );
    }

    #[test]
    fn parses_installed_and_release_versions() {
        assert_eq!(
            parse_version("PowerPlay-BMS v1.5.13"),
            Some(Version::new(1, 5, 13))
        );
        assert_eq!(parse_version("v1.26.3"), Some(Version::new(1, 26, 3)));
        assert!(parse_version("unknown").is_none());
    }

    #[test]
    fn reports_release_without_guessing_an_image() {
        let release = json!({"tag_name": "v1.26.3"});
        let newer = stats(&release, Some(Version::new(1, 5, 13))).unwrap();
        assert!(newer.update_available());
        assert!(newer.firmware.is_none());
        assert!(
            !stats(&release, Some(Version::new(1, 26, 3)))
                .unwrap()
                .update_available()
        );
        assert!(
            !stats(&release, Some(Version::new(1, 38, 3)))
                .unwrap()
                .update_available()
        );
        assert!(!stats(&release, None).unwrap().update_available());
        assert!(stats(&json!({}), None).is_err());
        assert!(stats(&json!({"tag_name": "bad"}), None).is_err());
    }
    #[test]
    fn snapshot_retains_raw_release_metadata_and_rejects_guessed_board_identity()
    -> anyhow::Result<()> {
        use crate::backends::PowerPlay;
        use asic_rs_core::{data::collector::DataField, traits::miner::MinerConstructor};
        use asic_rs_makes_antminer::models::AntMinerModel;
        use std::{collections::HashMap, net::IpAddr};

        for version in [None, Some(Version::new(1, 28, 4))] {
            let miner = PowerPlay::new(IpAddr::from([127, 0, 0, 1]), AntMinerModel::S19XP, version);
            for (platform, cpu, expected) in [
                (Some("UMC"), Some("STM32"), Some(UMC)),
                (Some("AMLogic"), Some("AMLOGIC"), Some(STOCK)),
                (None, Some("STM32 (Device Tree Support)"), Some(UMC)),
                (None, Some("Xilinx Zynq"), Some(STOCK)),
                (None, Some("unknown"), None),
                (Some("TVXilinx"), Some("STM32"), None),
                (None, None, None),
            ] {
                let snapshot = miner.parse_data(HashMap::from([
                    (
                        DataField::ControlBoardVersion,
                        json!({"platform": platform, "cpu": cpu}),
                    ),
                    (DataField::FirmwareVersion, json!("PowerPlay-BMS v1.28.4")),
                ]));
                let encoded = serde_json::to_value(&snapshot)?;
                let snapshot: MinerData = serde_json::from_value(encoded)?;
                if let Some(board) = snapshot.control_board_version.as_ref() {
                    assert_eq!(board.platform.as_deref(), platform);
                    assert_eq!(board.cpu.as_deref(), cpu);
                }
                let target = snapshot_update_target(&snapshot);
                if let Some(expected) = expected {
                    let (repository, version) = target?;
                    assert_eq!(repository, expected);
                    assert_eq!(version, Some(Version::new(1, 28, 4)));
                } else {
                    assert!(target.is_err());
                }
            }
            // An old serialized snapshot lacks raw metadata: never trust the normalized name.
            let mut snapshot = miner.parse_data(HashMap::new());
            snapshot.control_board_version = Some(
                asic_rs_core::data::board::MinerControlBoard::known("ePIC UMC".to_string()),
            );
            assert!(snapshot_update_target(&snapshot).is_err());
        }
        Ok(())
    }

    #[tokio::test]
    async fn snapshot_update_check_only_uses_the_release_cache() -> anyhow::Result<()> {
        use crate::backends::PowerPlay;
        use asic_rs_core::{data::collector::DataField, traits::miner::MinerConstructor};
        use asic_rs_makes_antminer::models::AntMinerModel;
        use std::{collections::HashMap, net::IpAddr};

        // No miner or release server is running. Any accidental miner read will fail
        // or exceed this deadline; the vendor result is provided by the shared cache.
        *RELEASE_CACHE.umc.lock().await = Some(CachedRelease {
            result: Ok(json!({"tag_name": "v1.40.0"})),
            expires_at: Instant::now() + RELEASE_TTL,
        });
        for version in [None, Some(Version::new(1, 28, 4))] {
            let miner = PowerPlay::new(IpAddr::from([192, 0, 2, 1]), AntMinerModel::S19XP, version);
            assert!(miner.supports_check_firmware_update_from_data());
            for installed in ["v1.28.4", "v1.41.0"] {
                let mut snapshot = miner.parse_data(HashMap::from([
                    (
                        DataField::ControlBoardVersion,
                        json!({"platform": "UMC", "cpu": "STM32"}),
                    ),
                    (DataField::FirmwareVersion, json!(installed)),
                ]));
                let stats = tokio::time::timeout(
                    Duration::from_secs(1),
                    miner.check_firmware_update_from_data(&snapshot),
                )
                .await??;
                assert_eq!(stats.current_version, parse_version(installed));
                assert_eq!(stats.latest_version, Some(Version::new(1, 40, 0)));
                assert_eq!(stats.update_available(), installed == "v1.28.4");
                snapshot.ip = IpAddr::from([192, 0, 2, 2]);
                assert!(
                    miner
                        .check_firmware_update_from_data(&snapshot)
                        .await
                        .is_err()
                );
            }
            let snapshot = miner.parse_data(HashMap::new());
            assert!(
                miner
                    .check_firmware_update_from_data(&snapshot)
                    .await
                    .is_err()
            );
        }
        *RELEASE_CACHE.umc.lock().await = None;
        Ok(())
    }
}
