use std::path::Path;

use anyhow::Context;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

/// Result of checking a miner for an available firmware update.
///
/// Read-only and obtained on demand (a firmware-update check usually hits the
/// vendor's release server, so it is not part of the regular telemetry poll).
#[cfg_attr(
    feature = "python",
    pyclass(get_all, skip_from_py_object, module = "asic_rs")
)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FirmwareUpdate {
    /// The firmware version currently installed, if known.
    pub current_version: Option<String>,
    /// The latest firmware version offered by the vendor, if any.
    pub latest_version: Option<String>,
    /// Whether a newer firmware than the installed one is available.
    pub update_available: bool,
    /// Release date of the latest firmware, as reported by the vendor.
    pub release_date: Option<String>,
    /// URL of the latest firmware release, when provided.
    pub release_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    pub filename: String,
    pub bytes: Vec<u8>,
}

impl FirmwareImage {
    pub fn new(filename: String, bytes: Vec<u8>) -> Self {
        Self { filename, bytes }
    }

    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let filename = Self::filename_from_path(path)?;
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read firmware file: {}", path.display()))?;

        Ok(Self { filename, bytes })
    }

    pub async fn from_file_async(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let filename = Self::filename_from_path(path)?;
        let mut file = tokio::fs::File::open(path)
            .await
            .with_context(|| format!("Failed to open firmware file: {}", path.display()))?;
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut chunk)
                .await
                .with_context(|| format!("Failed to read firmware file: {}", path.display()))?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            tokio::task::yield_now().await;
        }

        Ok(Self { filename, bytes })
    }

    fn filename_from_path(path: &Path) -> anyhow::Result<String> {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .context("Firmware path must include a valid UTF-8 filename")
    }
}
