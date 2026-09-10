use std::path::Path;

use anyhow::Context;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use semver::Version;
use tokio::io::AsyncReadExt;

/// Result of checking a miner for an available firmware update.
///
/// Read-only and obtained on demand (a firmware-update check usually hits the
/// vendor's release server, so it is not part of the regular telemetry poll).
#[cfg_attr(
    feature = "python",
    pyclass(name = "FirmwareStats", skip_from_py_object, module = "asic_rs")
)]
#[derive(Debug, Clone, Default)]
pub struct FirmwareStats {
    /// The firmware version currently installed, if known.
    pub current_version: Option<Version>,
    /// The latest firmware version offered by the vendor, if any.
    pub latest_version: Option<Version>,
    /// Where to obtain the available update, if any: a local image or a remote URL.
    pub firmware: Option<FirmwareUpdate>,
}

impl FirmwareStats {
    /// Whether a newer firmware than the installed one is available.
    ///
    /// Derived by comparing `current_version` to `latest_version`; returns
    /// `false` when either version is unknown.
    pub fn update_available(&self) -> bool {
        match (&self.current_version, &self.latest_version) {
            (Some(current), Some(latest)) => latest > current,
            _ => false,
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl FirmwareStats {
    #[getter(current_version)]
    fn py_current_version(&self) -> Option<String> {
        self.current_version.as_ref().map(ToString::to_string)
    }

    #[getter(latest_version)]
    fn py_latest_version(&self) -> Option<String> {
        self.latest_version.as_ref().map(ToString::to_string)
    }

    #[getter(update_available)]
    fn py_update_available(&self) -> bool {
        self.update_available()
    }

    #[getter(firmware)]
    fn py_firmware(&self) -> Option<FirmwareUpdate> {
        self.firmware.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "FirmwareStats(current_version={:?}, latest_version={:?}, update_available={})",
            self.current_version.as_ref().map(ToString::to_string),
            self.latest_version.as_ref().map(ToString::to_string),
            self.update_available(),
        )
    }
}

/// Where an available firmware update can be obtained: either a local image
/// (bytes already in hand) or a remote URL to download from.
#[derive(Debug, Clone)]
pub enum FirmwareUpdate {
    /// A firmware image available locally (e.g. already downloaded).
    Local(FirmwareImage),
    /// A URL the firmware can be downloaded from.
    Remote(String),
}

/// Firmware bytes ready for validation, preparation, or upload.
///
/// For container formats, [`crate::traits::miner::UpgradeFirmware::prepare_firmware`]
/// may return a different filename and payload selected for the target miner.
#[cfg_attr(
    feature = "python",
    pyclass(skip_from_py_object, get_all, module = "asic_rs")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    /// Filename presented to the miner during upload.
    pub filename: String,
    /// Complete firmware payload bytes.
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

#[cfg(feature = "python")]
pub use python_firmware_update::PyFirmwareUpdate;

#[cfg(feature = "python")]
mod python_firmware_update {
    use pyo3::prelude::*;

    use super::FirmwareUpdate;

    /// Python view of [`FirmwareUpdate`]: the source of an available update.
    #[pyclass(name = "FirmwareUpdate", skip_from_py_object, module = "asic_rs")]
    #[derive(Debug, Clone)]
    pub enum PyFirmwareUpdate {
        Local { filename: String, data: Vec<u8> },
        Remote { url: String },
    }

    #[pymethods]
    impl PyFirmwareUpdate {
        #[getter]
        fn variant(&self) -> &'static str {
            match self {
                Self::Local { .. } => "local",
                Self::Remote { .. } => "remote",
            }
        }

        #[getter]
        fn url(&self) -> Option<String> {
            match self {
                Self::Remote { url } => Some(url.clone()),
                Self::Local { .. } => None,
            }
        }

        #[getter]
        fn filename(&self) -> Option<String> {
            match self {
                Self::Local { filename, .. } => Some(filename.clone()),
                Self::Remote { .. } => None,
            }
        }

        #[getter]
        fn data(&self) -> Option<Vec<u8>> {
            match self {
                Self::Local { data, .. } => Some(data.clone()),
                Self::Remote { .. } => None,
            }
        }

        fn __repr__(&self) -> String {
            match self {
                Self::Local { filename, data } => {
                    format!(
                        "FirmwareUpdate.Local(filename={filename:?}, {} bytes)",
                        data.len()
                    )
                }
                Self::Remote { url } => format!("FirmwareUpdate.Remote(url={url:?})"),
            }
        }
    }

    impl From<FirmwareUpdate> for PyFirmwareUpdate {
        fn from(value: FirmwareUpdate) -> Self {
            match value {
                FirmwareUpdate::Local(image) => Self::Local {
                    filename: image.filename,
                    data: image.bytes,
                },
                FirmwareUpdate::Remote(url) => Self::Remote { url },
            }
        }
    }

    impl<'py> pyo3::IntoPyObject<'py> for FirmwareUpdate {
        type Target = pyo3::PyAny;
        type Output = pyo3::Bound<'py, pyo3::PyAny>;
        type Error = pyo3::PyErr;

        const OUTPUT_TYPE: pyo3::inspect::PyStaticExpr =
            { <PyFirmwareUpdate as pyo3::PyTypeInfo>::TYPE_HINT };

        fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
            PyFirmwareUpdate::from(self)
                .into_pyobject(py)
                .map(pyo3::Bound::into_any)
        }
    }
}
