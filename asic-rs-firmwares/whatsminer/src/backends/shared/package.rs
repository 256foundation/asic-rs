use anyhow::{Context, Result, ensure};
use asic_rs_core::data::firmware::FirmwareImage;
use sha2::{Digest, Sha256};

pub const MAX_PACKAGE_SIZE: usize = 128 * 1024 * 1024;
const CONTAINER_BYTES: usize = 6812;
const MEMBER_HEADER_BYTES: usize = 184;
const LEGACY_HEADER_BYTES: usize = 32;
const CONTAINER_MAGIC: u32 = 0xae640a3f;
const ENCRYPTED_MAGIC: u32 = 0x9a5ba57c;
const MINER_MAGIC: u32 = 0x8c790afe;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirmwareRepresentation {
    Legacy,
    Sim,
    War,
}

#[derive(Clone, Debug)]
pub struct FirmwareImageSpan {
    pub representation: FirmwareRepresentation,
    pub offset: usize,
    pub length: usize,
    pub payload_size: usize,
    pub created_at: u32,
    pub expiry_hours: u32,
    pub sha256: String,
}

#[derive(Clone, Debug)]
pub struct FirmwareMember {
    pub key: String,
    pub offset: usize,
    pub length: usize,
    pub payload_size: usize,
    pub created_at: u32,
    pub expiry_hours: u32,
    pub sha256: String,
    pub images: Vec<FirmwareImageSpan>,
}

#[derive(Clone, Debug)]
pub struct LegacyHeader {
    pub created_at: u32,
    pub payload_size: usize,
}

#[derive(Clone, Debug)]
pub struct FirmwareInspection {
    pub legacy: Option<LegacyHeader>,
    pub sha256: String,
    pub size: usize,
    pub members: Vec<FirmwareMember>,
    pub images: Vec<FirmwareImageSpan>,
}

pub(crate) struct Package {
    bytes: Vec<u8>,
    inspection: FirmwareInspection,
}

impl Package {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let inspection = inspect(&bytes)?;
        ensure!(
            inspection.legacy.is_none(),
            "legacy images require a verified receiver and representation selection before upload"
        );
        ensure!(
            !inspection.members.is_empty(),
            "standalone firmware requires verified target compatibility before upload; use a combined bundle"
        );
        Ok(Self { bytes, inspection })
    }
}

fn u32_at(data: &[u8], start: usize) -> Result<u32> {
    let end = start.checked_add(4).context("firmware field overflow")?;
    let field = data.get(start..end).context("truncated firmware field")?;
    Ok(u32::from_le_bytes(field.try_into()?))
}

fn check_header(header: &[u8]) -> Result<()> {
    let end = header
        .len()
        .checked_sub(4)
        .context("truncated firmware header")?;
    ensure!(
        u32_at(header, end)? == !crc32fast::hash(&header[..end]),
        "firmware header CRC mismatch"
    );
    Ok(())
}

fn selector(field: &[u8]) -> Result<&str> {
    let end = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len());
    ensure!(
        field[end..].iter().all(|&byte| byte == 0),
        "invalid selector padding"
    );
    ensure!(
        end < field.len() && field[..end].iter().all(u8::is_ascii_graphic),
        "invalid firmware selector"
    );
    Ok(std::str::from_utf8(&field[..end])?)
}

fn member_range(descriptor: &[u8], bundle_len: usize) -> Result<std::ops::Range<usize>> {
    let offset = u32_at(descriptor, 32)? as usize;
    let length = u32_at(descriptor, 36)? as usize;
    let end = offset
        .checked_add(length)
        .context("firmware member range overflow")?;
    ensure!(
        offset >= CONTAINER_BYTES && length >= MEMBER_HEADER_BYTES,
        "member overlaps container header or is truncated"
    );
    ensure!(end <= bundle_len, "firmware member exceeds bundle bounds");
    Ok(offset..end)
}

