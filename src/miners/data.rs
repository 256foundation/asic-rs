use crate::miners::{
    backends::traits::{APIClient, MinerInterface},
    commands::MinerCommand,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use strum::{EnumIter, IntoEnumIterator};

/// Represents the individual pieces of data that can be queried from a miner device.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy, EnumIter)]
pub enum DataField {
    /// Schema version of the miner data.
    SchemaVersion,
    /// Timestamp of when the data was collected.
    Timestamp,
    /// IP address of the miner.
    Ip,
    /// MAC address of the miner.
    Mac,
    /// Information about the miner's device.
    DeviceInfo,
    /// Serial number of the miner.
    SerialNumber,
    /// Hostname assigned to the miner.
    Hostname,
    /// Version of the miner's API.
    ApiVersion,
    /// Firmware version of the miner.
    FirmwareVersion,
    /// Control board version of the miner.
    ControlBoardVersion,
    /// Details about the hashboards (e.g., temperatures, chips, etc.).
    Hashboards,
    /// Current hashrate reported by the miner.
    Hashrate,
    /// Expected hashrate for the miner.
    ExpectedHashrate,
    /// Fan speed or fan configuration.
    Fans,
    /// PSU fan speed or configuration.
    PsuFans,
    /// Average temperature reported by the miner.
    AverageTemperature,
    /// Fluid temperature reported by the miner.
    FluidTemperature,
    /// Current power consumption in watts.
    Wattage,
    /// Configured power limit in watts.
    WattageLimit,
    /// Efficiency of the miner (e.g., J/TH).
    Efficiency,
    /// Whether the fault or alert light is flashing.
    LightFlashing,
    /// Messages reported by the miner (e.g., errors or warnings).
    Messages,
    /// Uptime in seconds.
    Uptime,
    /// Whether mining is enabled (i.e., not sleeping/paused/stopped).
    MiningMode,
    /// Whether the miner is currently hashing.
    IsMining,
    /// Pool configuration (addresses, statuses, etc.).
    Pools,
}

/// A function pointer type that takes a JSON `Value` and an optional key,
/// returning the extracted value if found.
type ExtractorFn = for<'a> fn(&'a Value, Option<&'static str>) -> Option<&'a Value>;

/// Describes how to extract a specific value from a command's response.
///
/// Created by a backend and used to locate a field within a JSON structure.
#[derive(Clone, Copy)]
pub struct DataExtractor {
    /// Function used to extract data from a JSON response.
    pub func: ExtractorFn,
    /// Optional key or pointer within the response to extract.
    pub key: Option<&'static str>,
    /// Optional tag to move the extracted value to
    pub tag: Option<&'static str>,
}

/// Alias for a tuple describing the API command and the extractor used to parse its result.
pub type DataLocation = (MinerCommand, DataExtractor);

/// Extracts a value from a JSON object using a key (flat lookup).
///
/// Returns `None` if the key is `None` or not found in the object.
pub fn get_by_key<'a>(data: &'a Value, key: Option<&str>) -> Option<&'a Value> {
    data.get(key?.to_string())
}

/// Extracts a value from a JSON object using a JSON pointer path.
///
/// Returns `None` if the pointer is `None` or the path doesn't exist.
pub fn get_by_pointer<'a>(data: &'a Value, pointer: Option<&str>) -> Option<&'a Value> {
    data.pointer(pointer?)
}

/// A trait for types that can be extracted from a JSON Value.
pub trait FromValue: Sized {
    /// Attempts to convert a JSON Value to Self.
    fn from_value(value: &Value) -> Option<Self>;
}

// Implement FromValue for common types
impl FromValue for String {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_str().map(String::from)
    }
}

impl FromValue for f64 {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_f64()
    }
}

impl FromValue for u64 {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_u64()
    }
}

impl FromValue for i64 {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_i64()
    }
}

impl FromValue for bool {
    fn from_value(value: &Value) -> Option<Self> {
        // Try to get as bool first
        value.as_bool().or_else(|| {
            // If not a bool, try to interpret as a number (0 = false, non-zero = true)
            value
                .as_u64()
                .map(|n| n != 0)
                .or_else(|| value.as_i64().map(|n| n != 0))
        })
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_array()?.iter().map(|v| T::from_value(v)).collect()
    }
}

/// Extension trait for HashMap<DataField, &Value> to provide cleaner value extraction.
pub trait DataExtensions {
    /// Extract a value of type T from the data map for the given field.
    fn extract<T: FromValue>(&self, field: DataField) -> Option<T>;

    /// Extract a value of type T from the data map for the given field, with a default value.
    fn extract_or<T: FromValue>(&self, field: DataField, default: T) -> T;

    /// Extract a nested value of type T from the data map for the given field and nested key.
    fn extract_nested<T: FromValue>(&self, field: DataField, nested_key: &str) -> Option<T>;

