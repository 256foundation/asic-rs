use crate::data::device::models::{MinerModelFactory, ModelSelectionError};
use crate::data::device::{MinerMake, MinerModel};
use chrono::{Datelike, NaiveDateTime};
use diqwest::WithDigestAuth;
use reqwest::{Client, Response};
use semver;
use std::net::IpAddr;

pub(crate) async fn get_model_antminer(ip: IpAddr) -> Result<MinerModel, ModelSelectionError> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/cgi-bin/get_system_info.cgi"))
        .send_digest_auth(("root", "root"))
        .await
        .ok();
    match response {
        Some(data) => {
            let json_data = data.json::<serde_json::Value>().await.ok();
            if json_data.is_none() {
                return Err(ModelSelectionError::UnexpectedModelResponse);
            }
            let json_data = json_data.unwrap();

            let model = json_data["minertype"].as_str().unwrap_or("").to_uppercase();

            MinerModelFactory::new()
                .with_make(MinerMake::AntMiner)
                .parse_model(&model)
        }
        None => Err(ModelSelectionError::NoModelResponse),
    }
}

pub(crate) async fn get_version_antminer(ip: IpAddr) -> Option<semver::Version> {
    let response: Option<Response> = Client::new()
        .get(format!("http://{ip}/cgi-bin/summary.cgi"))
        .send_digest_auth(("root", "root"))
        .await
        .ok();
    match response {
        Some(data) => {
            let json_data = data.json::<serde_json::Value>().await.ok()?;
            let fw_version = json_data["INFO"]["CompileTime"].as_str().unwrap_or("");

            let cleaned: String = {
                let mut parts: Vec<&str> = fw_version.split_whitespace().collect();
                parts.remove(4); // remove time zone
                parts.join(" ")
            };

            let dt = NaiveDateTime::parse_from_str(&cleaned, "%a %b %e %H:%M:%S %Y").ok()?;

            let version =
                semver::Version::new(dt.year() as u64, dt.month() as u64, dt.day() as u64);

            Some(version)
        }
        None => None,
    }
}
