use crate::data::board::BoardData;
use crate::data::device::{
    DeviceInfo, MinerFirmware, MinerModel,
};
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::miner::MinerData;
use crate::miners::api::web::ESPMinerWebAPI::{ESPMinerError, ESPMinerWebAPI};
use crate::miners::backends::traits::GetMinerData;
use async_trait::async_trait;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use macaddr::MacAddr;
use measurements::{Frequency, Power, Temperature, Voltage};
use serde::{Deserialize, Serialize};
use crate::data::device::HashAlgorithm::SHA256;
use crate::data::device::MinerMake::BitAxe;

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
        let system_info_res = self.web.system_info().await
            .and_then(|val| {
                serde_json::from_value::<BitAxeSysInfo>(val)
                    .map_err(|_| ESPMinerError::WebError)
            });

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |dur| dur.as_secs());

        let ip = self.web.ip.parse::<IpAddr>().unwrap(); //This really shouldn't fail.

        let device_info = DeviceInfo::new(BitAxe, self.model.clone(), MinerFirmware::Stock, SHA256);

        // Handle error case by returning partial MinerData
        let sys_info = match system_info_res {
            Ok(info) => info,
            Err(_) => {
                return MinerData {
                    schema_version: env!("CARGO_PKG_VERSION").to_string(),
                    timestamp,
                    ip,
                    mac: None,
                    device_info,
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
                };
            }
        };

        // Build hashrate if available
        let hashrate = Some(HashRate {
            value: sys_info.hashRate,
            unit: HashRateUnit::GigaHash,
            algo: "SHA256".to_string(), // Use enum's Display if available, but string for now
        });

        // Build hashboard data
        let hashboards = vec![BoardData {
            position: 0,
            hashrate: hashrate.clone(),
            expected_hashrate: Some(HashRate {
                value: sys_info.expectedHashrate as f64,
                unit: HashRateUnit::GigaHash,
                algo: "SHA256".to_string(),
            }),
            board_temperature: Some(Temperature::from_celsius(sys_info.temp)),
            intake_temperature: None,
            outlet_temperature: None,
            expected_chips: None,
            working_chips: None,
            serial_number: None,
            chips: vec![],
            voltage: Some(Voltage::from_volts(sys_info.voltage)), // TODO: Volts or mV?
            frequency: Some(Frequency::from_megahertz(sys_info.frequency as f64)), // TODO: assuming Mhz here, dont know for sure.
            tuned: None,
            active: None,
        }];

        MinerData {
            schema_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            ip,
            mac: MacAddr::from_str(&sys_info.macAddr).ok(),
            device_info,
            serial_number: None,
            hostname: Some(sys_info.hostname.clone()),
            api_version: None,
            firmware_version: None, // TODO: Could set from sys_info.version or sys_info.axeOSVersion
            control_board_version: None,
            expected_hashboards: None,
            hashboards,
            hashrate,
            expected_chips: None,
            total_chips: None,
            expected_fans: None,
            fans: vec![],
            psu_fans: vec![],
            average_temperature: None,
            fluid_temperature: None,
            wattage: Some(Power::from_watts(sys_info.power)),
            wattage_limit: None,
            efficiency: None,
            light_flashing: None,
            messages: vec![],
            uptime: Some(Duration::from_secs(sys_info.uptimeSeconds)),
            is_mining: (sys_info.sharesAccepted > 0 || sys_info.sharesRejected > 0) && sys_info.hashRate > 0.0,
            pools: vec![], // TODO: Could populate from stratumURL
        }
    }
}