    /// Extract a nested value of type T from the data map for the given field and nested key, with a default value.
    fn extract_nested_or<T: FromValue>(&self, field: DataField, nested_key: &str, default: T) -> T;

    /// Extract a value and map it to another type using the provided function.
    fn extract_map<T: FromValue, U>(&self, field: DataField, f: impl FnOnce(T) -> U) -> Option<U>;

    /// Extract a value, map it to another type, or use a default value.
    fn extract_map_or<T: FromValue, U>(
        &self,
        field: DataField,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U;

    /// Extract a nested value and map it to another type using the provided function.
    fn extract_nested_map<T: FromValue, U>(
        &self,
        field: DataField,
        nested_key: &str,
        f: impl FnOnce(T) -> U,
    ) -> Option<U>;

    /// Extract a nested value, map it to another type, or use a default value.
    fn extract_nested_map_or<T: FromValue, U>(
        &self,
        field: DataField,
        nested_key: &str,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U;
}

impl DataExtensions for HashMap<DataField, Value> {
    fn extract<T: FromValue>(&self, field: DataField) -> Option<T> {
        self.get(&field).and_then(|v| T::from_value(v))
    }

    fn extract_or<T: FromValue>(&self, field: DataField, default: T) -> T {
        self.extract(field).unwrap_or(default)
    }

    fn extract_nested<T: FromValue>(&self, field: DataField, nested_key: &str) -> Option<T> {
        self.get(&field)
            .and_then(|v| v.get(nested_key))
            .and_then(|v| T::from_value(v))
    }

    fn extract_nested_or<T: FromValue>(&self, field: DataField, nested_key: &str, default: T) -> T {
        self.extract_nested(field, nested_key).unwrap_or(default)
    }

    fn extract_map<T: FromValue, U>(&self, field: DataField, f: impl FnOnce(T) -> U) -> Option<U> {
        self.extract(field).map(f)
    }

    fn extract_map_or<T: FromValue, U>(
        &self,
        field: DataField,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U {
        self.extract(field).map(f).unwrap_or(default)
    }

    fn extract_nested_map<T: FromValue, U>(
        &self,
        field: DataField,
        nested_key: &str,
        f: impl FnOnce(T) -> U,
    ) -> Option<U> {
        self.extract_nested(field, nested_key).map(f)
    }

    fn extract_nested_map_or<T: FromValue, U>(
        &self,
        field: DataField,
        nested_key: &str,
        default: U,
        f: impl FnOnce(T) -> U,
    ) -> U {
        self.extract_nested(field, nested_key)
            .map(f)
            .unwrap_or(default)
    }
}

/// A utility for collecting structured miner data from an API backend.
pub struct DataCollector<'a> {
    /// Backend-specific data mapping logic.
    miner: &'a dyn MinerInterface,
    client: &'a dyn APIClient,
    /// Cache of command responses keyed by command string.
    cache: HashMap<MinerCommand, Value>,
}

impl<'a> DataCollector<'a> {
    /// Constructs a new `DataCollector` with the given backend and API client.
    pub fn new(miner: &'a dyn MinerInterface) -> Self {
        Self {
            miner,
            client: miner,
            cache: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_with_client(
        miner: &'a dyn MinerInterface,
        client: &'a dyn APIClient,
    ) -> Self {
        Self {
            miner,
            client,
            cache: HashMap::new(),
        }
    }

    /// Collects **all** available fields from the miner and returns a map of results.
    pub async fn collect_all(&mut self) -> HashMap<DataField, Value> {
        let fields = DataField::iter()
            .filter(|f| *f != DataField::IsMining)
            .collect::<Vec<_>>();
        self.collect(fields.as_slice()).await
    }

    /// Collects only the specified fields from the miner and returns a map of results.
    ///
    /// This method sends only the minimum required set of API commands.
    pub async fn collect(&mut self, fields: &[DataField]) -> HashMap<DataField, Value> {
        let mut results = HashMap::new();
        let required_commands = self.get_required_commands(fields);

        for command in required_commands {
            if let Ok(response) = self.client.get_api_result(&command).await {
                self.cache.insert(command, response);
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
    fn get_required_commands(&self, fields: &[DataField]) -> HashSet<MinerCommand> {
        fields
            .iter()
            .flat_map(|&field| self.miner.get_locations(field))
            .map(|(cmd, _)| cmd.clone())
            .collect()
    }

    /// Attempts to extract the value for a specific field from the cached command responses.
    ///
    /// Uses the extractor function and key associated with the field for parsing.
    fn extract_field(&self, field: DataField) -> Option<Value> {
        let mut success: Vec<Value> = Vec::new();
        for (command, extractor) in self.miner.get_locations(field) {
            if let Some(response_data) = self.cache.get(&command)
                && let Some(value) = (extractor.func)(response_data, extractor.key)
            {
                match extractor.tag {
                    Some(tag) => {
                        let tag = tag.to_string();
                        success.push(json!({ tag: value.clone() }).clone());
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