fn encrypted_image(data: &[u8], offset: usize, limit: usize) -> Result<FirmwareImageSpan> {
    let header_end = offset
        .checked_add(MEMBER_HEADER_BYTES)
        .context("firmware header range overflow")?;
    ensure!(header_end <= limit, "truncated encrypted firmware header");
    let header = data
        .get(offset..header_end)
        .context("truncated encrypted firmware header")?;
    check_header(header)?;
    ensure!(
        u32_at(header, 0)? == ENCRYPTED_MAGIC && u32_at(header, 4)? == MINER_MAGIC,
        "only encrypted miner packages are supported; power packages are unsupported"
    );
    ensure!(
        u32_at(header, 8)? & 0x07ff_ffff < 2,
        "unsupported member version"
    );
    let representation = match u32_at(header, 56)? {
        0 => FirmwareRepresentation::Sim,
        1 => FirmwareRepresentation::War,
        _ => anyhow::bail!("unsupported firmware representation"),
    };
    let algorithm = u32_at(header, 124)?;
    ensure!(matches!(algorithm, 0 | 1), "unsupported package encoding");
    let encrypted_size = u32_at(header, 32)? as usize;
    let end = header_end
        .checked_add(encrypted_size)
        .context("firmware image range overflow")?;
    ensure!(end <= limit, "encrypted firmware image exceeds span bounds");
    let payload = data
        .get(header_end..end)
        .context("truncated encrypted firmware payload")?;
    let payload_size = u32_at(header, 28)? as usize;
    ensure!(
        payload_size > 0
            && payload_size <= payload.len()
            && (algorithm == 0 || payload.len().is_multiple_of(16)),
        "invalid encrypted payload length"
    );
    ensure!(
        header[36..52] == md5::compute(payload).0,
        "encrypted payload MD5 mismatch"
    );
    Ok(FirmwareImageSpan {
        representation,
        offset,
        length: end - offset,
        payload_size,
        created_at: u32_at(header, 12)?,
        expiry_hours: u32_at(header, 16)?,
        sha256: hex::encode(Sha256::digest(&data[offset..end])),
    })
}

fn encrypted_images(data: &[u8], offset: usize, limit: usize) -> Result<Vec<FirmwareImageSpan>> {
    let first = encrypted_image(data, offset, limit)?;
    let next = first.offset + first.length;
    let mut images = vec![first];
    if next < limit {
        ensure!(
            images[0].representation == FirmwareRepresentation::Sim,
            "only a Sim image may precede another encrypted image"
        );
        let second = encrypted_image(data, next, limit)?;
        ensure!(
            second.representation == FirmwareRepresentation::War,
            "a Sim image must be followed by a War image"
        );
        ensure!(
            u32_at(data, offset + 20)? == u32_at(data, next + 20)?,
            "incompatible chip types in firmware image pair"
        );
        ensure!(
            second.offset + second.length == limit,
            "unexpected trailing firmware data"
        );
        images.push(second);
    }
    Ok(images)
}

fn standalone_images(data: &[u8]) -> Result<Vec<FirmwareImageSpan>> {
    if u32_at(data, 0)? == ENCRYPTED_MAGIC {
        return encrypted_images(data, 0, data.len());
    }
    ensure!(
        u32_at(data, 0)? == MINER_MAGIC,
        "unrecognized firmware format"
    );
    let header = data
        .get(..LEGACY_HEADER_BYTES)
        .context("truncated legacy firmware header")?;
    let payload_size = u32_at(header, 8)? as usize;
    let end = LEGACY_HEADER_BYTES
        .checked_add(payload_size)
        .context("legacy firmware range overflow")?;
    ensure!(payload_size > 0, "invalid legacy image length");
    let payload = data
        .get(LEGACY_HEADER_BYTES..end)
        .context("invalid legacy image length")?;
    check_header(header)?;
    ensure!(
        u32_at(header, 12)? == !crc32fast::hash(payload),
        "legacy payload CRC mismatch"
    );
    let mut images = vec![FirmwareImageSpan {
        representation: FirmwareRepresentation::Legacy,
        offset: 0,
        length: end,
        payload_size,
        created_at: u32_at(header, 4)?,
        expiry_hours: 0,
        sha256: hex::encode(Sha256::digest(&data[..end])),
    }];
    if end < data.len() {
        let suffix = encrypted_image(data, end, data.len())?;
        ensure!(
            suffix.representation == FirmwareRepresentation::Sim,
            "a legacy image must be followed by a Sim image"
        );
        ensure!(
            suffix.offset + suffix.length == data.len(),
            "unexpected trailing firmware data"
        );
        images.push(suffix);
    }
    Ok(images)
}

fn member(data: &[u8], descriptor: &[u8]) -> Result<FirmwareMember> {
    let range = member_range(descriptor, data.len())?;
    let descriptor_version = u32_at(descriptor, 40)?;
    ensure!(descriptor_version <= 3, "unsupported member version");
    let chip = selector(&descriptor[..16])?;
    let chip = if chip.is_empty() { "0" } else { chip };
    let platform = selector(&descriptor[16..32])?.to_ascii_lowercase();
    ensure!(!platform.is_empty(), "missing descriptor platform");
    let images = encrypted_images(data, range.start, range.end)?;
    let first = &images[0];
    let is_btrom = chip == "1011200000" || u32_at(data, range.start + 20)? == 1_011_200_000;
    let mut key = format!("{chip}-{platform}");
    if descriptor_version >= 2 {
        key.push_str(&format!("v{descriptor_version}"));
    }
    if is_btrom {
        key.push_str("-btrom");
    }
    Ok(FirmwareMember {
        key,
        offset: range.start,
        length: range.len(),
        payload_size: first.payload_size,
        created_at: first.created_at,
        expiry_hours: first.expiry_hours,
        sha256: hex::encode(Sha256::digest(&data[range])),
        images,
    })
}

