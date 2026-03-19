use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use strum::{EnumIter, IntoEnumIterator};

pub use crate::data::collector::{get_by_key, get_by_pointer};
use crate::{
    data::command::MinerCommand,
    traits::miner::{APIClient, GetConfigsLocations},
};

/// Represents the individual configs that can be queried from a miner device.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy, EnumIter)]
pub enum ConfigField {
    Pools,
    Scaling,
    Tuning,
}

/// A function pointer type that takes a JSON `Value` and an optional key,
/// returning the extracted value if found.
type ExtractorFn = for<'a> fn(&'a Value, Option<&'static str>) -> Option<&'a Value>;

/// Describes how to extract a specific value from a command's response.
///
/// Created by a backend and used to locate a field within a JSON structure.
#[derive(Clone, Copy)]
pub struct ConfigExtractor {
    /// Function used to extract data from a JSON response.
    pub func: ExtractorFn,
    /// Optional key or pointer within the response to extract.
    pub key: Option<&'static str>,
    /// Optional tag to move the extracted value to
    pub tag: Option<&'static str>,
}

/// Alias for a tuple describing the API command and the extractor used to parse its result.
pub type ConfigLocation = (MinerCommand, ConfigExtractor);

/// A trait for types that can be extracted from a JSON Value.
pub trait FromValue: Sized {
    /// Attempts to convert a JSON Value to Self.
    fn from_value(value: &Value) -> Option<Self>;
}

/// Extension trait for HashMap<ConfigField, Value> to provide cleaner value extraction.
pub trait ConfigExtensions {
    /// Extract a value of type T from the config map for the given field.
    fn extract<T: FromValue>(&self, field: ConfigField) -> Option<T>;

    /// Extract a value of type T from the config map for the given field, with a default value.
    fn extract_or<T: FromValue>(&self, field: ConfigField, default: T) -> T;

    /// Extract a nested value of type T from the config map for the given field and nested key.
    fn extract_nested<T: FromValue>(&self, field: ConfigField, nested_key: &str) -> Option<T>;

    /// Extract a nested value of type T from the config map for the given field and nested key, with a default value.
    fn extract_nested_or<T: FromValue>(
        &self,
        field: ConfigField,
        nested_key: &str,
        default: T,
    ) -> T;

    /// Extract a value and map it to another type using the provided function.
    fn extract_map<T: FromValue, U>(&self, field: ConfigField, f: impl FnOnce(T) -> U)
    -> Option<U>;

