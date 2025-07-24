use crate::miners::api::ApiClient;
use crate::miners::backends::traits::GetMinerData;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use strum::{EnumIter, IntoEnumIterator};
#[derive(Debug, Clone, Hash, Eq, PartialEq, Copy, EnumIter)]
pub enum DataField {
    Mac,
    ApiVersion,
    FwVersion,
    Hostname,
    Hashrate,
    ExpectedHashrate,
    Hashboards,
    Wattage,
    Fans,
    Uptime,
    Pools,
    Errors,
    FaultLight,
    IsMining,
}

type ExtractorFn = fn(&Value, Option<&'static str>) -> Option<Value>;

#[derive(Clone, Copy)]
pub struct DataExtractor {
    pub func: ExtractorFn,
    pub key: Option<&'static str>,
}

pub type DataLocation = (&'static str, DataExtractor);

pub fn get_by_key(data: &Value, key: Option<&str>) -> Option<Value> {
    data.get(key?.to_string()).cloned()
}


pub fn get_by_pointer(data: &Value, pointer: Option<&str>) -> Option<Value> {
    data.pointer(pointer?).cloned()
}

pub struct DataCollector<'a> {
    miner: &'a dyn GetMinerData,
    api_client: &'a dyn ApiClient,
    cache: HashMap<String, Value>,
}

impl<'a> DataCollector<'a> {
    pub fn new(miner: &'a dyn GetMinerData, api_client: &'a dyn ApiClient) -> Self {
        Self {
            miner,
            api_client,
            cache: HashMap::new(),
        }
    }

    pub async fn collect_all(&mut self) -> HashMap<DataField, Value> {
        self.collect(DataField::iter().collect::<Vec<_>>().as_slice()).await
    }
    pub async fn collect(&mut self, fields: &[DataField]) -> HashMap<DataField, Value> {
        let mut results = HashMap::new();
        let required_commands = self.get_required_commands(fields);

        for command in required_commands {
            if let Ok(response) = self.api_client.send_command(command).await {
                self.cache.insert(command.to_string(), response);
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

    // Determines the unique set of API commands needed for the requested fields.
    fn get_required_commands(&self, fields: &[DataField]) -> HashSet<&'static str> {
        fields
            .iter()
            .flat_map(|&field| self.miner.get_locations(field))
            .map(|(cmd, _)| *cmd)
            .collect()
    }

    // Tries available locations to extract a single data field.
    fn extract_field(&self, field: DataField) -> Option<Value> {
        for (command, extractor) in self.miner.get_locations(field) {
            if let Some(response_data) = self.cache.get(*command) {
                if let Some(value) = (extractor.func)(response_data, extractor.key) {
                    return Some(value); // Return the first successful extraction.
                }
            }
        }
        None
    }
}
