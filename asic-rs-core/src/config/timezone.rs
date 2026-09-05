//! Timezone configuration and the conversions between the canonical IANA
//! representation and the dialects the individual firmwares store.

use chrono::{DateTime, Datelike, FixedOffset, Offset, TimeZone, Utc};
pub use chrono_tz::Tz;
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(from_py_object, get_all, module = "asic_rs")
)]
#[cfg_attr(feature = "python", asic_rs_pydantic::py_pydantic_model)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Timezone configuration.
///
/// The zone is a [`Tz`] on every firmware — `Europe/Vienna`, `Etc/GMT-2` — and
/// each backend translates to and from whatever it stores: BraiinsOS speaks IANA
/// natively; VNish keeps a fixed `GMT±N` offset, which maps onto the `Etc/GMT*`
/// zones (see [`vnish_offset_to_tz`]) and therefore only accepts those.
///
/// Over the wire (serde, `model_dump`) the zones are their IANA names. From
/// Python the fields are `zoneinfo.ZoneInfo` objects; the constructor and the
/// pydantic validator take either a `ZoneInfo` or an IANA name.
pub struct TimezoneConfig {
    /// The configured timezone.
    pub timezone: Option<Tz>,
    /// The timezones the miner accepts.
    pub available: Vec<Tz>,
}

