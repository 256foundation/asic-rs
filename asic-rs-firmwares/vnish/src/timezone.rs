//! VNish fixed-offset timezone conversions and validation.

use asic_rs_core::config::timezone::Tz;
use jiff::{Timestamp, civil::DateTime, tz::Offset};

/// Whole hours *east* of UTC encoded by a fixed-offset zone, or `None` for any
/// zone that carries transition rules.
///
/// The `Etc/GMT*` zones follow the POSIX convention, which inverts the sign
/// most people expect: `Etc/GMT+2` is two hours *behind* UTC (UTC−2) and
/// `Etc/GMT-2` is two hours *ahead* (UTC+2). `Etc/GMT`, `Etc/GMT0`,
/// `Etc/GMT+0`, `Etc/GMT-0`, `Etc/UTC` and `UTC` are all zero.
fn fixed_offset_hours(tz: &Tz) -> Option<i32> {
    if matches!(tz.name(), "UTC" | "Etc/UTC") {
        return Some(0);
    }
    let posix_suffix = tz.name().strip_prefix("Etc/GMT")?;
    if posix_suffix.is_empty() {
        return Some(0);
    }
    // "+2" -> 2 (UTC-2), "-2" -> -2 (UTC+2): negate to get hours east of UTC.
    posix_suffix
        .parse::<i32>()
        .ok()
        .map(|posix_hours| -posix_hours)
}

/// The `Etc/GMT*` zone for a whole-hour offset given in hours *east* of UTC.
///
/// UTC+2 → `Etc/GMT-2`, UTC−5 → `Etc/GMT+5`, 0 → `Etc/GMT`. Errors outside the
/// range the IANA database defines (UTC−12 … UTC+14).
fn tz_from_offset_hours(hours: i32) -> anyhow::Result<Tz> {
    let name = match hours {
        0 => "Etc/GMT".to_string(),
        east if east > 0 => format!("Etc/GMT-{east}"),
        west => format!("Etc/GMT+{}", -west),
    };
    name.parse::<Tz>()
        .map_err(|_| anyhow::anyhow!("No Etc/GMT zone for a UTC{hours:+} hour offset"))
}

/// Parse VNish's `"GMT±N"` offset into the canonical `Etc/GMT∓N` zone.
///
/// VNish uses the everyday sign (`GMT+2` is UTC+2), the IANA `Etc/GMT*` zones
/// use the POSIX sign (`Etc/GMT-2` is UTC+2), so the sign flips on the way in.
pub(crate) fn vnish_offset_to_tz(value: &str) -> anyhow::Result<Tz> {
    let hours = value
        .strip_prefix("GMT")
        .and_then(|offset| offset.parse::<i32>().ok())
        .ok_or_else(|| {
            anyhow::anyhow!("Unrecognised VNish timezone {value:?}, expected \"GMT+N\"/\"GMT-N\"")
        })?;
    tz_from_offset_hours(hours)
}

/// Render a zone in VNish's `"GMT±N"` dialect (`GMT+2` is UTC+2).
///
/// VNish stores one fixed UTC offset and has no DST rules, so only the
/// fixed-offset zones (`Etc/GMT*`, `UTC`) can be represented losslessly. Any
/// other zone is rejected rather than pinned to whatever offset applies today:
/// a `Europe/Vienna` pinned in summer would be an hour off all winter. The
/// error names the `Etc/GMT*` zone for the offset in effect at `now` so the
/// caller can pick it explicitly.
fn tz_to_vnish_offset(tz: &Tz, now: Timestamp) -> anyhow::Result<String> {
    if let Some(hours) = fixed_offset_hours(tz) {
        return Ok(format!("GMT{hours:+}"));
    }

    let current = tz.offset_at(now);
    // Sample mid-winter and mid-summer to tell a DST zone from a plain named one.
    let year = Offset::UTC.to_datetime(now).year();
    let january = tz.offset_at(noon_utc(year, 1).unwrap_or(now));
    let july = tz.offset_at(noon_utc(year, 7).unwrap_or(now));

    if january != july {
        anyhow::bail!(
            "Timezone {tz} observes daylight saving time, which VNish cannot follow \
             (it stores one fixed UTC offset). Use {} for the offset in effect now; \
             over the year the zone alternates between {} and {}.",
            describe_offset(current),
            describe_offset(january),
            describe_offset(july)
        );
    }
    let description = display_offset(current);
    match etc_gmt_equivalent(current) {
        Some(etc) => anyhow::bail!(
            "Timezone {tz} is not a fixed-offset Etc/GMT zone, which is all VNish can store. \
             Use {etc} (UTC{description}) instead."
        ),
        None => anyhow::bail!(
            "Timezone {tz} has a UTC{description} offset, which VNish cannot store \
             (whole hours between UTC-12 and UTC+14 only)."
        ),
    }
}

