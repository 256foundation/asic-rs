use std::{net::IpAddr, time::Duration};

use anyhow::{Result, anyhow, bail};
use asic_rs_core::{
    config::pools::{PoolConfig, PoolGroupConfig},
    data::command::MinerCommand,
    traits::miner::{APIClient, MinerAuth, WebAPIClient},
};
use async_trait::async_trait;
use diqwest::WithDigestAuth;
use once_cell::sync::OnceCell;
use reqwest::{Client, Method};
use serde_json::Value;
use tokio::time::sleep;
use url::form_urlencoded;

#[derive(Debug, Default)]
struct VolcMinerConfMetadata {
    runmode: String,
    voltage: String,
    debug_enabled: bool,
}

#[derive(Debug)]
pub struct VolcMinerWebAPI {
    ip: IpAddr,
    port: u16,
    client: OnceCell<Client>,
    timeout: Duration,
    auth: MinerAuth,
}

impl VolcMinerWebAPI {
    pub fn new(ip: IpAddr, auth: MinerAuth) -> Self {
        Self {
            ip,
            port: 80,
            client: OnceCell::new(),
            timeout: Duration::from_secs(10),
            auth,
        }
    }

    pub fn set_auth(&mut self, auth: MinerAuth) {
        self.auth = auth;
    }

