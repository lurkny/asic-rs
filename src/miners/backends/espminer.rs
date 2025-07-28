use std::net::IpAddr;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use macaddr::MacAddr;
use measurements::{AngularVelocity, Frequency, Power, Temperature, Voltage};

use crate::data::board::{BoardData, ChipData};
use crate::data::device::MinerFirmware::Stock;
use crate::data::device::MinerMake::BitAxe;
use crate::data::device::{DeviceInfo, HashAlgorithm, MinerHardware, MinerModel};
use crate::data::fan::FanData;
use crate::data::hashrate::{HashRate, HashRateUnit};
use crate::data::miner::MinerData;
use crate::miners::api::web::esp_web_api::EspWebApi;
use crate::miners::backends::traits::GetMinerData;
use crate::miners::data::{
    DataCollector, DataExtractor, DataField, DataLocation, get_by_key, get_by_pointer,
};

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

        // Extract hashrate and convert to HashRate structure
        let hashrate = data
            .get(&DataField::Hashrate)
            .and_then(|v| v.as_f64())
            .map(|f| HashRate {
                value: f,
                unit: HashRateUnit::GigaHash,
                algo: String::from("SHA256"),
            });

        let total_chips = data
            .get(&DataField::TotalChips)
            .and_then(|v| v.as_u64())
            .map(|u| u as u16);

        let wattage = data
            .get(&DataField::Wattage)
            .and_then(|v| v.as_f64())
            .map(Power::from_watts);

        let average_temperature = data
            .get(&DataField::AverageTemperature)
            .and_then(|v| v.as_f64())
            .map(Temperature::from_celsius);

        let efficiency = match (hashrate.as_ref(), wattage.as_ref()) {
            (Some(hr), Some(w)) => {
                let hashrate_th = hr.value / 1000.0;
                Some(w.as_watts() / hashrate_th)
            },
            _ => None,
        };

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

        // Determine if the miner is actively mining based on hashrate
        let is_mining = hashrate.as_ref().map_or(false, |hr| hr.value > 0.0);

        // Get hardware specifications based on the miner model
        let miner_hardware = MinerHardware::from(&self.model);

        let hashboards = {
            let board_voltage = data.get(&DataField::Hashboards).and_then(|hashboard_data| {
                hashboard_data.get("voltage")
                    .and_then(|voltage_value| voltage_value.as_f64())
                    .map(Voltage::from_millivolts)
            });
            
            let board_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|hashboard_data| hashboard_data.get("vrTemp"))
                .and_then(|temp_value| temp_value.as_f64())
                .map(Temperature::from_celsius);
                
            let board_frequency = data
                .get(&DataField::Hashboards)
                .and_then(|hashboard_data| hashboard_data.get("frequency"))
                .and_then(|freq_value| freq_value.as_f64())
                .map(Frequency::from_megahertz);
                
            let chip_temperature = data
                .get(&DataField::Hashboards)
                .and_then(|hashboard_data| hashboard_data.get("temp"))
                .and_then(|temp_value| temp_value.as_f64())
                .map(Temperature::from_celsius);
                
            let board_hashrate = hashrate.clone();

            let chip_info = ChipData {
                position: 0,
                temperature: chip_temperature,
                voltage: board_voltage,
                frequency: board_frequency,
                tuned: None,
                working: Some(true),
                hashrate: board_hashrate.clone(),
            };

            let board_data = BoardData {
                position: 0,
                hashrate: board_hashrate,
                expected_hashrate: None,
                board_temperature,
                intake_temperature: None,
                outlet_temperature: None,
                expected_chips: miner_hardware.chips,
                working_chips: total_chips,
                serial_number: None,
                chips: vec![chip_info],
                voltage: board_voltage,
                frequency: board_frequency,
                tuned: None,
                active: Some(true),
            };
            
            vec![board_data]
        };

        MinerData {
            // Version information
            schema_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            
            // Network identification
            ip: self.web.ip.clone().parse().unwrap(),
            mac,
            
            // Device identification
            device_info: DeviceInfo::new(BitAxe, self.model.clone(), Stock, HashAlgorithm::SHA256),
            serial_number: None,
            hostname,
            
            // Version information
            api_version,
            firmware_version,
            control_board_version,
            
            // Hashboard information
            expected_hashboards: miner_hardware.boards,
            hashboards,
            hashrate,
            
            // Chip information
            expected_chips: miner_hardware.chips,
            total_chips,
            
            // Cooling information
            expected_fans: miner_hardware.fans,
            fans,
            psu_fans: vec![],
            average_temperature,
            fluid_temperature: None,
            
            // Power information
            wattage,
            wattage_limit: None,
            efficiency,
            
            // Status information
            light_flashing: None,
            messages: vec![],
            uptime,
            is_mining,
            
            pools: vec![],
        }
    }

    fn get_locations(&self, data_field: DataField) -> &'static [DataLocation] {
        const SYSTEM_INFO_CMD: &str = "system/info";
        const ASIC_INFO_CMD: &str = "system/asic";

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
            DataField::Hashboards => &[(
                SYSTEM_INFO_CMD,
                DataExtractor {
                    func: get_by_pointer,
                    key: Some(""),
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
                        key: Some("asicCount"),
                    },
                ),
                (
                    ASIC_INFO_CMD,
                    DataExtractor {
                        func: get_by_key,
                        key: Some("asicCount"),
                    },
                ),
            ],
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
