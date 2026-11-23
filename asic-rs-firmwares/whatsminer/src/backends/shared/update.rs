use super::package;

use anyhow::{Context, Result, bail};
pub(crate) use package::{Device, Package, platform_version};
pub use package::{
    FirmwareImageSpan, FirmwareInspection, FirmwareMember, FirmwareRepresentation, LegacyHeader,
    MAX_PACKAGE_SIZE,
};
use serde_json::Value;
use std::time::Duration;
use tokio::{io::AsyncWriteExt, net::TcpStream, time::timeout};

pub fn inspect_firmware(bytes: &[u8]) -> Result<FirmwareInspection> {
    package::inspect(bytes)
}

pub(crate) const COMMAND_DEADLINE: Duration = Duration::from_secs(15);
const UPLOAD_DEADLINE: Duration = Duration::from_secs(180);

#[derive(Debug)]
pub struct UploadOutcomeUnconfirmed {
    pub stage: &'static str,
}

impl std::fmt::Display for UploadOutcomeUnconfirmed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Firmware outcome unconfirmed: {}; verify the device before retrying",
            self.stage
        )
    }
}

impl std::error::Error for UploadOutcomeUnconfirmed {}

pub(crate) async fn transfer(socket: &mut TcpStream, member: &[u8]) -> Result<()> {
    let length = u32::try_from(member.len())?;
    let transfer = timeout(UPLOAD_DEADLINE, async {
        socket.write_all(&length.to_le_bytes()).await?;
        socket.write_all(member).await
    })
    .await;
    if !matches!(transfer, Ok(Ok(()))) {
        return Err(UploadOutcomeUnconfirmed {
            stage: "member transfer interrupted",
        }
        .into());
    }
    Ok(())
}

pub(crate) fn field<'a>(msg: &'a Value, key: &str) -> Result<&'a str> {
    msg[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("missing WhatsMiner {key}"))
}

pub(crate) fn parse_btrom(msg: &Value) -> Result<bool> {
    match &msg["btrom"] {
        Value::Bool(flag) => Ok(*flag),
        value if value == "1" || value == 1 => Ok(true),
        value if value == "0" || value == 0 => Ok(false),
        Value::Null => Ok(false),
        Value::String(s) if s.trim().is_empty() => Ok(false),
        _ => bail!("invalid btrom"),
    }
}

pub(crate) fn chip_selector(chip_data: &Value, model: &Value) -> String {
    let first_board = chip_data
        .as_str()
        .unwrap_or_default()
        .split('_')
        .next()
        .unwrap_or_default();
    let tail = first_board.rsplit('-').next().unwrap_or_default();
    const LEGACY_CHIPS: [&str; 6] = ["1917", "1918", "1919", "1920", "1921", "1922"];
    if LEGACY_CHIPS.iter().any(|id| tail.starts_with(id))
        || tail.match_indices("KF").any(|(pos, _)| {
            LEGACY_CHIPS
                .iter()
                .any(|id| tail[pos + 2..].starts_with(id))
        })
        || first_board.find("1918 BIN2").is_some_and(|pos| pos > 0)
    {
        return "1920".into();
    }
    if let Some(prefix) = tail.as_bytes().get(..4) {
        return String::from_utf8_lossy(prefix).into_owned();
    }
    let model = model.as_str().unwrap_or_default();
    if model.contains("M2") {
        "1920".into()
    } else if model.contains("M3") {
        "1930".into()
    } else {
        String::new()
    }
}
