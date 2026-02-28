use crate::iot::types::{IoTSector, IoTTransaction, Location};
use chrono::{Duration, Utc};

/// 线性同余生成器 (LCG) - 确定性随机数生成
pub struct LCG {
    seed: u64,
}

impl LCG {
    pub fn new(seed: u64) -> Self {
        LCG { seed }
    }

    pub fn next(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223) % 4294967296;
        self.seed as f64 / 4294967296.0
    }

    pub fn next_int(&mut self, max: usize) -> usize {
        (self.next() * max as f64) as usize
    }
}

/// IoT 数据生成器
pub struct IoTGenerator {
    locations: Vec<Location>,
}

impl IoTGenerator {
    pub fn new() -> Self {
        IoTGenerator {
            locations: Location::all(),
        }
    }

    /// 生成传感器数据载荷
    fn generate_payload(&self, sector: &IoTSector, rng: &mut LCG) -> String {
        match sector {
            IoTSector::SmartCity => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let density = ["low", "medium", "high"][rng.next_int(3)];
                        let speed = rng.next_int(80);
                        format!(
                            r#"{{"traffic_density": "{}", "avg_speed": {}kmh}}"#,
                            density, speed
                        )
                    }
                    1 => {
                        let co2 = 350 + rng.next_int(150);
                        let pm25 = rng.next_int(50);
                        format!(r#"{{"co2_ppm": {}, "pm25": {}}}"#, co2, pm25)
                    }
                    _ => {
                        let occupancy = rng.next() > 0.5;
                        let zone = rng.next_int(100);
                        format!(r#"{{"occupancy": {}, "zone": "A-{}"}}"#, occupancy, zone)
                    }
                }
            }
            IoTSector::Industrial => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let pressure = 900 + rng.next_int(300);
                        format!(r#"{{"pressure_psi": {}, "status": "optimal"}}"#, pressure)
                    }
                    1 => {
                        let rpm = 1200 + rng.next_int(500);
                        let vibration = rng.next();
                        format!(r#"{{"rpm": {}, "vibration": {:.3}}}"#, rpm, vibration)
                    }
                    _ => {
                        let temp = 40 + rng.next_int(40);
                        let cooling = rng.next() > 0.3;
                        format!(r#"{{"temp_c": {}, "cooling": {}}}"#, temp, cooling)
                    }
                }
            }
            IoTSector::Agriculture => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let moisture = 20 + rng.next_int(60);
                        let ph = 5.5 + rng.next() * 2.0;
                        format!(r#"{{"moisture": "{}%", "ph": {:.1}}}"#, moisture, ph)
                    }
                    1 => {
                        let wind = rng.next_int(30);
                        let rain = rng.next_int(10);
                        format!(r#"{{"wind_speed": "{}kmh", "rain_mm": {}}}"#, wind, rain)
                    }
                    _ => {
                        let lat = rng.next() * 180.0 - 90.0;
                        let lng = rng.next() * 360.0 - 180.0;
                        format!(r#"{{"lat": {:.4}, "lng": {:.4}}}"#, lat, lng)
                    }
                }
            }
            IoTSector::Healthcare => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let bpm = 60 + rng.next_int(40);
                        let spo2 = 95 + rng.next_int(5);
                        format!(r#"{{"bpm": {}, "spo2": "{}%"}}"#, bpm, spo2)
                    }
                    1 => {
                        let glucose = 80 + rng.next_int(60);
                        format!(r#"{{"glucose_mgdl": {}, "trend": "stable"}}"#, glucose)
                    }
                    _ => {
                        let steps = rng.next_int(10000);
                        let cal = rng.next_int(500);
                        format!(r#"{{"steps": {}, "cal": {}}}"#, steps, cal)
                    }
                }
            }
            IoTSector::Logistics => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let lat = rng.next() * 180.0 - 90.0;
                        let lng = rng.next() * 360.0 - 180.0;
                        let speed = rng.next_int(100);
                        format!(
                            r#"{{"lat": {:.4}, "lng": {:.4}, "speed": "{}kmh"}}"#,
                            lat, lng, speed
                        )
                    }
                    1 => {
                        let temp = -20 + rng.next_int(30) as i32;
                        let humidity = rng.next_int(100);
                        format!(r#"{{"temp_c": {}, "humidity": "{}%"}}"#, temp, humidity)
                    }
                    _ => {
                        let pkg_id = rng.next_int(10000);
                        format!(r#"{{"scan_id": "PKG-{}", "status": "in_transit"}}"#, pkg_id)
                    }
                }
            }
            IoTSector::Energy => {
                let payload_type = rng.next_int(3);
                match payload_type {
                    0 => {
                        let voltage = 220 + rng.next_int(20);
                        let current = rng.next_int(50);
                        format!(r#"{{"voltage": {}, "current": "{}A"}}"#, voltage, current)
                    }
                    1 => {
                        let output = rng.next() * 10.0;
                        let efficiency = 80 + rng.next_int(20);
                        format!(
                            r#"{{"output_kw": {:.2}, "efficiency": "{}%"}}"#,
                            output, efficiency
                        )
                    }
                    _ => {
                        let charge = rng.next_int(100);
                        format!(r#"{{"charge_level": "{}%", "status": "charging"}}"#, charge)
                    }
                }
            }
        }
    }

