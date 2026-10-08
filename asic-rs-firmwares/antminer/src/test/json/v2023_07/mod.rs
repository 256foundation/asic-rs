#![cfg(test)]
#![allow(dead_code)]

pub(crate) const S21_PLUS_HYDRO_SYNTHETIC: &str = include_str!("s21_plus_hydro_synthetic.json");
pub(crate) const S21_HYDRO_COOLANT_SYNTHETIC: &str =
    include_str!("s21_hydro_coolant_synthetic.json");
pub(crate) const S23_HYDRO_STANDARD_SYNTHETIC: &str =
    include_str!("s23_hydro_standard_synthetic.json");

// Captured from live hardware: an Antminer L9 and L11 on stock firmware
// (BMMiner 2.12, API 3.1). These commands carry telemetry only -- no pool,
// worker, network or serial identifiers.
pub(crate) const L9_STATS: &str = include_str!("l9_stats.json");
pub(crate) const L9_SUMMARY: &str = include_str!("l9_summary.json");
pub(crate) const L9_VERSION: &str = include_str!("l9_version.json");
pub(crate) const L11_STATS: &str = include_str!("l11_stats.json");
pub(crate) const L11_SUMMARY: &str = include_str!("l11_summary.json");
pub(crate) const L11_VERSION: &str = include_str!("l11_version.json");

// Reduced stock hydro telemetry captures from 2026-10-07; identifiers are redacted.
pub(crate) const S21_XP_HYDRO_WEB_STATS_CAPTURED: &str =
    include_str!("s21_xp_hydro_web_stats_captured.json");
pub(crate) const S21J_XP_HYDRO_WEB_STATS_CAPTURED: &str =
    include_str!("s21j_xp_hydro_web_stats_captured.json");
pub(crate) const S21J_XP_HYDRO_RPC_STATS_CAPTURED: &str =
    include_str!("s21j_xp_hydro_rpc_stats_captured.json");
pub(crate) const S21_XP_HYDRO_WEB_SUMMARY_CAPTURED: &str =
    include_str!("s21_xp_hydro_web_summary_captured.json");
pub(crate) const S21J_XP_HYDRO_WEB_SUMMARY_CAPTURED: &str =
    include_str!("s21j_xp_hydro_web_summary_captured.json");

// Read-only RPC telemetry captures from 2026-10-08. Identifiers are redacted;
// schema defects and numerical readings are preserved for parser regression.
pub(crate) const Z15_MALFORMED_STATS_CAPTURED: &str =
    include_str!("z15_malformed_stats_captured.txt");
pub(crate) const Z15_SUMMARY_CAPTURED: &str = include_str!("z15_summary_captured.json");
pub(crate) const Z15PRO_MODERN_STATS_CAPTURED: &str =
    include_str!("z15pro_modern_stats_captured.json");
pub(crate) const Z15PRO_LEGACY_STATS_CAPTURED: &str =
    include_str!("z15pro_legacy_stats_captured.json");
pub(crate) const Z15PRO_SUMMARY_CAPTURED: &str = include_str!("z15pro_summary_captured.json");
pub(crate) const HIVEON_S19X88_STATS_CAPTURED: &str =
    include_str!("hiveon_s19x88_stats_captured.json");
pub(crate) const KS7_MODERN_STATS_CAPTURED: &str = include_str!("ks7_modern_stats_captured.json");
pub(crate) const S21PROPLUS_MODERN_STATS_CAPTURED: &str =
    include_str!("s21proplus_modern_stats_captured.json");
