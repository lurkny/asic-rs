use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[async_trait]
pub(crate) trait SendWebCommand {
    async fn send_web_command<T, P>(
        &self,
        command: &'static str,
        param: Option<P>,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
        P: Serialize + Send;
}
