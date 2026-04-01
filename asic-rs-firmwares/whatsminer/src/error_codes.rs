const UNKNOWN: &str = "Unknown error type.";

/// Look up a human-readable description for a WhatsMiner error code.
///
/// Error codes are decoded by splitting the decimal digits into
/// `(err_type, err_subtype, err_value)`.  The splitting rule (matching
/// the WhatsMiner error code format) is:
///
/// * < 100 (fewer than 3 digits) → unknown
/// * 6-digit codes where the leading digit is **not** `1` →
///   type = first 2 digits, subtype = digit 3, value = last 3 digits
/// * Everything else →
///   type = all-but-last-2 digits, subtype = second-to-last digit,
///   value = last digit
pub fn error_message(code: u64) -> String {
    if code < 100 {
        return UNKNOWN.to_string();
    }

    let (err_type, err_subtype, err_value) =
        if code.checked_ilog10() == Some(5) && code / 100_000 != 1 {
            (code / 10_000, (code / 1_000) % 10, code % 1_000)
        } else {
            (code / 100, (code / 10) % 10, code % 10)
        };

    lookup(err_type, err_subtype, err_value)
}

/// Three-level lookup against the WhatsMiner error code table.
///
/// The table supports two kinds of wildcard entries:
///
/// 1. **Subtype-level `{n}`** – when `err_subtype` has no exact match the
///    "wildcard" arm is tried, substituting `err_value` for `{n}`.
/// 2. **Type-level `{n}{c}`** – when the *entire* subtype level is a
///    wildcard the template uses `err_subtype` → `{n}` and
///    `err_value` → `{c}`.
fn lookup(err_type: u64, err_subtype: u64, err_value: u64) -> String {
    match err_type {
        1 => fan_error(err_subtype, err_value),
        2 => power_error(err_subtype, err_value),
        3 => temperature_error(err_subtype, err_value),
        4 => eeprom_error(err_subtype, err_value),
        5 => hashboard_error(err_subtype, err_value),
        6 => env_temp_error(err_subtype, err_value),
        7 => control_board_error(err_subtype, err_value),
        8 => checksum_error(err_subtype, err_value),
        9 => power_rate_error(err_subtype, err_value),
        20 => pool_error(err_subtype, err_value),
        21 => factory_test_error(err_subtype, err_value),
        23 => hashrate_error(err_subtype, err_value),
        50 => voltage_water_error(err_subtype, err_value),
        51 => frequency_error(err_subtype, err_value),
        52 => slot_chip_template("error nonce", err_subtype, err_value),
        53 => slot_chip_template("too few nonce", err_subtype, err_value),
        54 => slot_chip_template("temp protected", err_subtype, err_value),
        55 => slot_chip_template("has been reset", err_subtype, err_value),
        56 => slot_chip_template("zero nonce", err_subtype, err_value),
        80 => tool_error(err_subtype, err_value),
        81 => match (err_subtype, err_value) {
            (0, 0) => "Chip data error.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        82 => match (err_subtype, err_value) {
            (0, 0) => "Power version error.".to_string(),
            (1, 0) => "Miner type error.".to_string(),
            (2, 0) => "Version info error.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        83 => match (err_subtype, err_value) {
            (0, 0) => "Empty level error.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        84 => match (err_subtype, err_value) {
            (0, 0) => "Old firmware.".to_string(),
            (1, 0) => "Software version error.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        85 => substandard_error(err_subtype, err_value),
        86 => serial_info_error(err_subtype, err_value),
        87 => match (err_subtype, err_value) {
            (0, 0) => "Miner power mismatch.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        90 => match (err_subtype, err_value) {
            (0, 0) | (1, 0) => "Process error, exited with signal: 3.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        99 => match (err_subtype, err_value) {
            (9, 9) => "Miner unknown error.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        1000 => match (err_subtype, err_value) {
            (0, 0) => "Security library error, please upgrade firmware".to_string(),
            (0, 1) => "/antiv/signature illegal.".to_string(),
            (0, 2) => "/antiv/dig/init.d illegal.".to_string(),
            (0, 3) => "/antiv/dig/pf_partial.dig illegal.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        1001 => match (err_subtype, err_value) {
            (0, 0) => "Security BTMiner removed, please upgrade firmware.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        1100 => match (err_subtype, err_value) {
            (0, 0) => "Security illegal file, please upgrade firmware.".to_string(),
            (0, 1) => "Security virus 0001 is removed, please upgrade firmware.".to_string(),
            _ => UNKNOWN.to_string(),
        },
        _ => UNKNOWN.to_string(),
    }
}

// ── type 1: Fan ──────────────────────────────────────────────────────

fn fan_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Fan unknown.".to_string(),
        (1, 0) => "Intake fan speed error.".to_string(),
        (1, 1) => "Exhaust fan speed error.".to_string(),
        (2, 0) => "Intake fan speed error.  Fan speed deviates by more than 2000.".to_string(),
        (2, 1) => "Exhaust fan speed error.  Fan speed deviates by more than 2000.".to_string(),
        (3, 0) => "Intake fan speed error.  Fan speed deviates by more than 3000.".to_string(),
        (3, 1) => "Exhaust fan speed error.  Fan speed deviates by more than 3000.".to_string(),
        (4, 0) => "Fan speed too high.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 2: Power ────────────────────────────────────────────────────

fn power_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Power probing error.  No power found.".to_string(),
        (0, 1) => "Power supply and configuration file don't match.".to_string(),
        (0, 2) => "Power output voltage error.".to_string(),
        (0, 3) => "Power protecting due to high environment temperature.".to_string(),
        (0, 4) => "Power current protecting due to high environment temperature.".to_string(),
        (0, 5) => "Power current error.".to_string(),
        (0, 6) => "Power input low voltage error.".to_string(),
        (0, 7) => "Power input current protecting due to bad power input.".to_string(),
        (0, 8) => "Power power error.".to_string(),
        (0, 9) => "Power voltage offset error.".to_string(),
        (1, 0) => "Power error.".to_string(),
        (1, 1) => "Power iout error, please reboot.".to_string(),
        (1, 2) => "Power vout error, reach vout border. Border: [1150, 1500]".to_string(),
        (1, 3) => "Power input voltage and current do not match power output.".to_string(),
        (1, 4) => "Power pin did not change.".to_string(),
        (1, 5) => "Power vout set error.".to_string(),
        (1, 6) => "Power remained unchanged for a long time.".to_string(),
        (1, 7) => "Power set enable error.".to_string(),
        (1, 8) => "Power input voltage is lower than 230V for high power mode.".to_string(),
        (1, 9) => "Power input current is incorrect.".to_string(),
        (3, 3..=5) => "Power output high temperature protection error.".to_string(),
        (3, 6..=8) => "Power output high current protection error.".to_string(),
        (3, 9) => "Power output high voltage protection error.".to_string(),
        (4, 0) => "Power output low voltage protection error.".to_string(),
        (4, 1) => "Power output current imbalance error.".to_string(),
        (4, 3..=5) => "Power input high temperature protection error.".to_string(),
        (4, 6 | 7) => "Power input high voltage protection error.".to_string(),
        (4, 8 | 9) => "Power input high current protection error.".to_string(),
        (5, 0 | 1) => "Power input low voltage protection error.".to_string(),
        (5, 3 | 4) => "Power supply fan error.".to_string(),
        (5, 5 | 6) => "Power output high power protection error.".to_string(),
        (5, 7) => "Input over current protection of power supply on primary side.".to_string(),
        (6, 3) => "Power communication warning.".to_string(),
        (6, 4) => "Power communication error.".to_string(),
        (6, 5 | 6) => "Power unknown error.".to_string(),
        (6, 7) => "Power watchdog protection.".to_string(),
        (6, 8) => "Power output high current protection.".to_string(),
        (6, 9) => "Power input high current protection.".to_string(),
        (7, 0) => "Power input high voltage protection.".to_string(),
        (7, 1) => "Power input low voltage protection.".to_string(),
        (7, 2) => "Excessive power supply output warning.".to_string(),
        (7, 3) => "Power input too high warning.".to_string(),
        (7, 4) => "Power fan warning.".to_string(),
        (7, 5) => "Power high temperature warning.".to_string(),
        (7, 6..=9) => "Power unknown error.".to_string(),
        (8, 0) => "Power unknown error.".to_string(),
        (8, 1) => "Power vendor status 1 bit 0 error.".to_string(),
        (8, 2) => "Power vendor status 1 bit 1 error.".to_string(),
        (8, 3) => "Power vendor status 1 bit 2 error.".to_string(),
        (8, 4) => "Power vendor status 1 bit 3 error.".to_string(),
        (8, 5) => "Power vendor status 1 bit 4 error.".to_string(),
        (8, 6) => "Power vendor status 1 bit 5 error.".to_string(),
        (8, 7) => "Power vendor status 1 bit 6 error.".to_string(),
        (8, 8) => "Power vendor status 1 bit 7 error.".to_string(),
        (8, 9) => "Power vendor status 2 bit 0 error.".to_string(),
        (9, 0) => "Power vendor status 2 bit 1 error.".to_string(),
        (9, 1) => "Power vendor status 2 bit 2 error.".to_string(),
        (9, 2) => "Power vendor status 2 bit 3 error.".to_string(),
        (9, 3) => "Power vendor status 2 bit 4 error.".to_string(),
        (9, 4) => "Power vendor status 2 bit 5 error.".to_string(),
        (9, 5) => "Power vendor status 2 bit 6 error.".to_string(),
        (9, 6) => "Power vendor status 2 bit 7 error.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 3: Temperature ──────────────────────────────────────────────

fn temperature_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, n) => format!("Slot {n} temperature sensor detection error."),
        (2, 9) => "Control board temperature sensor communication error.".to_string(),
        (2, n) => format!("Slot {n} temperature reading error."),
        (5, n) => format!("Slot {n} temperature protecting."),
        (6, 0..=3) => "Hashboard high temperature error.".to_string(),
        (7, 0) => "The environment temperature fluctuates too much.".to_string(),
        (8, 0) => "Humidity sensor not found.".to_string(),
        (8, 1 | 2) => "Humidity sensor read error.".to_string(),
        (8, 3) => "Humidity sensor protecting.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 4: EEPROM ───────────────────────────────────────────────────

