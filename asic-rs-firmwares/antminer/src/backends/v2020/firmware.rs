use anyhow::{Context, bail};
use asic_rs_core::data::firmware::FirmwareImage;
use crc32fast::Hasher;
use serde_json::Value;

use super::AntMinerV2020;

const BMU_MAGIC: u32 = 0xABABABAB;
const BMU_HEADER_SIZE: usize = 36;
const BMU_ITEM_FIXED_SIZE: usize = 172;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinerTypeInfo {
    pub(super) model: String,
    pub(super) subtype: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BmuEntry {
    filename: String,
    chip: String,
    hardware: String,
    model: String,
    bytes: Vec<u8>,
}

pub(super) trait AntMinerFirmwareImageExt {
    fn resolve_for_miner(self, miner: &MinerTypeInfo) -> anyhow::Result<FirmwareImage>;
}

pub(super) trait AntMinerFirmwareUpgradeResponseExt {
    fn validate_firmware_upgrade_response(&self) -> anyhow::Result<()>;
}

impl AntMinerFirmwareImageExt for FirmwareImage {
    fn resolve_for_miner(self, miner: &MinerTypeInfo) -> anyhow::Result<FirmwareImage> {
        resolve_firmware_image_inner(self.filename, self.bytes, miner, 0)
    }
}

impl AntMinerFirmwareUpgradeResponseExt for str {
    fn validate_firmware_upgrade_response(&self) -> anyhow::Result<()> {
        let parsed: Value = serde_json::from_str(self)
            .map_err(|e| anyhow::anyhow!("Invalid firmware upload response: {}", e))?;
        let code = parsed.get("code").and_then(|value| value.as_str());
        let stats = parsed.get("stats").and_then(|value| value.as_str());
        if code == Some("U000") && stats == Some("success") {
            return Ok(());
        }

        let message = parsed
            .get("msg")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown error");
        bail!("Firmware upload rejected: {}", message);
    }
}

fn sanitize_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn normalized(value: &str) -> String {
    sanitize_name(value).to_ascii_lowercase()
}

fn compact_normalized(value: &str) -> String {
    normalized(value)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn wildcard_compatible(left: &str, right: &str) -> bool {
    let left = compact_normalized(left);
    let right = compact_normalized(right);

    left.len() == right.len()
        && left
            .chars()
            .zip(right.chars())
            .all(|(lhs, rhs)| lhs == rhs || lhs == 'x' || rhs == 'x')
}

fn read_u32_le(bytes: &[u8], offset: usize) -> anyhow::Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .context("BMU is truncated while reading u32")?;
    let array: [u8; 4] = raw.try_into().context("Invalid u32 field length")?;
    Ok(u32::from_le_bytes(array))
}

fn parse_bmu_entries(bytes: &[u8]) -> anyhow::Result<Option<Vec<BmuEntry>>> {
    if bytes.len() < BMU_HEADER_SIZE {
        return Ok(None);
    }

    let magic = read_u32_le(bytes, 0)?;
    if magic != BMU_MAGIC {
        return Ok(None);
    }

    let header_size = read_u32_le(bytes, 8)? as usize;
    let item_count = read_u32_le(bytes, 12)? as usize;
    let item_size = read_u32_le(bytes, 16)? as usize;
    let crc32 = read_u32_le(bytes, 24)?;

    if header_size != BMU_HEADER_SIZE {
        bail!("Unsupported BMU header size: {header_size}");
    }
    if item_size < BMU_ITEM_FIXED_SIZE {
        bail!("Unsupported BMU item size: {item_size}");
    }

    let table_end = BMU_HEADER_SIZE
        .checked_add(
            item_count
                .checked_mul(item_size)
                .context("BMU item table size overflow")?,
        )
        .context("BMU table end overflow")?;
    if table_end > bytes.len() {
        bail!("BMU item table exceeds file size");
    }

    let mut hasher = Hasher::new();
    hasher.update(&bytes[..24]);
    hasher.update(&[0, 0, 0, 0]);
    hasher.update(&bytes[28..]);
    if hasher.finalize() != crc32 {
        bail!("BMU CRC mismatch");
    }

    let mut entries = Vec::with_capacity(item_count);
    for idx in 0..item_count {
        let offset = BMU_HEADER_SIZE + idx * item_size;
        let entry = bytes
            .get(offset..offset + item_size)
            .context("BMU entry exceeds file size")?;

        let filename_len = entry[0] as usize;
        let chip_len = entry[1] as usize;
        let hardware_len = entry[2] as usize;
        let model_len = entry[3] as usize;

        let filename = decode_bmu_field(
            entry.get(4..68).context("BMU missing filename field")?,
            filename_len,
        );
        let chip = decode_bmu_field(
            entry.get(68..100).context("BMU missing chip field")?,
            chip_len,
        );
        let hardware = decode_bmu_field(
            entry.get(100..132).context("BMU missing hardware field")?,
            hardware_len,
        );
        let model = decode_bmu_field(
            entry.get(132..164).context("BMU missing model field")?,
            model_len,
        );

        let data_offset = read_u32_le(entry, 164)? as usize;
        let size = read_u32_le(entry, 168)? as usize;
        let data_end = data_offset
            .checked_add(size)
            .context("BMU payload size overflow")?;
        let data = bytes
            .get(data_offset..data_end)
            .context("BMU payload exceeds file size")?;

        entries.push(BmuEntry {
            filename,
            chip,
            hardware,
            model,
            bytes: data.to_vec(),
        });
    }

    Ok(Some(entries))
}

fn decode_bmu_field(field: &[u8], len: usize) -> String {
    let end = len.min(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

fn candidate_score(entry: &BmuEntry, miner: &MinerTypeInfo) -> Option<u8> {
    let model = normalized(&entry.model);
    let hardware = normalized(&entry.hardware);
    let chip = normalized(&entry.chip);
    let filename = normalized(&entry.filename);
    let miner_model = normalized(&miner.model);
    let miner_subtype = normalized(&miner.subtype);

    if model != miner_model {
        return None;
    }

    if miner_subtype.is_empty() {
        Some(1)
    } else if hardware == miner_subtype || chip == miner_subtype {
        Some(4)
    } else if wildcard_compatible(&entry.hardware, &miner.subtype)
        || wildcard_compatible(&entry.chip, &miner.subtype)
    {
        Some(4)
    } else if hardware.contains(&miner_subtype) || chip.contains(&miner_subtype) {
        Some(3)
    } else if filename.contains(&miner_subtype) {
        Some(2)
    } else if !hardware.is_empty() || !chip.is_empty() || filename.contains("ctrl") {
        None
    } else {
        Some(1)
    }
}

fn resolve_firmware_image_inner(
    filename: String,
    firmware: Vec<u8>,
    miner: &MinerTypeInfo,
    depth: usize,
) -> anyhow::Result<FirmwareImage> {
    if depth > 8 {
        bail!("BMU nesting depth exceeded");
    }

    let Some(entries) = parse_bmu_entries(&firmware)? else {
        return Ok(FirmwareImage::new(filename, firmware));
    };

    let best = entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| candidate_score(entry, miner).map(|score| (score, idx)))
        .max_by_key(|(score, _)| *score)
        .map(|(_, idx)| entries[idx].clone())
        .context("No matching firmware image found in BMU bundle")?;

    resolve_firmware_image_inner(best.filename, best.bytes, miner, depth + 1)
}

impl AntMinerV2020 {
    pub(super) async fn get_miner_type_info(&self) -> anyhow::Result<MinerTypeInfo> {
        let info = self.web.miner_type().await?;
        let model = info
            .get("miner_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .context("Missing miner_type in miner_type.cgi response")?;

        let subtype = info
            .get("subtype")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or_default();

        Ok(MinerTypeInfo {
            model: model.to_string(),
            subtype: subtype.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn encode_field(target: &mut [u8], value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(target.len());
        target[..len].copy_from_slice(&bytes[..len]);
    }

    fn build_bmu_entry(
        filename: &str,
        chip: &str,
        hardware: &str,
        model: &str,
        payload: &[u8],
        data_offset: usize,
    ) -> Vec<u8> {
        let mut entry = vec![0_u8; BMU_ITEM_FIXED_SIZE];
        entry[0] = filename.len().min(64) as u8;
        entry[1] = chip.len().min(32) as u8;
        entry[2] = hardware.len().min(32) as u8;
        entry[3] = model.len().min(32) as u8;
        encode_field(&mut entry[4..68], filename);
        encode_field(&mut entry[68..100], chip);
        encode_field(&mut entry[100..132], hardware);
        encode_field(&mut entry[132..164], model);
        write_u32_le(&mut entry, 164, data_offset as u32);
        write_u32_le(&mut entry, 168, payload.len() as u32);
        entry
    }

    fn build_bmu(entries: &[(&str, &str, &str, &str, &[u8])]) -> Vec<u8> {
        let item_size = BMU_ITEM_FIXED_SIZE;
        let table_size = BMU_HEADER_SIZE + entries.len() * item_size;
        let payload_size: usize = entries
            .iter()
            .map(|(_, _, _, _, payload)| payload.len())
            .sum();
        let mut bytes = vec![0_u8; table_size + payload_size];

        write_u32_le(&mut bytes, 0, BMU_MAGIC);
        write_u32_le(&mut bytes, 8, BMU_HEADER_SIZE as u32);
        write_u32_le(&mut bytes, 12, entries.len() as u32);
        write_u32_le(&mut bytes, 16, item_size as u32);

        let mut next_payload_offset = table_size;
        for (idx, (filename, chip, hardware, model, payload)) in entries.iter().enumerate() {
            let entry = build_bmu_entry(
                filename,
                chip,
                hardware,
                model,
                payload,
                next_payload_offset,
            );
            let start = BMU_HEADER_SIZE + idx * item_size;
            bytes[start..start + item_size].copy_from_slice(&entry);
            bytes[next_payload_offset..next_payload_offset + payload.len()]
                .copy_from_slice(payload);
            next_payload_offset += payload.len();
        }

        let mut hasher = Hasher::new();
        hasher.update(&bytes[..24]);
        hasher.update(&[0, 0, 0, 0]);
        hasher.update(&bytes[28..]);
        let crc32 = hasher.finalize();
        write_u32_le(&mut bytes, 24, crc32);

        bytes
    }

    #[test]
    fn raw_firmware_is_passed_through_when_file_is_not_bmu() {
        let firmware = vec![1, 2, 3, 4, 5];
        let miner = MinerTypeInfo {
            model: "S21".to_string(),
            subtype: "X21".to_string(),
        };

        let resolved =
            resolve_firmware_image_inner("stock.bin".to_string(), firmware.clone(), &miner, 0)
                .unwrap();

        assert_eq!(resolved.filename, "stock.bin");
        assert_eq!(resolved.bytes, firmware);
    }

    #[test]
    fn bmu_selects_matching_entry_for_model_and_subtype() {
        let bmu = build_bmu(&[
            ("s21-xp.bin", "X21", "X21", "S21", b"wrong"),
            ("s21-hyd.bin", "X22", "X22", "S21", b"right"),
        ]);
        let miner = MinerTypeInfo {
            model: "S21".to_string(),
            subtype: "X22".to_string(),
        };

        let resolved =
            resolve_firmware_image_inner("bundle.bmu".to_string(), bmu, &miner, 0).unwrap();

        assert_eq!(resolved.filename, "s21-hyd.bin");
        assert_eq!(resolved.bytes, b"right");
    }

    #[test]
    fn bmu_crc_mismatch_returns_error() {
        let mut bmu = build_bmu(&[("s21.bin", "X21", "X21", "S21", b"payload")]);
        bmu[24] ^= 0xFF;

        let err = parse_bmu_entries(&bmu).unwrap_err();

        assert!(err.to_string().contains("BMU CRC mismatch"));
    }

    #[test]
    fn bmu_truncated_payload_returns_error() {
        let mut bmu = build_bmu(&[("s21.bin", "X21", "X21", "S21", b"payload")]);
        bmu.truncate(bmu.len() - 2);

        let err = parse_bmu_entries(&bmu).unwrap_err();

        assert!(err.to_string().contains("BMU CRC mismatch"));

        let mut bmu = build_bmu(&[("s21.bin", "X21", "X21", "S21", b"payload")]);
        let payload_offset = BMU_HEADER_SIZE + BMU_ITEM_FIXED_SIZE;
        bmu.truncate(payload_offset + 3);

        let mut hasher = Hasher::new();
        hasher.update(&bmu[..24]);
        hasher.update(&[0, 0, 0, 0]);
        hasher.update(&bmu[28..]);
        write_u32_le(&mut bmu, 24, hasher.finalize());

        let err = parse_bmu_entries(&bmu).unwrap_err();

        assert!(err.to_string().contains("BMU payload exceeds file size"));
    }
}
