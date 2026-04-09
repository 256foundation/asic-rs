use std::path::Path;

use anyhow::Context;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FirmwareUpgradeOptions {
    pub retain_settings: Option<bool>,
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
        file.read_to_end(&mut bytes)
            .await
            .with_context(|| format!("Failed to read firmware file: {}", path.display()))?;

        Ok(Self { filename, bytes })
    }

    fn filename_from_path(path: &Path) -> anyhow::Result<String> {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .context("Firmware path must include a valid UTF-8 filename")
    }
}
