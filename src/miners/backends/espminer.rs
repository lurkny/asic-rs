use crate::data::device::MinerFirmware::Stock;
use crate::data::device::MinerMake::BitAxe;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerModel};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::miner::MinerData;
use crate::miners::api::web::esp_web_api::EspWebApi;
use crate::miners::backends::traits::GetMinerData;
use crate::miners::data::{DataCollector, DataExtractor, DataField, DataLocation, get_by_key};
use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Power, Temperature};
use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
            .get(&DataField::FirmwareVersion)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let control_board_version = data
            .get(&DataField::ControlBoardVersion)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let expected_hashboards = data
            .get(&DataField::ExpectedHashboards)
            .and_then(|v| v.as_u64())
            .map(|u| u as u8);

        let hashrate = data
            .get(&DataField::Hashrate)
            .and_then(|v| v.as_f64())
            .map(|f| HashRate {
                value: f,
                unit: HashRateUnit::MegaHash,
                algo: String::from("SHA256"),
            });

        let expected_chips = data
            .get(&DataField::TotalChips)
            .and_then(|v| v.as_u64())
            .map(|u| u as u16);

        let total_chips = data
            .get(&DataField::TotalChips)
            .and_then(|v| v.as_u64())
            .map(|u| u as u16);

        let wattage = data
            .get(&DataField::Wattage)
            .and_then(|v| v.as_f64())
            .map(|f| Power::from_watts(f));

        let average_temperature = data
            .get(&DataField::AverageTemperature)
            .and_then(|v| v.as_f64())
            .map(Temperature::from_celsius);

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

        let fans = data
            .get(&DataField::Fans)
            .and_then(|v| v.as_f64())
            .map(|f| {
                vec![FanData {
                    position: 0,
                    rpm: AngularVelocity::from_rpm(f),
                }]
            })
            .unwrap_or_default();

        let uptime = data
            .get(&DataField::Uptime)
            .and_then(|v| v.as_u64())
            .map(Duration::from_secs);

        let is_mining = hashrate.as_ref().map_or(false, |hr| hr.value > 0.0);

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
            control_board_version,
            expected_hashboards,
            hashboards: vec![],
            hashrate,
            expected_chips,
            total_chips,
            expected_fans: Some(1),
            fans,
            psu_fans: vec![],
            average_temperature,
            fluid_temperature: None,
            wattage,
            wattage_limit: None,
            efficiency,
            light_flashing: None,
            messages: vec![],
            uptime,
            is_mining,
            pools: vec![],
        }
    }

    fn get_locations(&self, data_field: DataField) -> &'static [DataLocation] {
        const SYSTEM_INFO_CMD: &str = "system/info";
        const ASIC_INFO_CMD: &str = "asic/info";

        match data_field {
            DataField::Mac => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("macAddr"),
                },
            )],
            DataField::Hostname => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hostname"),
                },
            )],
            DataField::ApiVersion => &[],
            DataField::FirmwareVersion => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("version"),
                },
            )],
            DataField::ControlBoardVersion => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("boardVersion"),
                },
            )],
            DataField::ExpectedHashboards => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("asicCount"),
                },
            )],
            DataField::Hashboards => &[(
                ASIC_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: None,
                },
            )],
            DataField::Hashrate => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("hashRate"),
                },
            )],
            DataField::TotalChips => &[
                (
                    SYSTEM_INFO_CMD,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("smallCoreCount"),
                    },
                ),
                (
                    ASIC_INFO_CMD,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("smallCoreCount"),
                    },
                ),
            ],
            DataField::ExpectedFans => &[],
            DataField::Fans => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("fanrpm"),
                },
            )],
            DataField::AverageTemperature => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("temp"),
                },
            )],
            DataField::Wattage => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("power"),
                },
            )],
            DataField::Uptime => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_key,
                    key: Some("uptimeSeconds"),
                },
            )],
            _ => &[],
        }
    }
}