    /// 生成单个 IoT 交易
    pub fn generate_transaction(&self, index: u64) -> IoTTransaction {
        let mut rng = LCG::new(index + 12345);

        // 选择行业
        let base_index = (index as usize) % 6;
        let sector_index = if rng.next() > 0.3 {
            base_index
        } else {
            rng.next_int(6)
        };
        let sector = IoTSector::from_index(sector_index);

        // 选择设备
        let devices = sector.devices();
        let device_index = rng.next_int(devices.len());
        let device_type = devices[device_index].to_string();

        // 生成载荷
        let data_payload = self.generate_payload(&sector, &mut rng);

        // 生成哈希
        let hash_part1 = (rng.next() * 1e16) as u64;
        let hash_part2 = (rng.next() * 1e16) as u64;
        let hash = format!("0x{:016x}{:016x}", hash_part1, hash_part2);

        // 生成金额
        let amount = (rng.next() * 5.0 * 10000.0).round() / 10000.0;

        // 生成时间戳
        let now = Utc::now();
        let offset = Duration::milliseconds((index * 100) as i64);
        let timestamp = now - offset;

        // 生成位置
        let location_index = rng.next_int(self.locations.len());
        let location = &self.locations[location_index];
        let jitter_lat = (rng.next() - 0.5) * 2.0;
        let jitter_lng = (rng.next() - 0.5) * 2.0;
        let coordinates = (
            location.coords.0 + jitter_lat,
            location.coords.1 + jitter_lng,
        );

        // 生成地址
        let from_part1 = (rng.next() * 1e8) as u64;
        let from_part2 = (rng.next() * 1e8) as u64;
        let to_part1 = (rng.next() * 1e8) as u64;
        let to_part2 = (rng.next() * 1e8) as u64;

        IoTTransaction {
            id: hash.clone(),
            hash,
            tx_type: "DataContribution".to_string(),
            from: format!("0x{:08x}...{:08x}", from_part1, from_part2),
            to: format!("0x{:08x}...{:08x}", to_part1, to_part2),
            amount,
            timestamp: timestamp.to_rfc3339(),
            status: "Confirmed".to_string(),
            sector: sector.display_name().to_string(),
            device_type,
            location: location.name.clone(),
            coordinates,
            data_payload,
        }
    }

    /// 生成 IoT 交易列表
    pub fn generate_transactions(&self, page: u64, limit: u64) -> Vec<IoTTransaction> {
        let start = (page - 1) * limit;
        (0..limit)
            .map(|i| self.generate_transaction(start + i))
            .collect()
    }
}

impl Default for IoTGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_transaction() {
        let generator = IoTGenerator::new();
        let tx = generator.generate_transaction(0);

        assert!(!tx.hash.is_empty());
        assert!(!tx.sector.is_empty());
        assert!(!tx.device_type.is_empty());
        assert!(!tx.data_payload.is_empty());
    }

    #[test]
    fn test_deterministic_generation() {
        let generator = IoTGenerator::new();
        let tx1 = generator.generate_transaction(42);
        let tx2 = generator.generate_transaction(42);

        assert_eq!(tx1.hash, tx2.hash);
        assert_eq!(tx1.sector, tx2.sector);
        assert_eq!(tx1.device_type, tx2.device_type);
    }
}
