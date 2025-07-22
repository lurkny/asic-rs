use crate::data::board::BoardData;
use crate::data::device::HashAlgorithm::SHA256;
use crate::data::device::MinerFirmware::Stock;
use crate::data::device::MinerMake::BitAxe;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerFirmware, MinerModel};
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::miner::MinerData;
use crate::miners::api::web::ESPMinerWebAPI::{ESPMinerError, ESPMinerWebAPI};
use crate::miners::backends::traits::GetMinerData;
use crate::miners::data::{DataCollector, DataExtractor, DataField, DataLocation, get_by_key};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{Frequency, Power, Temperature, Voltage};
use serde::{Deserialize, Serialize};
use serde_json::error::Category::Data;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strum::EnumIter;
use strum::IntoEnumIterator;
use tokio::time::Instant;

pub struct ESPMiner {
    model: MinerModel,
    web: ESPMinerWebAPI,
}

impl ESPMiner {
    pub fn new(ip: IpAddr, model: MinerModel) -> Self {
        ESPMiner {
            model,
            web: ESPMinerWebAPI::new(ip.to_string(), 80),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShareRejectReason {
    pub message: String,
    pub count: u32,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
pub struct BitAxeSysInfo {
    pub power: f64,
    pub voltage: f64,
    pub current: f64,
    pub temp: f64,
    pub vrTemp: u32,
    pub maxPower: u32,
    pub nominalVoltage: u32,
    pub hashRate: f64,
    pub expectedHashrate: u32,
    pub bestDiff: String,
    pub bestSessionDiff: String,
    pub poolDifficulty: u32,
    pub isUsingFallbackStratum: u8,
    pub isPSRAMAvailable: u8,
    pub freeHeap: u64,
    pub coreVoltage: u32,
    pub coreVoltageActual: u32,
    pub frequency: u32,
    pub ssid: String,
    pub macAddr: String,
    pub hostname: String,
    pub wifiStatus: String,
    pub wifiRSSI: i32,
    pub apEnabled: u8,
    pub sharesAccepted: u32,
    pub sharesRejected: u32,
    pub sharesRejectedReasons: Vec<ShareRejectReason>,
    pub uptimeSeconds: u64,
    pub smallCoreCount: u32,
    #[serde(rename = "ASICModel")]
    pub asic_model: String,
    pub stratumURL: String,
    pub stratumPort: u32,
    pub stratumUser: String,
    pub stratumSuggestedDifficulty: u32,
    pub stratumExtranonceSubscribe: u8,
    pub fallbackStratumURL: String,
    pub fallbackStratumPort: u32,
    pub fallbackStratumUser: String,
    pub fallbackStratumSuggestedDifficulty: u32,
    pub fallbackStratumExtranonceSubscribe: u8,
    pub responseTime: f64,
    pub version: String,
    pub axeOSVersion: String,
    pub idfVersion: String,
    pub boardVersion: String,
    pub runningPartition: String,
    pub overheat_mode: u8,
    pub overclockEnabled: u8,
    pub display: String,
    pub rotation: u8,
    pub invertscreen: u8,
    pub displayTimeout: i32,
    pub autofanspeed: u8,
    pub fanspeed: u32,
    pub temptarget: u32,
    pub fanrpm: u32,
    pub statsFrequency: u32,
}

#[async_trait]
impl GetMinerData for ESPMiner {
    async fn get_data(&self) -> MinerData {
        let mut collector = DataCollector::new(self, &self.web);
        let data = collector.collect(&*vec![DataField::Mac]).await;

        println!("{:?}", data);

        // Parse MAC address if available, otherwise set to None
        let mac = data
            .get(&DataField::Mac)
            .and_then(|v| v.as_str())
            .and_then(|s| MacAddr::from_str(s).ok());

        MinerData {
            schema_version: "".to_string(),
            timestamp: 0,
            ip: self.web.ip.clone().parse().unwrap(),
            mac,
            device_info: DeviceInfo::new(BitAxe, self.model.clone(), Stock, HashAlgorithm::SHA256),
            serial_number: None,
            hostname: None,
            api_version: None,
            firmware_version: None,
            control_board_version: None,
            expected_hashboards: None,
            hashboards: vec![],
            hashrate: None,
            expected_chips: None,
            total_chips: None,
            expected_fans: None,
            fans: vec![],
            psu_fans: vec![],
            average_temperature: None,
            fluid_temperature: None,
            wattage: None,
            wattage_limit: None,
            efficiency: None,
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
            DataField::Fans => &[],
            DataField::Uptime => &[],
            DataField::Pools => &[],
            DataField::Errors => &[],
            DataField::FaultLight => &[],
            DataField::IsMining => &[],
        }
    }
}
