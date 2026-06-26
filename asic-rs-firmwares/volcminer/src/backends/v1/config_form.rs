use asic_rs_core::config::pools::PoolConfig;
use serde_json::Value;
use url::form_urlencoded;

#[derive(Debug, Default)]
pub(super) struct MinerConfMetadata {
    pub(super) runmode: String,
    pub(super) voltage: String,
    pub(super) debug_enabled: bool,
}

fn string_field(value: &Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn build_miner_conf_body(
    current: &Value,
    metadata: &MinerConfMetadata,
    pools: &[PoolConfig],
) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());

    for idx in 0..3 {
        let pool = pools.get(idx);
        let prefix = idx + 1;
        serializer.append_pair(
            &format!("_bb_pool{prefix}url"),
            &pool.map(|p| p.url.to_string()).unwrap_or_default(),
        );
        serializer.append_pair(
            &format!("_bb_pool{prefix}user"),
            pool.map(|p| p.username.as_str()).unwrap_or_default(),
        );
        serializer.append_pair(
            &format!("_bb_pool{prefix}pw"),
            pool.map(|p| p.password.as_str()).unwrap_or_default(),
        );
    }

    let bool_str = |value: bool| if value { "true" } else { "false" };
    serializer.append_pair("_bb_nobeeper", bool_str(bool_field(current, "nobeeper")));
    serializer.append_pair(
        "_bb_notempoverctrl",
        bool_str(bool_field(current, "notempoverctrl")),
    );
    serializer.append_pair(
        "_bb_fan_customize_switch",
        bool_str(bool_field(current, "fan-ctrl")),
    );
    serializer.append_pair(
        "_bb_fan_customize_value_front",
        &string_field(current, "fan-pwm-front", ""),
    );
    serializer.append_pair(
        "_bb_fan_customize_value_back",
        &string_field(current, "fan-pwm-back", ""),
    );
    serializer.append_pair("_bb_freq", &string_field(current, "freq", "2000"));
    serializer.append_pair("_bb_coin_type", &string_field(current, "coin-type", "ltc"));
    serializer.append_pair(
        "_bb_runmode",
        if metadata.runmode.is_empty() {
            "0"
        } else {
            &metadata.runmode
        },
    );
    serializer.append_pair(
        "_bb_voltage_customize_value",
        if metadata.voltage.is_empty() {
            "1260"
        } else {
            &metadata.voltage
        },
    );
    serializer.append_pair("_bb_ema", &string_field(current, "sram-voltage", "3"));
    serializer.append_pair("_bb_debug", bool_str(metadata.debug_enabled));

    serializer.finish()
}

fn configured_pools(config: &Value) -> Vec<(String, String, String)> {
    config
        .get("pools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|pool| {
            let url = pool.get("url").and_then(Value::as_str).unwrap_or_default();
            if url.is_empty() {
                return None;
            }
            Some((
                url.to_string(),
                pool.get("user")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                pool.get("pass")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            ))
        })
        .collect()
}

pub(super) fn pools_match_config(config: &Value, pools: &[PoolConfig]) -> bool {
    let expected = pools
        .iter()
        .map(|pool| {
            (
                pool.url.to_string(),
                pool.username.clone(),
                pool.password.clone(),
            )
        })
        .collect::<Vec<_>>();

    configured_pools(config) == expected
}
