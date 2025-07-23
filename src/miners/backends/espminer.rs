use crate::data::board::BoardData;
use crate::data::device::HashAlgorithm::SHA256;
use crate::data::device::MinerFirmware::Stock;
use crate::data::device::MinerMake::BitAxe;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::miner::MinerData;
use crate::miners::api::web::ESPMinerWebAPI::{ESPMinerError, EspWebApi};
use crate::miners::backends::traits::GetMinerData;
use crate::miners::data::{DataCollector, DataExtractor, DataField, DataLocation, get_by_key};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};
use serde::{Deserialize, Serialize};
use serde_json::error::Category::Data;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strum::EnumIter;
use strum::IntoEnumIterator;
use tokio::time::Instant;
use crate::data::fan::FanData;

pub struct ESPMiner {
    model: MinerModel,
    web: EspWebApi,
}

impl ESPMiner {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        ESPMiner {
            model,
            web: EspWebApi::new(ip.to_string(), 80),
        }
    }
}

#[async_trait]
impl GetMinerData for ESPMiner {
    async fn get_data(&self) -> MinerData {
        let mut collector = DataCollector::new(self, &self.web);
        let data = collector.collect_all().await;

        let mac = data
            .get(&DataField::Mac)
            .and_then(|v| v.as_str())
            .and_then(|s| MacAddr::from_str(s).ok());

        let hostname = data
            .get(&DataField::Hostname)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let api_version = data
            .get(&DataField::ApiVersion)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let firmware_version = data
            .get(&DataField::FwVersion)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let hashrate = data
            .get(&DataField::Hashrate)
            .and_then(|v| v.as_f64())
            .map(|f| HashRate {
                value: f,
                unit: HashRateUnit::MegaHash,
                algo: String::from("SHA256"),
            });

        let wattage = data
            .get(&DataField::Wattage)
            .and_then(|v| v.as_f64())
            .map(|f| Power::from_watts(f));

        // Calculate efficiency if both hashrate and wattage are available
        let efficiency = match (hashrate.clone(), wattage.clone()) {
            (Some(hr), Some(w)) => Some(w / hr),
            _ => None,
        };

        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_secs();


        let fans = data.get(&DataField::Fans)
            .and_then(|v| v.as_f64())
            .map(|f| vec![FanData { position: 0, rpm: AngularVelocity::from_rpm(f) }])
            .unwrap_or_default();

        MinerData {
            schema_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            ip: self.web.ip.clone().parse().unwrap(),
            mac,
            device_info: DeviceInfo::new(BitAxe, self.model.clone(), Stock, HashAlgorithm::SHA256),
            serial_number: None,
            hostname,
            api_version,
            firmware_version,
            control_board_version: None,
            expected_hashboards: None,
            hashboards: vec![],
            hashrate,
            expected_chips: None,
            total_chips: None,
            expected_fans: None,
            fans: fans,
            psu_fans: vec![],
            average_temperature: None,
            fluid_temperature: None,
            wattage,
            wattage_limit: None,
            efficiency,
            light_flashing: None,
            messages: vec![],
            uptime: None,
            is_mining: false,
            pools: vec![],
        }
    }

    fn get_locations(&self, data_field: DataField) -> &'static [DataLocation] {
        const CMD: &str = "system/info";

        match data_field {
            DataField::Mac => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("macAddr"),
                },
            )],
            DataField::ApiVersion => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: None,
                },
            )],
            DataField::FwVersion => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: None,
                },
            )],
            DataField::Hostname => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hostname"),
                },
            )],
            DataField::Hashrate => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hashRate"),
                },
            )],
            DataField::ExpectedHashrate => &[],
            DataField::Hashboards => &[],
            DataField::Wattage => &[(
                CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("power"),
                },
            )],
            DataField::Fans => &[(CMD, DataExtractor{
                func: get_by_key,
                key: Some("fanSpeed"),
            })],
            DataField::Uptime => &[],
            DataField::Pools => &[],
            DataField::Errors => &[],
            DataField::FaultLight => &[],
            DataField::IsMining => &[],
        }
    }
}