    /// Extract a value, map it to another type, or use a default value.
    fn extract_map_or<T: FromValue, U>(
        &self,
        field: ConfigField,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U;

    /// Extract a nested value and map it to another type using the provided function.
    fn extract_nested_map<T: FromValue, U>(
        &self,
        field: ConfigField,
        nested_key: &str,
        f: impl FnOnce(T) -> U,
    ) -> Option<U>;

    /// Extract a nested value, map it to another type, or use a default value.
    fn extract_nested_map_or<T: FromValue, U>(
        &self,
        field: ConfigField,
        nested_key: &str,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U;
}

impl ConfigExtensions for HashMap<ConfigField, Value> {
    fn extract<T: FromValue>(&self, field: ConfigField) -> Option<T> {
        self.get(&field).and_then(|v| T::from_value(v))
    }

    fn extract_or<T: FromValue>(&self, field: ConfigField, default: T) -> T {
        self.extract(field).unwrap_or(default)
    }

    fn extract_nested<T: FromValue>(&self, field: ConfigField, nested_key: &str) -> Option<T> {
        self.get(&field)
            .and_then(|v| v.get(nested_key))
            .and_then(|v| T::from_value(v))
    }

    fn extract_nested_or<T: FromValue>(
        &self,
        field: ConfigField,
        nested_key: &str,
        default: T,
    ) -> T {
        self.extract_nested(field, nested_key).unwrap_or(default)
    }

    fn extract_map<T: FromValue, U>(
        &self,
        field: ConfigField,
        f: impl FnOnce(T) -> U,
    ) -> Option<U> {
        self.extract(field).map(f)
    }

    fn extract_map_or<T: FromValue, U>(
        &self,
        field: ConfigField,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U {
        self.extract(field).map(f).unwrap_or(default)
    }

    fn extract_nested_map<T: FromValue, U>(
        &self,
        field: ConfigField,
        nested_key: &str,
        f: impl FnOnce(T) -> U,
    ) -> Option<U> {
        self.extract_nested(field, nested_key).map(f)
    }

    fn extract_nested_map_or<T: FromValue, U>(
        &self,
        field: ConfigField,
        nested_key: &str,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U {
        self.extract_nested(field, nested_key)
            .map(f)
            .unwrap_or(default)
    }
}

/// A utility for collecting structured miner config data from an API backend.
pub struct ConfigCollector<'a> {
    /// Backend-specific config mapping logic.
    miner: &'a dyn GetConfigsLocations,
    client: &'a dyn APIClient,
    /// Cache of command responses keyed by command string.
    cache: HashMap<MinerCommand, Value>,
}

impl<'a> ConfigCollector<'a> {
    /// Constructs a new `ConfigCollector` with the given backend and API client.
    pub fn new(miner: &'a dyn GetConfigsLocations) -> Self {
        Self {
            miner,
            client: miner,
            cache: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_client(miner: &'a dyn GetConfigsLocations, client: &'a dyn APIClient) -> Self {
        Self {
            miner,
            client,
            cache: HashMap::new(),
        }
    }

    /// Collects **all** available fields from the miner and returns a map of results.
    pub async fn collect_all(&mut self) -> HashMap<ConfigField, Value> {
        self.collect(ConfigField::iter().collect::<Vec<_>>().as_slice())
            .await
    }

    /// Collects only the specified fields from the miner and returns a map of results.
    ///
    /// This method sends only the minimum required set of API commands.
    pub async fn collect(&mut self, fields: &[ConfigField]) -> HashMap<ConfigField, Value> {
        let mut results = HashMap::new();
        let required_commands: Vec<MinerCommand> =
            self.get_required_commands(fields).into_iter().collect();

        // Execute all API commands concurrently
        let responses = futures::future::join_all(
            required_commands
                .iter()
                .map(|cmd| self.client.get_api_result(cmd)),
        )
        .await;

        for (command, response) in required_commands.into_iter().zip(responses) {
            if let Ok(value) = response {
                self.cache.insert(command, value);
            }
        }

        // Extract the data for each field using the cached responses.
        for &field in fields {
            if let Some(value) = self.extract_field(field) {
                results.insert(field, value);
            }
        }

        results
    }

    fn merge(&self, a: &mut Value, b: Value) {
        Self::merge_values(a, b);
    }

    fn merge_values(a: &mut Value, b: Value) {
        match (a, b) {
            (Value::Object(a_map), Value::Object(b_map)) => {
                for (k, v) in b_map {
                    Self::merge_values(a_map.entry(k).or_insert(Value::Null), v);
                }
            }
            (Value::Array(a_array), Value::Array(b_array)) => {
                // Combine arrays by extending
                a_array.extend(b_array);
            }
            (a_slot, b_val) => {
                // For everything else (including mismatched types), overwrite
                *a_slot = b_val;
            }
        }
    }

    /// Determines the unique set of API commands needed for the requested fields.
    ///
    /// Uses the backend's location mappings to identify required commands.
    fn get_required_commands(&self, fields: &[ConfigField]) -> HashSet<MinerCommand> {
        fields
            .iter()
            .flat_map(|&field| self.miner.get_configs_locations(field))
            .map(|(cmd, _)| cmd.clone())
            .collect()
    }

    /// Attempts to extract the value for a specific field from the cached command responses.
    ///
    /// Uses the extractor function and key associated with the field for parsing.
    fn extract_field(&self, field: ConfigField) -> Option<Value> {
        let mut success: Vec<Value> = Vec::new();
        for (command, extractor) in self.miner.get_configs_locations(field) {
            if let Some(response_data) = self.cache.get(&command)
                && let Some(value) = (extractor.func)(response_data, extractor.key)
            {
                match extractor.tag {
                    Some(tag) => {
                        let tag = tag.to_string();
                        success.push(json!({ tag: value.clone() }));
                    }
                    None => {
                        success.push(value.clone());
                    }
                }
            }
        }
        if success.is_empty() {
            None
        } else {
            let mut response = json!({});
            for value in success {
                self.merge(&mut response, value)
            }
            Some(response)
        }
    }
}
