use crate::data::miner::MinerData;
use crate::miners::data::{DataField, DataLocation};
use async_trait::async_trait;

#[async_trait]
pub trait GetMinerData: Send + Sync {
    async fn get_data(&self) -> MinerData;

    fn get_locations(&self, data_field: DataField) -> &'static [DataLocation];
}
