use async_trait::async_trait;
use serde_json::Value;

pub mod rpc;
pub mod web;

#[async_trait]
pub trait ApiClient: Send + Sync {
    async fn send_command(&self, command: &'static str) -> Result<Value, String>;
}
