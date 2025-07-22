use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub mod rpc;
pub mod web;

#[async_trait]
pub(crate) trait ApiClient: Send + Sync {
    async fn send_command(&self, command: &'static str) -> Result<Value, String>;
}