/// [`tz_to_vnish_offset`] evaluated at the current instant.
pub(crate) fn tz_to_vnish_offset_now(tz: &Tz) -> anyhow::Result<String> {
    tz_to_vnish_offset(tz, Timestamp::now())
}

fn noon_utc(year: i16, month: i8) -> Option<Timestamp> {
    let date = DateTime::new(year, month, 1, 12, 0, 0, 0).ok()?;
    Offset::UTC.to_timestamp(date).ok()
}

/// The `Etc/GMT*` zone with exactly this offset, if it is a whole hour in range.
fn etc_gmt_equivalent(offset: Offset) -> Option<Tz> {
    let seconds = offset.seconds();
    if seconds % 3600 != 0 {
        return None;
    }
    tz_from_offset_hours(seconds / 3600).ok()
}

fn describe_offset(offset: Offset) -> String {
    let description = display_offset(offset);
    match etc_gmt_equivalent(offset) {
        Some(etc) => format!("{etc} (UTC{description})"),
        None => format!("UTC{description} (no Etc/GMT zone)"),
    }
}

fn display_offset(offset: Offset) -> String {
    if offset.seconds() % 3600 == 0 {
        format!("{offset}:00")
    } else {
        offset.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i16, month: i8) -> anyhow::Result<Timestamp> {
        noon_utc(year, month).ok_or_else(|| anyhow::anyhow!("invalid date"))
    }

    /// VNish `GMT+2` means UTC+2, which is `Etc/GMT-2` in POSIX terms.
    #[test]
    fn vnish_positive_offset_maps_to_etc_gmt_minus() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT+2")?.name(), "Etc/GMT-2");
        assert_eq!(vnish_offset_to_tz("GMT+14")?.name(), "Etc/GMT-14");
        Ok(())
    }

    /// VNish `GMT-5` means UTC-5, which is `Etc/GMT+5` in POSIX terms.
    #[test]
    fn vnish_negative_offset_maps_to_etc_gmt_plus() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT-5")?.name(), "Etc/GMT+5");
        assert_eq!(vnish_offset_to_tz("GMT-12")?.name(), "Etc/GMT+12");
        Ok(())
    }

    #[test]
    fn vnish_zero_offset_maps_to_etc_gmt() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT+0")?.name(), "Etc/GMT");
        assert_eq!(vnish_offset_to_tz("GMT-0")?.name(), "Etc/GMT");
        Ok(())
    }

    #[test]
    fn vnish_garbage_is_rejected() {
        assert!(vnish_offset_to_tz("Europe/Vienna").is_err());
        assert!(vnish_offset_to_tz("UTC+2").is_err());
        assert!(vnish_offset_to_tz("GMT+15").is_err());
        assert!(vnish_offset_to_tz("GMT-13").is_err());
    }

    /// `Etc/GMT-2` is UTC+2, which VNish spells `GMT+2`.
    #[test]
    fn etc_gmt_minus_renders_as_vnish_positive() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        assert_eq!(tz_to_vnish_offset(&"Etc/GMT-2".parse()?, now)?, "GMT+2");
        assert_eq!(tz_to_vnish_offset(&"Etc/GMT-14".parse()?, now)?, "GMT+14");
        Ok(())
    }

    /// `Etc/GMT+5` is UTC-5, which VNish spells `GMT-5`.
    #[test]
    fn etc_gmt_plus_renders_as_vnish_negative() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        assert_eq!(tz_to_vnish_offset(&"Etc/GMT+5".parse()?, now)?, "GMT-5");
        assert_eq!(tz_to_vnish_offset(&"Etc/GMT+12".parse()?, now)?, "GMT-12");
        Ok(())
    }

    #[test]
    fn zero_offset_aliases_render_as_vnish_gmt_plus_zero() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        for name in [
            "Etc/GMT",
            "Etc/GMT0",
            "Etc/GMT+0",
            "Etc/GMT-0",
            "Etc/UTC",
            "UTC",
        ] {
            assert_eq!(tz_to_vnish_offset(&name.parse()?, now)?, "GMT+0", "{name}");
        }
        Ok(())
    }

    /// The whole VNish range survives a round trip in both directions.
    #[test]
    fn vnish_range_round_trips() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        for hours in -12..=14 {
            let vnish = format!("GMT{hours:+}");
            let tz = vnish_offset_to_tz(&vnish)?;
            assert_eq!(fixed_offset_hours(&tz), Some(hours), "{vnish}");
            assert_eq!(tz_to_vnish_offset(&tz, now)?, vnish);
            assert_eq!(tz_from_offset_hours(hours)?, tz);
        }
        Ok(())
    }

    /// The sign inversion checked against the actual offsets in the bundled timezone database,
    /// so a consistent-but-wrong mapping cannot pass the name-based tests above.
    #[test]
    fn mapped_zone_has_the_offset_vnish_means() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        let plus_two = vnish_offset_to_tz("GMT+2")?;
        let minus_five = vnish_offset_to_tz("GMT-5")?;
        assert_eq!(plus_two.offset_at(now).seconds(), 2 * 3600);
        assert_eq!(minus_five.offset_at(now).seconds(), -5 * 3600);
        Ok(())
    }

    /// A DST zone is refused, and the error names the Etc/GMT zone for the
    /// offset in effect at that moment — summer and winter differ.
    #[test]
    fn dst_zone_is_rejected_with_the_current_etc_gmt_equivalent() -> anyhow::Result<()> {
        let summer = tz_to_vnish_offset(&"Europe/Vienna".parse()?, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Europe/Vienna was accepted"))?;
        assert!(summer.contains("daylight saving"), "{summer}");
        assert!(summer.contains("Use Etc/GMT-2 (UTC+02:00)"), "{summer}");
        assert!(summer.contains("Etc/GMT-1 (UTC+01:00)"), "{summer}");

        let winter = tz_to_vnish_offset(&"Europe/Vienna".parse()?, at(2026, 1)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Europe/Vienna was accepted"))?;
        assert!(winter.contains("Use Etc/GMT-1 (UTC+01:00)"), "{winter}");

        // Western hemisphere: UTC-4 in summer is Etc/GMT+4.
        let new_york = tz_to_vnish_offset(&"America/New_York".parse()?, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("America/New_York was accepted"))?;
        assert!(new_york.contains("Use Etc/GMT+4 (UTC-04:00)"), "{new_york}");
        Ok(())
    }

    /// A named zone without DST is still not an Etc/GMT zone; the error points
    /// at the equivalent instead of silently substituting it.
    #[test]
    fn fixed_named_zone_is_rejected_with_its_equivalent() -> anyhow::Result<()> {
        let tokyo = tz_to_vnish_offset(&"Asia/Tokyo".parse()?, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Asia/Tokyo was accepted"))?;
        assert!(!tokyo.contains("daylight saving"), "{tokyo}");
        assert!(tokyo.contains("Use Etc/GMT-9 (UTC+09:00)"), "{tokyo}");
        Ok(())
    }

    #[test]
    fn half_hour_zone_has_no_vnish_equivalent() -> anyhow::Result<()> {
        let kolkata = tz_to_vnish_offset(&"Asia/Kolkata".parse()?, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Asia/Kolkata was accepted"))?;
        assert!(kolkata.contains("UTC+05:30"), "{kolkata}");
        assert!(!kolkata.contains("Use Etc/GMT"), "{kolkata}");
        Ok(())
    }
}
