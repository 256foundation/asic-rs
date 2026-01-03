use anyhow;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::LazyLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::miners::api::rpc::errors::RPCError;
use crate::miners::api::rpc::status::RPCCommandStatus;
use crate::miners::backends::traits::*;
use crate::miners::commands::MinerCommand;

static STATS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\w+)\[([^]]+)]").unwrap());
static NESTED_STATS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"'([^']+)':\{([^}]*)}").unwrap());

#[derive(Debug)]
pub struct AvalonMinerRPCAPI {
    ip: IpAddr,
    port: u16,
}

impl AvalonMinerRPCAPI {
    pub fn new(ip: IpAddr) -> Self {
        Self { ip, port: 4028 }
    }

    fn parse_rpc_result(&self, response: &str) -> anyhow::Result<Value> {
        let mut val: Value = serde_json::from_str(response)?;

        let status_array = val
            .get("STATUS")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid STATUS array"))?;

        if status_array.is_empty() {
            anyhow::bail!("Empty STATUS array");
        }

        let status_str = status_array[0]
            .get("STATUS")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing STATUS field"))?;

        let message = status_array[0].get("Msg").and_then(|v| v.as_str());
        let status = RPCCommandStatus::from_str(status_str, message);

        status.into_result().map_err(|e| anyhow::anyhow!(e))?;

        if let Some(stats_arr) = val["STATS"].as_array_mut() {
            for item in stats_arr {
                // MM ID0:Summary
                if let Some(s) = item["MM ID0:Summary"].as_str() {
                    let parsed = self.parse_nested_stats(s);
                    item["MM ID0:Summary"] = json!(parsed);
                }

                // HBinfo
                if let Some(s) = item["HBinfo"].as_str() {
                    let parsed = self.parse_nested_stats(s);
                    item["HBinfo"] = json!(parsed);
                }

                if let Some(s) = item["MM ID0"].as_str() {
                    let parsed = self.parse_stats(s);
                    item["MM ID0"] = json!(parsed);
                }
            }
        }

        Ok(val)
    }

    fn convert_value(&self, val: &str, key: &str) -> Value {
        let val = val.trim();

        if key == "SYSTEMSTATU" {
            return Value::String(val.to_string());
        }

        if val.contains(' ') {
            let parts = val.split_whitespace();
            let mut result = Vec::new();
            for part in parts {
                if part.chars().all(|c| c.is_ascii_digit()) {
                    // all digits → int
                    if let Ok(i) = part.parse::<i64>() {
                        result.push(Value::Number(i.into()));
                        continue;
                    }
                }
                // else try float
                if let Ok(f) = part.parse::<f64>() {
                    result.push(json!(f));
                } else {
                    result.push(Value::String(part.to_string()));
                }
            }
            Value::Array(result)
        } else if val.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(i) = val.parse::<i64>() {
                Value::Number(i.into())
            } else {
                Value::String(val.to_string())
            }
        } else if let Ok(f) = val.parse::<f64>() {
            json!(f)
        } else {
            Value::String(val.to_string())
        }
    }

    fn parse_stats(&self, stats: &str) -> HashMap<String, Value> {
        let mut stats_dict = HashMap::new();
        let re = STATS_RE.clone();

        for cap in re.captures_iter(stats) {
            let key = cap[1].to_string();
            let value_str = &cap[2];

            let parsed_value = self.convert_value(value_str, &key);
            stats_dict.insert(key, parsed_value);
        }

        stats_dict
    }

    fn parse_nested_stats(&self, stats: &str) -> HashMap<String, HashMap<String, Value>> {
        let mut outer = HashMap::new();
        let re = NESTED_STATS_RE.clone();

        for cap in re.captures_iter(stats) {
            let section = cap[1].to_string();
            let inner_str = &cap[2];
            let inner_map = self.parse_stats(inner_str);
            outer.insert(section, inner_map);
        }
        outer
    }
}