#[cfg(feature = "python")]
#[pymethods]
impl TimezoneConfig {
    #[new]
    #[pyo3(signature = (timezone: "tzinfo | str | None" = None, available: "list[tzinfo | str] | None" = None))]
    fn py_new(
        timezone: Option<&Bound<'_, PyAny>>,
        available: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use asic_rs_pydantic::PyPydanticType;
        Ok(Self {
            timezone: timezone
                .map(<Tz as PyPydanticType>::from_pydantic)
                .transpose()?,
            available: available
                .map(<Vec<Tz> as PyPydanticType>::from_pydantic)
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

/// Whole hours *east* of UTC encoded by a fixed-offset zone, or `None` for any
/// zone that carries transition rules.
///
/// The `Etc/GMT*` zones follow the POSIX convention, which inverts the sign
/// most people expect: `Etc/GMT+2` is two hours *behind* UTC (UTC−2) and
/// `Etc/GMT-2` is two hours *ahead* (UTC+2). `Etc/GMT`, `Etc/GMT0`,
/// `Etc/GMT+0`, `Etc/GMT-0`, `Etc/UTC` and `UTC` are all zero.
pub fn fixed_offset_hours(tz: Tz) -> Option<i32> {
    if matches!(tz, Tz::UTC | Tz::Etc__UTC) {
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
pub fn tz_from_offset_hours(hours: i32) -> anyhow::Result<Tz> {
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
pub fn vnish_offset_to_tz(value: &str) -> anyhow::Result<Tz> {
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
pub fn tz_to_vnish_offset(tz: Tz, now: DateTime<Utc>) -> anyhow::Result<String> {
    if let Some(hours) = fixed_offset_hours(tz) {
        return Ok(format!("GMT{hours:+}"));
    }

    let current = utc_offset_at(tz, now);
    // Sample mid-winter and mid-summer to tell a DST zone from a plain named one.
    let january = utc_offset_at(tz, noon_utc(now.year(), 1).unwrap_or(now));
    let july = utc_offset_at(tz, noon_utc(now.year(), 7).unwrap_or(now));

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
    match etc_gmt_equivalent(current) {
        Some(etc) => anyhow::bail!(
            "Timezone {tz} is not a fixed-offset Etc/GMT zone, which is all VNish can store. \
             Use {etc} (UTC{current}) instead."
        ),
        None => anyhow::bail!(
            "Timezone {tz} has a UTC{current} offset, which VNish cannot store \
             (whole hours between UTC-12 and UTC+14 only)."
        ),
    }
}

/// [`tz_to_vnish_offset`] evaluated at the current instant.
pub fn tz_to_vnish_offset_now(tz: Tz) -> anyhow::Result<String> {
    tz_to_vnish_offset(tz, Utc::now())
}

fn utc_offset_at(tz: Tz, at: DateTime<Utc>) -> FixedOffset {
    tz.offset_from_utc_datetime(&at.naive_utc()).fix()
}

fn noon_utc(year: i32, month: u32) -> Option<DateTime<Utc>> {
    Utc.with_ymd_and_hms(year, month, 1, 12, 0, 0).single()
}

/// The `Etc/GMT*` zone with exactly this offset, if it is a whole hour in range.
fn etc_gmt_equivalent(offset: FixedOffset) -> Option<Tz> {
    let seconds = offset.local_minus_utc();
    if seconds % 3600 != 0 {
        return None;
    }
    tz_from_offset_hours(seconds / 3600).ok()
}

fn describe_offset(offset: FixedOffset) -> String {
    match etc_gmt_equivalent(offset) {
        Some(etc) => format!("{etc} (UTC{offset})"),
        None => format!("UTC{offset} (no Etc/GMT zone)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: u32) -> anyhow::Result<DateTime<Utc>> {
        noon_utc(year, month).ok_or_else(|| anyhow::anyhow!("invalid date"))
    }

    /// VNish `GMT+2` means UTC+2, which is `Etc/GMT-2` in POSIX terms.
    #[test]
    fn vnish_positive_offset_maps_to_etc_gmt_minus() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT+2")?, Tz::Etc__GMTMinus2);
        assert_eq!(vnish_offset_to_tz("GMT+14")?, Tz::Etc__GMTMinus14);
        Ok(())
    }

    /// VNish `GMT-5` means UTC-5, which is `Etc/GMT+5` in POSIX terms.
    #[test]
    fn vnish_negative_offset_maps_to_etc_gmt_plus() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT-5")?, Tz::Etc__GMTPlus5);
        assert_eq!(vnish_offset_to_tz("GMT-12")?, Tz::Etc__GMTPlus12);
        Ok(())
    }

    #[test]
    fn vnish_zero_offset_maps_to_etc_gmt() -> anyhow::Result<()> {
        assert_eq!(vnish_offset_to_tz("GMT+0")?, Tz::Etc__GMT);
        assert_eq!(vnish_offset_to_tz("GMT-0")?, Tz::Etc__GMT);
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
        assert_eq!(tz_to_vnish_offset(Tz::Etc__GMTMinus2, now)?, "GMT+2");
        assert_eq!(tz_to_vnish_offset(Tz::Etc__GMTMinus14, now)?, "GMT+14");
        Ok(())
    }

    /// `Etc/GMT+5` is UTC-5, which VNish spells `GMT-5`.
    #[test]
    fn etc_gmt_plus_renders_as_vnish_negative() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        assert_eq!(tz_to_vnish_offset(Tz::Etc__GMTPlus5, now)?, "GMT-5");
        assert_eq!(tz_to_vnish_offset(Tz::Etc__GMTPlus12, now)?, "GMT-12");
        Ok(())
    }

    #[test]
    fn zero_offset_aliases_render_as_vnish_gmt_plus_zero() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        for tz in [
            Tz::Etc__GMT,
            Tz::Etc__GMT0,
            Tz::Etc__GMTPlus0,
            Tz::Etc__GMTMinus0,
            Tz::Etc__UTC,
            Tz::UTC,
        ] {
            assert_eq!(tz_to_vnish_offset(tz, now)?, "GMT+0", "{tz}");
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
            assert_eq!(fixed_offset_hours(tz), Some(hours), "{vnish}");
            assert_eq!(tz_to_vnish_offset(tz, now)?, vnish);
            assert_eq!(tz_from_offset_hours(hours)?, tz);
        }
        Ok(())
    }

    /// The sign inversion checked against the actual offsets chrono-tz applies,
    /// so a consistent-but-wrong mapping cannot pass the name-based tests above.
    #[test]
    fn mapped_zone_has_the_offset_vnish_means() -> anyhow::Result<()> {
        let now = at(2026, 7)?;
        let plus_two = vnish_offset_to_tz("GMT+2")?;
        let minus_five = vnish_offset_to_tz("GMT-5")?;
        assert_eq!(utc_offset_at(plus_two, now).local_minus_utc(), 2 * 3600);
        assert_eq!(utc_offset_at(minus_five, now).local_minus_utc(), -5 * 3600);
        Ok(())
    }

    /// A DST zone is refused, and the error names the Etc/GMT zone for the
    /// offset in effect at that moment — summer and winter differ.
    #[test]
    fn dst_zone_is_rejected_with_the_current_etc_gmt_equivalent() -> anyhow::Result<()> {
        let summer = tz_to_vnish_offset(Tz::Europe__Vienna, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Europe/Vienna was accepted"))?;
        assert!(summer.contains("daylight saving"), "{summer}");
        assert!(summer.contains("Use Etc/GMT-2 (UTC+02:00)"), "{summer}");
        assert!(summer.contains("Etc/GMT-1 (UTC+01:00)"), "{summer}");

        let winter = tz_to_vnish_offset(Tz::Europe__Vienna, at(2026, 1)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Europe/Vienna was accepted"))?;
        assert!(winter.contains("Use Etc/GMT-1 (UTC+01:00)"), "{winter}");

        // Western hemisphere: UTC-4 in summer is Etc/GMT+4.
        let new_york = tz_to_vnish_offset(Tz::America__New_York, at(2026, 7)?)
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
        let tokyo = tz_to_vnish_offset(Tz::Asia__Tokyo, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Asia/Tokyo was accepted"))?;
        assert!(!tokyo.contains("daylight saving"), "{tokyo}");
        assert!(tokyo.contains("Use Etc/GMT-9 (UTC+09:00)"), "{tokyo}");
        Ok(())
    }

    #[test]
    fn half_hour_zone_has_no_vnish_equivalent() -> anyhow::Result<()> {
        let kolkata = tz_to_vnish_offset(Tz::Asia__Kolkata, at(2026, 7)?)
            .err()
            .map(|e| e.to_string())
            .ok_or_else(|| anyhow::anyhow!("Asia/Kolkata was accepted"))?;
        assert!(kolkata.contains("UTC+05:30"), "{kolkata}");
        assert!(!kolkata.contains("Use Etc/GMT"), "{kolkata}");
        Ok(())
    }

    /// On the wire the zones are their IANA names, and an unknown name is
    /// rejected on the way in rather than carried along as text.
    #[test]
    fn config_serializes_zones_as_iana_names() -> anyhow::Result<()> {
        let config = TimezoneConfig {
            timezone: Some(Tz::Europe__Vienna),
            available: vec![Tz::Europe__Vienna, Tz::Etc__GMTMinus2],
        };
        let json = serde_json::to_value(&config)?;
        assert_eq!(
            json,
            serde_json::json!({
                "timezone": "Europe/Vienna",
                "available": ["Europe/Vienna", "Etc/GMT-2"],
            })
        );

        let back: TimezoneConfig = serde_json::from_value(json)?;
        assert_eq!(back.timezone, Some(Tz::Europe__Vienna));
        assert_eq!(back.available, config.available);

        let empty: TimezoneConfig = serde_json::from_str(r#"{"timezone":null,"available":[]}"#)?;
        assert_eq!(empty.timezone, None);

        let bogus: Result<TimezoneConfig, _> =
            serde_json::from_str(r#"{"timezone":"GMT+2","available":[]}"#);
        assert!(bogus.is_err());
        Ok(())
    }
}
