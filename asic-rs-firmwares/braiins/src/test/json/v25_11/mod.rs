#![cfg(test)]

pub(crate) const WEB_NETWORK_COMMAND: &str = include_str!("network.json");
pub(crate) const WEB_VERSION_COMMAND: &str = include_str!("version.json");
pub(crate) const WEB_MINER_DETAILS_COMMAND: &str = include_str!("miner_details.json");
pub(crate) const WEB_MINER_STATS_COMMAND: &str = include_str!("miner_stats.json");
pub(crate) const WEB_PERFORMANCE_TUNER_STATE_COMMAND: &str =
    include_str!("performance_tuner_state.json");
pub(crate) const WEB_MINER_ERRORS_COMMAND: &str = include_str!("miner_errors.json");
pub(crate) const WEB_POOLS_COMMAND: &str = include_str!("pools.json");
pub(crate) const WEB_COOLING_STATE_COMMAND: &str = include_str!("cooling_state.json");
pub(crate) const WEB_HASHBOARDS_COMMAND: &str = include_str!("miner_hw_hashboards.json");
pub(crate) const WEB_LOCATE_COMMAND: &str = include_str!("actions_locate.json");