    fn build_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow!("failed to create HTTP client: {e}"))
    }

    fn client(&self) -> Result<&Client> {
        self.client.get_or_try_init(Self::build_client)
    }

    fn url(&self, command: &str) -> String {
        format!("http://{}:{}/cgi-bin/{}.cgi", self.ip, self.port, command)
    }

    async fn request_text(
        &self,
        command: &str,
        method: Method,
        body: Option<String>,
    ) -> Result<String> {
        let client = self.client()?;
        let url = self.url(command);
        let mut builder = match method {
            Method::GET => client.get(url),
            Method::POST => client.post(url),
            _ => bail!("Unsupported HTTP method: {method}"),
        }
        .timeout(self.timeout);

        if let Some(body) = body {
            builder = builder
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(body);
        }

        let response = builder
            .send_digest_auth((self.auth.username(), self.auth.password()))
            .await
            .map_err(|e| anyhow!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            bail!("HTTP request failed with status: {status}");
        }

        response.text().await.map_err(|e| anyhow!(e.to_string()))
    }

    async fn request_json(&self, command: &str) -> Result<Value> {
        let text = self.request_text(command, Method::GET, None).await?;
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(error) if command == "get_miner_status" => Self::parse_miner_status_text(&text)
                .map_err(|fallback_error| {
                    anyhow!(
                        "failed to parse {command} JSON: {error}; fallback parser failed: {fallback_error}"
                    )
                }),
            Err(error) => Err(anyhow!("failed to parse {command} JSON: {error}")),
        }
    }

    pub async fn get_miner_conf(&self) -> Result<Value> {
        self.request_json("get_miner_conf").await
    }

    pub async fn get_system_info(&self) -> Result<Value> {
        self.request_json("get_system_info").await
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

    fn extract_text_field(text: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let after_key = text.get(text.find(&needle)? + needle.len()..)?;
        let after_colon = after_key.get(after_key.find(':')? + 1..)?.trim_start();

        if let Some(value) = after_colon.strip_prefix('"') {
            return value.split('"').next().map(str::to_string);
        }

        let value = after_colon
            .split([',', '}', '\n'])
            .next()?
            .trim()
            .trim_matches('"');
        Some(value.to_string())
    }

    fn matching_delimiter_end(
        text: &str,
        start: usize,
        opener: char,
        closer: char,
    ) -> Option<usize> {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;

        for (offset, ch) in text.get(start..)?.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }

            if ch == '"' {
                in_string = true;
            } else if ch == opener {
                depth += 1;
            } else if ch == closer {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(start + offset + ch.len_utf8());
                }
            }
        }

        None
    }

    fn extract_json_after_key(text: &str, key: &str, opener: char, closer: char) -> Option<String> {
        let needle = format!("\"{key}\"");
        let after_key_start = text.find(&needle)? + needle.len();
        let after_key = text.get(after_key_start..)?;
        let colon = after_key.find(':')?;
        let after_colon_start = after_key_start + colon + 1;
        let after_colon = text.get(after_colon_start..)?;
        let opener_offset = after_colon.find(opener)?;
        let start = after_colon_start + opener_offset;
        let end = Self::matching_delimiter_end(text, start, opener, closer)?;

        text.get(start..end).map(str::to_string)
    }

    fn extract_quoted_field(text: &str, key: &str) -> Option<String> {
        let needle = format!("\"{key}\"");
        let after_key = text.get(text.find(&needle)? + needle.len()..)?;
        let after_colon = after_key.get(after_key.find(':')? + 1..)?.trim_start();
        let value = after_colon.strip_prefix('"')?;
        value.split('"').next().map(str::to_string)
    }

    fn extract_temp_field(text: &str) -> Option<String> {
        let needle = "\"temp\"";
        let after_key = text.get(text.find(needle)? + needle.len()..)?;
        let after_bracket = after_key.get(after_key.find('[')? + 1..)?;
        after_bracket
            .split([',', ']'])
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn parse_legacy_devs(text: &str) -> Vec<Value> {
        let Some(devs_text) = Self::extract_json_after_key(text, "devs", '[', ']') else {
            return vec![];
        };

        let mut devs = Vec::new();
        let mut remaining = devs_text.as_str();
        while let Some(start) = remaining.find('{') {
            let Some(end) = Self::matching_delimiter_end(remaining, start, '{', '}') else {
                break;
            };
            let Some(dev_text) = remaining.get(start..end) else {
                break;
            };
            let mut dev = serde_json::Map::new();
            for key in ["index", "chain_acn", "freq", "chain_acs"] {
                if let Some(value) = Self::extract_quoted_field(dev_text, key) {
                    dev.insert(key.to_string(), Value::String(value));
                }
            }
            if let Some(temp) = Self::extract_temp_field(dev_text) {
                dev.insert("temp".to_string(), Value::String(temp));
            }
            if !dev.is_empty() {
                devs.push(Value::Object(dev));
            }
            remaining = remaining.get(end..).unwrap_or_default();
        }

        devs
    }

    fn parse_miner_status_text(text: &str) -> Result<Value> {
        let mut status = serde_json::Map::new();

        let summary = Self::extract_json_after_key(text, "summary", '{', '}')
            .ok_or_else(|| anyhow!("missing summary"))?;
        status.insert("summary".to_string(), serde_json::from_str(&summary)?);

        if let Some(pools) = Self::extract_json_after_key(text, "pools", '[', ']')
            && let Ok(pools) = serde_json::from_str(&pools)
        {
            status.insert("pools".to_string(), pools);
        }

        for idx in 1..=4 {
            let key = format!("fan{idx}");
            if let Some(value) = Self::extract_quoted_field(text, &key) {
                status.insert(key, Value::String(value));
            }
        }

        let devs = Self::parse_legacy_devs(text);
        if !devs.is_empty() {
            status.insert("devs".to_string(), Value::Array(devs));
        }

        Ok(Value::Object(status))
    }

    async fn conf_metadata(&self) -> VolcMinerConfMetadata {
        let Ok(text) = self
            .request_text("get_miner_confV1", Method::GET, None)
            .await
        else {
            return VolcMinerConfMetadata::default();
        };

        VolcMinerConfMetadata {
            runmode: Self::extract_text_field(&text, "runmode").unwrap_or_else(|| "0".to_string()),
            voltage: Self::extract_text_field(&text, "voltage")
                .unwrap_or_else(|| "1260".to_string()),
            debug_enabled: Self::extract_text_field(&text, "bb_debug_enable")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }

    fn append_pair(
        serializer: &mut form_urlencoded::Serializer<'_, String>,
        key: &str,
        value: &str,
    ) {
        serializer.append_pair(key, value);
    }

    fn build_miner_conf_body(
        current: &Value,
        metadata: &VolcMinerConfMetadata,
        pools: &[PoolConfig],
    ) -> String {
        let mut serializer = form_urlencoded::Serializer::new(String::new());

        for idx in 0..3 {
            let pool = pools.get(idx);
            let prefix = idx + 1;
            Self::append_pair(
                &mut serializer,
                &format!("_bb_pool{prefix}url"),
                &pool.map(|p| p.url.to_string()).unwrap_or_default(),
            );
            Self::append_pair(
                &mut serializer,
                &format!("_bb_pool{prefix}user"),
                pool.map(|p| p.username.as_str()).unwrap_or_default(),
            );
            Self::append_pair(
                &mut serializer,
                &format!("_bb_pool{prefix}pw"),
                pool.map(|p| p.password.as_str()).unwrap_or_default(),
            );
        }

        let bool_str = |value: bool| if value { "true" } else { "false" };
        Self::append_pair(
            &mut serializer,
            "_bb_nobeeper",
            bool_str(Self::bool_field(current, "nobeeper")),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_notempoverctrl",
            bool_str(Self::bool_field(current, "notempoverctrl")),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_fan_customize_switch",
            bool_str(Self::bool_field(current, "fan-ctrl")),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_fan_customize_value_front",
            &Self::string_field(current, "fan-pwm-front", ""),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_fan_customize_value_back",
            &Self::string_field(current, "fan-pwm-back", ""),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_freq",
            &Self::string_field(current, "freq", "2000"),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_coin_type",
            &Self::string_field(current, "coin-type", "ltc"),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_runmode",
            if metadata.runmode.is_empty() {
                "0"
            } else {
                &metadata.runmode
            },
        );
        Self::append_pair(
            &mut serializer,
            "_bb_voltage_customize_value",
            if metadata.voltage.is_empty() {
                "1260"
            } else {
                &metadata.voltage
            },
        );
        Self::append_pair(
            &mut serializer,
            "_bb_ema",
            &Self::string_field(current, "sram-voltage", "3"),
        );
        Self::append_pair(
            &mut serializer,
            "_bb_debug",
            bool_str(metadata.debug_enabled),
        );

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

    fn pools_match_config(config: &Value, pools: &[PoolConfig]) -> bool {
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

        Self::configured_pools(config) == expected
    }

    async fn confirm_pools_config(&self, pools: &[PoolConfig]) -> bool {
        for _ in 0..5 {
            if self
                .get_miner_conf()
                .await
                .map(|config| Self::pools_match_config(&config, pools))
                .unwrap_or(false)
            {
                return true;
            }
            sleep(Duration::from_secs(1)).await;
        }

        false
    }

    pub async fn set_pools_config(&self, config: Vec<PoolGroupConfig>) -> Result<bool> {
        let current = self.get_miner_conf().await?;
        let metadata = self.conf_metadata().await;
        let pools = config
            .into_iter()
            .flat_map(|group| group.pools)
            .take(3)
            .collect::<Vec<_>>();
        let body = Self::build_miner_conf_body(&current, &metadata, &pools);

        if let Err(error) = self
            .request_text("set_miner_conf", Method::POST, Some(body))
            .await
            && !self.confirm_pools_config(&pools).await
        {
            return Err(error);
        }

        Ok(true)
    }
}

#[async_trait]
impl APIClient for VolcMinerWebAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> Result<Value> {
        match command {
            MinerCommand::WebAPI { command, .. } => {
                self.send_command(command, false, None, Method::GET).await
            }
            _ => Err(anyhow!("Unsupported command type for VolcMiner API")),
        }
    }
}

#[async_trait]
impl WebAPIClient for VolcMinerWebAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        _parameters: Option<Value>,
        method: Method,
    ) -> Result<Value> {
        match method {
            Method::GET => self.request_json(command).await,
            _ => bail!("Unsupported VolcMiner command method: {method}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use asic_rs_core::data::pool::PoolURL;

    use super::*;

    #[test]
    fn build_conf_body_preserves_settings_and_orders_pool_fields() {
        let current = serde_json::json!({
            "freq": "1875",
            "coin-type": "ltc",
            "sram-voltage": "3",
            "fan-ctrl": true,
            "fan-pwm-front": "70",
            "fan-pwm-back": "80"
        });
        let metadata = VolcMinerConfMetadata {
            runmode: "0".to_string(),
            voltage: "1260".to_string(),
            debug_enabled: false,
        };
        let pools = vec![PoolConfig {
            url: PoolURL::from("stratum+tcp://example.invalid:3333".to_string()),
            username: "worker".to_string(),
            password: "x".to_string(),
        }];

        let body = VolcMinerWebAPI::build_miner_conf_body(&current, &metadata, &pools);

        assert!(body.starts_with("_bb_pool1url=stratum%2Btcp%3A%2F%2Fexample.invalid%3A3333&_bb_pool1user=worker&_bb_pool1pw=x&_bb_pool2url="));
        assert!(body.contains("&_bb_freq=1875&"));
        assert!(body.contains("&_bb_runmode=0&"));
        assert!(body.contains("&_bb_voltage_customize_value=1260&"));
    }

    #[test]
    fn parse_malformed_status_response_extracts_fans() -> Result<()> {
        let status = VolcMinerWebAPI::parse_miner_status_text(
            r#"{
"summary": {"elapsed":"119","ghs5s":"6678.46"},
"pools": [{"index":"0","url":"stratum+tcp://pool.invalid:3333","user":"worker","status":"Alive"}],
"fan1":"4530",
"fan2":"4530",
"fan3":"4530",
"fan4":"4500",
"devs": [{"index":"1","chain_acn":"105","freq":"1900","temp":[45,temp5=0],"chain_acs":"oooooooo"}]
}"#,
        )?;

        assert_eq!(
            status.pointer("/summary/ghs5s"),
            Some(&Value::String("6678.46".to_string()))
        );
        assert_eq!(
            status.pointer("/fan1"),
            Some(&Value::String("4530".to_string()))
        );
        assert_eq!(
            status.pointer("/devs/0/temp"),
            Some(&Value::String("45".to_string()))
        );

        Ok(())
    }
}
