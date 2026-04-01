#![cfg(test)]

pub(crate) const SUMMARY_COMMAND: &str = include_str!("summary.json");
pub(crate) const STATUS_COMMAND: &str = include_str!("status.json");
pub(crate) const POOLS_COMMAND: &str = include_str!("pools.json");
pub(crate) const DEVS_COMMAND: &str = include_str!("devs.json");
pub(crate) const GET_VERSION_COMMAND: &str = include_str!("get_version.json");
pub(crate) const GET_PSU_COMMAND: &str = include_str!("get_psu.json");
pub(crate) const GET_MINER_INFO_COMMAND: &str = include_str!("get_miner_info.json");
pub(crate) const GET_ERROR_CODE_COMMAND: &str = include_str!("get_error_code.json");
pub(crate) const GET_ERROR_CODE_WITH_ERRORS_COMMAND: &str =
    include_str!("get_error_code_with_errors.json");