pub(super) fn inspect(data: &[u8]) -> Result<FirmwareInspection> {
    ensure!(data.len() <= MAX_PACKAGE_SIZE, "firmware exceeds 128 MiB");
    if u32_at(data, 0)? != CONTAINER_MAGIC {
        let images = standalone_images(data)?;
        let legacy = images
            .first()
            .filter(|image| image.representation == FirmwareRepresentation::Legacy)
            .map(|image| LegacyHeader {
                created_at: image.created_at,
                payload_size: image.payload_size,
            });
        return Ok(FirmwareInspection {
            legacy,
            members: Vec::new(),
            images,
            size: data.len(),
            sha256: hex::encode(Sha256::digest(data)),
        });
    }
    ensure!(
        (CONTAINER_BYTES..=MAX_PACKAGE_SIZE).contains(&data.len()),
        "combined firmware must be between 6812 bytes and 128 MiB"
    );
    check_header(&data[..CONTAINER_BYTES])?;
    ensure!(
        u32_at(data, 16)? as usize == data.len() - CONTAINER_BYTES,
        "combined payload length mismatch"
    );
    let count = u32_at(data, 20)? as usize;
    ensure!(
        (1..=64).contains(&count),
        "invalid firmware descriptor count"
    );
    let (descriptors, _) = data[24..24 + count * 104].as_chunks::<104>();
    let mut ranges = Vec::with_capacity(count);
    for descriptor in descriptors {
        let range = member_range(descriptor, data.len())?;
        ranges.push((range.start, range.end));
    }
    // Sort only the ranges: member selection must retain descriptor order.
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        ensure!(pair[0].1 <= pair[1].0, "overlapping firmware members");
    }
    let members = descriptors
        .iter()
        .map(|descriptor| member(data, descriptor))
        .collect::<Result<Vec<_>>>()?;
    Ok(FirmwareInspection {
        legacy: None,
        sha256: hex::encode(Sha256::digest(data)),
        size: data.len(),
        members,
        images: Vec::new(),
    })
}

pub(crate) fn platform_version(platform: &str, board_version: &str, firmware: &str) -> Option<u8> {
    let composite = format!("{platform}-{board_version}-{firmware}");
    let token = composite.split('.').nth(2).unwrap_or_default();
    match token.to_ascii_lowercase().as_str() {
        "rel3" | "v3" => Some(3),
        "rel2" | "v2" => Some(2),
        _ if composite.contains("NeedUpgradeAgain1") || composite.contains("NeedUpgradeAgain2") => {
            Some(2)
        }
        _ => None,
    }
}

fn candidate_keys(
    platform: &str,
    chip: &str,
    platform_version: Option<u8>,
    is_btrom: bool,
) -> Result<Vec<String>> {
    if platform == "h616" {
        let (prefix, suffix) = if is_btrom {
            ("1011200000", "-btrom")
        } else {
            ("0", "")
        };
        let generation = platform_version.unwrap_or(2);
        let mut keys = vec![format!("{prefix}-h616v{generation}{suffix}")];
        if platform_version.is_none() {
            keys.push(format!("{prefix}-h616{suffix}"));
        }
        return Ok(keys);
    }
    ensure!(
        matches!(platform, "h3" | "h6" | "h6os"),
        "unsupported combined firmware platform {platform}"
    );
    let (prefix, suffix) = if is_btrom {
        ("all", "-btrom")
    } else {
        (chip, "")
    };
    let mut keys = vec![format!("{prefix}-{platform}{suffix}")];
    let generic_platform = if platform == "h6" {
        keys.push(format!("{prefix}h6os"));
        "h6os"
    } else {
        platform
    };
    keys.push(format!("0-{generic_platform}"));
    Ok(keys)
}

pub(crate) struct Device {
    pub chip: String,
    pub platform_version: Option<u8>,
    pub platform: String,
    pub is_btrom: bool,
    pub clock: u64,
}

impl Package {
    pub(crate) fn select(&self, filename: String, device: Device) -> Result<FirmwareImage> {
        let keys = candidate_keys(
            &device.platform,
            &device.chip,
            device.platform_version,
            device.is_btrom,
        )?;
        let member = keys
            .iter()
            .find_map(|key| self.inspection.members.iter().find(|m| &m.key == key))
            .with_context(|| format!("no compatible member; attempted {}", keys.join(", ")))?;
        ensure!(
            member.images.len() == 1,
            "multi-image firmware member requires verified receiver representation selection before upload"
        );
        if member.expiry_hours != 0 {
            ensure!(
                device.clock.abs_diff(u64::from(member.created_at))
                    <= u64::from(member.expiry_hours) * 3600,
                "firmware is outside its validity window according to the miner clock"
            );
        }
        Ok(FirmwareImage::new(
            filename,
            self.bytes[member.offset..member.offset + member.length].to_vec(),
        ))
    }
}