fn eeprom_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Eeprom unknown error.".to_string(),
        (1, n) => format!("Slot {n} eeprom detection error."),
        (2, n) => format!("Slot {n} eeprom parsing error."),
        (3, n) => format!("Slot {n} chip bin type error."),
        (4, n) => format!("Slot {n} eeprom chip number X error."),
        (5, n) => format!("Slot {n} eeprom xfer error."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 5: Hashboard ────────────────────────────────────────────────

fn hashboard_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Board unknown error.".to_string(),
        (1, n) => format!("Slot {n} miner type error."),
        (2, n) => format!("Slot {n} bin type error."),
        (3, n) => format!("Slot {n} not found."),
        (4, n) => format!("Slot {n} error reading chip id."),
        (5, n) => format!("Slot {n} has bad chips."),
        (6, n) => format!("Slot {n} loss of balance error."),
        (7, n) => format!("Slot {n} xfer error chip."),
        (8, n) => format!("Slot {n} reset error."),
        (9, n) => format!("Slot {n} frequency too low."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 6: Environment temperature ──────────────────────────────────

fn env_temp_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Environment temperature is too high.".to_string(),
        (1, 0) => "Environment temperature is too high for high performance mode.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 7: Control board ────────────────────────────────────────────

fn control_board_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "MAC address invalid".to_string(),
        (0, 1) => "Control board no support chip.".to_string(),
        (1, 0 | 2) => "Control board rebooted as an exception.".to_string(),
        (1, 1) => {
            "Control board rebooted as exception and cpufreq reduced, please upgrade the firmware"
                .to_string()
        }
        (1, 3) => "The network is unstable, change time.".to_string(),
        (1, 4) => "Unknown error.".to_string(),
        (2, n) => format!("Control board slot {n} frame error."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 8: Checksum ─────────────────────────────────────────────────

fn checksum_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "CGMiner checksum error.".to_string(),
        (0, 1) => "System monitor checksum error.".to_string(),
        (0, 2) => "Remote daemon checksum error.".to_string(),
        (1, 0) => "Air to liquid PCB serial # does not match.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 9: Power rate ───────────────────────────────────────────────

fn power_rate_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Unknown error.".to_string(),
        (0, 1) => "Power rate error.".to_string(),
        (0, 2) => "Unknown error.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 20: Pool ────────────────────────────────────────────────────

fn pool_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "No pool information configured.".to_string(),
        (1, 0) => "All pools are disabled.".to_string(),
        (2, n) => format!("Pool {n} connection failed."),
        (3, 0) => "High rejection rate on pool.".to_string(),
        (4, 0) => "The pool does not support asicboost mode.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 21: Factory test ────────────────────────────────────────────

fn factory_test_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (1, n) => format!("Slot {n} factory test step failed."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 23: Hashrate ────────────────────────────────────────────────

fn hashrate_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (1 | 2, 0) => "Hashrate is too low.".to_string(),
        (3 | 4, 0) => "Hashrate loss is too high.".to_string(),
        (5, 0) => "Hashrate loss.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 50: Voltage / water velocity ────────────────────────────────

fn voltage_water_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (1, n) => format!("Slot {n} chip voltage too low."),
        (2, n) => format!("Slot {n} chip voltage changed."),
        (3, n) => format!("Slot {n} chip temperature difference is too large."),
        (4, n) => format!("Slot {n} chip hottest temperature difference is too large."),
        (5, n) => format!("Slot {n} stopped hashing, chips temperature protecting."),
        (7, n) => format!("Slot {n} water velocity is abnormal."),
        (8, 0) => "Chip temp calibration failed, please restore factory settings.".to_string(),
        (9, n) => format!("Slot {n} chip temp calibration check no balance."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 51: Frequency ───────────────────────────────────────────────

fn frequency_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (1 | 7, n) => format!("Slot {n} frequency up timeout."),
        (2, n) => format!("Slot {n} too many CRC errors."),
        (3, n) => format!("Slot {n} unstable."),
        _ => UNKNOWN.to_string(),
    }
}

// ── types 52-56: Slot/chip templates ─────────────────────────────────

fn slot_chip_template(desc: &str, slot: u64, chip: u64) -> String {
    format!("Slot {slot} chip {chip} {desc}.")
}

// ── type 80: Tool / performance ──────────────────────────────────────

fn tool_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "The tool version is too low, please update.".to_string(),
        (1, 0) => "Low freq.".to_string(),
        (2, 0) => "Low hashrate.".to_string(),
        (3, 5) => "High env temp.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 85: Substandard ─────────────────────────────────────────────

fn substandard_error(sub: u64, val: u64) -> String {
    match val {
        0 => format!("Hashrate substandard L{sub}."),
        1 => format!("Power consumption substandard L{sub}."),
        2 | 3 => format!("Fan speed substandard L{sub}."),
        4 => format!("Voltage substandard L{sub}."),
        _ => UNKNOWN.to_string(),
    }
}

// ── type 86: Serial / product info ───────────────────────────────────

fn serial_info_error(sub: u64, val: u64) -> String {
    match (sub, val) {
        (0, 0) => "Missing product serial #.".to_string(),
        (1, 0) => "Missing product type.".to_string(),
        (2, 0) => "Missing miner serial #.".to_string(),
        (2, 1) => "Wrong miner serial # length.".to_string(),
        (3, 0) => "Missing power serial #.".to_string(),
        (3, 1) => "Wrong power serial #.".to_string(),
        (3, 2) => "Fault miner serial #.".to_string(),
        (4, 0) => "Missing power model.".to_string(),
        (4, 1) => "Wrong power model name.".to_string(),
        (4, 2) => "Wrong power model vout.".to_string(),
        (4, 3) => "Wrong power model rate.".to_string(),
        (4, 4) => "Wrong power model format.".to_string(),
        (5, 0) => "Wrong hash board struct.".to_string(),
        (6, 0) => "Wrong miner cooling type.".to_string(),
        (7, 0) => "Missing PCB serial #.".to_string(),
        _ => UNKNOWN.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_code_returns_unknown() {
        // Act
        let result = error_message(12);

        // Assert
        assert_eq!(result, "Unknown error type.");
    }

    #[test]
    fn fan_intake_speed_error() {
        // Act
        let result = error_message(110);

        // Assert
        assert_eq!(result, "Intake fan speed error.");
    }

    #[test]
    fn power_low_voltage_for_high_power() {
        // Act
        let result = error_message(218);

        // Assert
        assert_eq!(
            result,
            "Power input voltage is lower than 230V for high power mode."
        );
    }

    #[test]
    fn temperature_slot_template() {
        // Act
        let result = error_message(301);

        // Assert
        assert_eq!(result, "Slot 1 temperature sensor detection error.");
    }

    #[test]
    fn temperature_reading_exact_override() {
        // Act – code 329 should match the exact (2, 9) arm, not the template
        let result = error_message(329);

        // Assert
        assert_eq!(
            result,
            "Control board temperature sensor communication error."
        );
    }

    #[test]
    fn slot_chip_template_error_nonce() {
        // Act – type 52, subtype=1, value=3
        let result = error_message(5213);

        // Assert
        assert_eq!(result, "Slot 1 chip 3 error nonce.");
    }

    #[test]
    fn slot_chip_template_zero_nonce() {
        // Act – type 56, subtype=2, value=5
        let result = error_message(5625);

        // Assert
        assert_eq!(result, "Slot 2 chip 5 zero nonce.");
    }

    #[test]
    fn pool_connection_failed_template() {
        // Act
        let result = error_message(2022);

        // Assert
        assert_eq!(result, "Pool 2 connection failed.");
    }

    #[test]
    fn substandard_hashrate() {
        // Act – type 85, subtype=3, value=0
        let result = error_message(8530);

        // Assert
        assert_eq!(result, "Hashrate substandard L3.");
    }

    #[test]
    fn security_library_error() {
        // Act – type 1000, subtype=0, value=0
        let result = error_message(100000);

        // Assert
        assert_eq!(result, "Security library error, please upgrade firmware");
    }

    #[test]
    fn unknown_type_returns_unknown() {
        // Act
        let result = error_message(999);

        // Assert
        assert_eq!(result, "Unknown error type.");
    }

    #[test]
    fn env_temp_too_high() {
        // Act
        let result = error_message(600);

        // Assert
        assert_eq!(result, "Environment temperature is too high.");
    }

    #[test]
    fn hashboard_not_found() {
        // Act – type 5, subtype=3, value=2
        let result = error_message(532);

        // Assert
        assert_eq!(result, "Slot 2 not found.");
    }
}
