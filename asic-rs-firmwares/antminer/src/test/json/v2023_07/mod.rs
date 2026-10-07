#![cfg(test)]
#![allow(dead_code)]

// Captured from live hardware: an Antminer L9 and L11 on stock firmware
// (BMMiner 2.12, API 3.1). These commands carry telemetry only -- no pool,
// worker, network or serial identifiers.
pub(crate) const L9_STATS: &str = include_str!("l9_stats.json");
pub(crate) const L9_SUMMARY: &str = include_str!("l9_summary.json");
pub(crate) const L9_VERSION: &str = include_str!("l9_version.json");
pub(crate) const L11_STATS: &str = include_str!("l11_stats.json");
pub(crate) const L11_SUMMARY: &str = include_str!("l11_summary.json");
pub(crate) const L11_VERSION: &str = include_str!("l11_version.json");