#[async_trait]
impl RPCAPIClient for AvalonMinerRPCAPI {
    async fn send_command(
        &self,
        command: &str,
        _privileged: bool,
        param: Option<Value>,
    ) -> anyhow::Result<Value> {
        let cmd = match param {
            Some(params) => json!({
                "command": command,
                "parameter": params
            }),
            None => json!({
                "command": command
            }),
        };

        let stream = tokio::net::TcpStream::connect(format!("{}:{}", self.ip, self.port))
            .await
            .map_err(|_| RPCError::ConnectionFailed)?;
        let mut stream = stream;

        let json_str = cmd.to_string();
        stream.write_all(json_str.as_bytes()).await?;

        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer).await?;

        if buffer.is_empty() {
            anyhow::bail!("No data received from miner");
        }

        let response = String::from_utf8_lossy(&buffer)
            .into_owned()
            .replace('\0', "");

        if response == "Socket connect failed: Connection refused\n" {
            anyhow::bail!("Miner connection refused");
        }

        self.parse_rpc_result(&response)
    }
}

#[async_trait]
impl APIClient for AvalonMinerRPCAPI {
    async fn get_api_result(&self, command: &MinerCommand) -> anyhow::Result<Value> {
        match command {
            MinerCommand::RPC {
                command,
                parameters,
            } => self
                .send_command(command, false, parameters.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string())),
            _ => Err(anyhow::anyhow!("Cannot send non RPC command to RPC API")),
        }
    }
}
#[cfg(test)]
mod parse_rpc_result_nested_tests {
    use super::*;
    use crate::test::json::cgminer::avalon::{STATS_COMMAND, VERSION_COMMAND};
    use std::net::{IpAddr, Ipv4Addr};

    fn test_rpc() -> AvalonMinerRPCAPI {
        AvalonMinerRPCAPI::new(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }

    #[test]
    fn parses_full_cgminer_response() {
        let val = test_rpc().parse_rpc_result(STATS_COMMAND).unwrap();
        assert_eq!(val.pointer("/STATUS/0/STATUS"), Some(&json!("S")));

        assert_eq!(
            val.pointer("/STATS/0/MM ID0:Summary/STATS/GHSmm"),
            Some(&json!(55032.79))
        );
        assert_eq!(
            val.pointer("/STATS/0/MM ID0:Summary/STATS/Freq"),
            Some(&json!(282.86))
        );

        assert_eq!(
            val.pointer("/STATS/0/HBinfo/HB0/PVT_T0/0"),
            Some(&json!(58))
        );
        assert_eq!(val.pointer("/STATS/0/HBinfo/HB0/MW0/1"), Some(&json!(664)));

        assert_eq!(val.pointer("/STATS/1/ID"), Some(&json!("POOL0")));

        assert_eq!(
            val.pointer("/STATS/0/MM ID0:Summary/STATS/BVer"),
            Some(&json!("25052801_14a19a2"))
        );
    }

    #[test]
    fn hbinfo_realistic_long_string() {
        let long = "'HB0':{PVT_T0[58 59 60] MW0[100 200]} 'HB1':{PVT_T0[99 98] MW0[300 400]}";
        let resp = format!(
            r#"
            {{
              "STATUS":[{{"STATUS":"S","Msg":"ok"}}],
              "STATS":[{{"HBinfo":"{long}"}}]
            }}"#
        );
        let val = test_rpc().parse_rpc_result(&resp).unwrap();
        assert_eq!(
            val.pointer("/STATS/0/HBinfo/HB0/PVT_T0/2"),
            Some(&json!(60))
        );
        assert_eq!(val.pointer("/STATS/0/HBinfo/HB0/MW0/1"), Some(&json!(200)));
    }

    #[test]
    fn hbinfo_empty_block_is_ok() {
        let resp = r#"
        {
          "STATUS":[{"STATUS":"S","Msg":"ok"}],
          "STATS":[{
            "HBinfo":"'HB0':{}"
          }]
        }"#;
        let val = test_rpc().parse_rpc_result(resp).unwrap();
        assert_eq!(val.pointer("/STATS/0/HBinfo/HB0"), Some(&json!({})));
    }

    #[test]
    fn version_command_returns_version() {
        let val = test_rpc().parse_rpc_result(VERSION_COMMAND).unwrap();

        assert_eq!(val.pointer("/VERSION/0/API"), Some(&json!("3.7")));
    }
}
