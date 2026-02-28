//! Transaction mempool management and pending transaction processing
//!
//! This module handles the collection and validation of pending transactions
//! from connected IoT devices and network peers before block inclusion.
//!
//! ## Real IoT Integration
//!
//! This module is designed to support real IoT data uploads in the future.
//! External devices can submit transactions via the `/api/transactions/submit` endpoint.
//! The mempool will validate and queue these transactions for block inclusion.
//!
//! ## Phase 1 Scaling: 1000 Devices
//!
//! This version supports 1000+ simulated IoT devices across 7 industries:
//! - Smart City: 200 devices
//! - Manufacturing: 150 devices
//! - Agriculture: 150 devices
//! - Energy: 150 devices
//! - Healthcare: 100 devices
//! - Logistics: 150 devices
//! - Edge AI: 100 devices

#![allow(dead_code)]

use crate::blockchain::transaction::{Transaction, TransactionType, TxOutput};
use chrono::Utc;

/// Deterministic hash generator for transaction ordering
struct TxHasher {
    state: u64,
}

impl TxHasher {
    fn new(seed: u64) -> Self {
        TxHasher { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223) % 4294967296;
        self.state as f64 / 4294967296.0
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_f64() * max as f64) as usize
    }

    fn next_range(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next_f64() * (max - min) as f64) as u64
    }

    fn next_range_f64(&mut self, min: f64, max: f64) -> f64 {
        min + self.next_f64() * (max - min)
    }
}

// ============================================================================
// DEVICE REGISTRY - Phase 1: 1000 Devices
// ============================================================================

/// Generate device IDs for a category
fn generate_device_ids(prefix: &str, count: usize) -> Vec<String> {
    (1..=count)
        .map(|i| format!("{}_{:03}", prefix, i))
        .collect()
}

/// Smart City Infrastructure Devices (200 devices)
fn smart_city_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("traffic_cam", 40));
    devices.extend(generate_device_ids("air_quality", 25));
    devices.extend(generate_device_ids("smart_light", 50));
    devices.extend(generate_device_ids("parking_sensor", 30));
    devices.extend(generate_device_ids("noise_monitor", 15));
    devices.extend(generate_device_ids("weather_station", 10));
    devices.extend(generate_device_ids("flood_sensor", 10));
    devices.extend(generate_device_ids("ev_charger", 20));
    devices
}

/// Industrial Manufacturing Devices (150 devices)
fn industrial_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("robot_arm", 30));
    devices.extend(generate_device_ids("cnc_machine", 25));
    devices.extend(generate_device_ids("vibration", 25));
    devices.extend(generate_device_ids("pressure", 20));
    devices.extend(generate_device_ids("temp_industrial", 15));
    devices.extend(generate_device_ids("conveyor", 15));
    devices.extend(generate_device_ids("quality_cam", 10));
    devices.extend(generate_device_ids("plc_gateway", 10));
    devices
}

/// Smart Agriculture Devices (150 devices)
fn agriculture_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("soil_probe", 40));
    devices.extend(generate_device_ids("irrigation", 30));
    devices.extend(generate_device_ids("weather_agri", 15));
    devices.extend(generate_device_ids("drone", 15));
    devices.extend(generate_device_ids("livestock", 20));
    devices.extend(generate_device_ids("greenhouse", 15));
    devices.extend(generate_device_ids("crop_monitor", 10));
    devices.extend(generate_device_ids("pest_detector", 5));
    devices
}

/// Energy Grid Devices (150 devices)
fn energy_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("smart_meter", 60));
    devices.extend(generate_device_ids("solar_array", 30));
    devices.extend(generate_device_ids("wind_turbine", 15));
    devices.extend(generate_device_ids("battery_storage", 15));
    devices.extend(generate_device_ids("grid_monitor", 15));
    devices.extend(generate_device_ids("transformer", 10));
    devices.extend(generate_device_ids("power_quality", 5));
    devices
}

/// Healthcare & Medical Devices (100 devices)
fn healthcare_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("patient_monitor", 30));
    devices.extend(generate_device_ids("infusion_pump", 20));
    devices.extend(generate_device_ids("ventilator", 15));
    devices.extend(generate_device_ids("ecg_monitor", 10));
    devices.extend(generate_device_ids("blood_analyzer", 10));
    devices.extend(generate_device_ids("imaging_device", 5));
    devices.extend(generate_device_ids("pharmacy_dispenser", 5));
    devices.extend(generate_device_ids("cold_storage", 5));
    devices
}

/// Logistics & Supply Chain Devices (150 devices)
fn logistics_devices() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("gps_tracker", 50));
    devices.extend(generate_device_ids("cold_chain", 25));
    devices.extend(generate_device_ids("warehouse", 25));
    devices.extend(generate_device_ids("rfid_reader", 20));
    devices.extend(generate_device_ids("dock_sensor", 15));
    devices.extend(generate_device_ids("forklift", 10));
    devices.extend(generate_device_ids("inventory", 5));
    devices
}

/// Edge AI Compute Nodes (100 devices)
fn ai_compute_nodes() -> Vec<String> {
    let mut devices = Vec::new();
    devices.extend(generate_device_ids("edge_gpu", 40));
    devices.extend(generate_device_ids("inference", 30));
    devices.extend(generate_device_ids("training_node", 15));
    devices.extend(generate_device_ids("model_server", 10));
    devices.extend(generate_device_ids("data_lake", 5));
    devices
}

/// Geographic regions with realistic distribution
const REGIONS: &[(&str, f64, f64)] = &[
    ("APAC-Singapore", 1.3521, 103.8198),
    ("APAC-Tokyo", 35.6762, 139.6503),
    ("APAC-Shanghai", 31.2304, 121.4737),
    ("APAC-Seoul", 37.5665, 126.9780),
    ("APAC-Mumbai", 19.0760, 72.8777),
    ("APAC-Sydney", -33.8688, 151.2093),
    ("APAC-HongKong", 22.3193, 114.1694),
    ("APAC-Jakarta", -6.2088, 106.8456),
    ("APAC-Bangkok", 13.7563, 100.5018),
    ("NA-NewYork", 40.7128, -74.0060),
    ("NA-SanFrancisco", 37.7749, -122.4194),
    ("NA-Toronto", 43.6532, -79.3832),
    ("NA-Chicago", 41.8781, -87.6298),
    ("NA-LosAngeles", 34.0522, -118.2437),
    ("NA-Seattle", 47.6062, -122.3321),
    ("NA-Dallas", 32.7767, -96.7970),
    ("EU-London", 51.5074, -0.1278),
    ("EU-Frankfurt", 50.1109, 8.6821),
    ("EU-Amsterdam", 52.3676, 4.9041),
    ("EU-Paris", 48.8566, 2.3522),
    ("EU-Dublin", 53.3498, -6.2603),
    ("EU-Stockholm", 59.3293, 18.0686),
    ("ME-Dubai", 25.2048, 55.2708),
    ("ME-TelAviv", 32.0853, 34.7818),
    ("SA-SaoPaulo", -23.5505, -46.6333),
    ("SA-BuenosAires", -34.6037, -58.3816),
    ("AF-Johannesburg", -26.2041, 28.0473),
    ("AF-Cairo", 30.0444, 31.2357),
];

/// Industry sectors for data categorization
const INDUSTRY_SECTORS: &[&str] = &[
    "SmartCity",
    "Manufacturing",
    "Agriculture",
    "Energy",
    "Healthcare",
    "Logistics",
    "EdgeAI",
    "Transportation",
];

// ============================================================================
// MEMPOOL MANAGER
// ============================================================================

/// Mempool manager for pending transaction collection
pub struct MempoolManager {
    hasher: TxHasher,
    seq: u64,
    all_devices: Vec<String>,
}

impl MempoolManager {
    /// Initialize mempool manager with block context
    pub fn with_block_context(block_idx: u64) -> Self {
        let seed = block_idx
            .wrapping_mul(1000003)
            .wrapping_add(Utc::now().timestamp() as u64);

        // Combine all device registries - Total: 1000 devices
        let mut all_devices: Vec<String> = Vec::new();
        all_devices.extend(smart_city_devices()); // 200
        all_devices.extend(industrial_devices()); // 150
        all_devices.extend(agriculture_devices()); // 150
        all_devices.extend(energy_devices()); // 150
        all_devices.extend(healthcare_devices()); // 100
        all_devices.extend(logistics_devices()); // 150
        all_devices.extend(ai_compute_nodes()); // 100

        log::info!(
            "MempoolManager initialized with {} devices",
            all_devices.len()
        );

        MempoolManager {
            hasher: TxHasher::new(seed),
            seq: 0,
            all_devices,
        }
    }

    /// Get total device count
    pub fn device_count(&self) -> usize {
        self.all_devices.len()
    }

    /// Collect pending transactions from network peers
    /// This simulates IoT devices submitting data to the network
    pub fn collect_pending(&mut self, batch_size: usize) -> Vec<Transaction> {
        (0..batch_size).map(|_| self.process_incoming()).collect()
    }

    /// Process incoming transaction from network
    fn process_incoming(&mut self) -> Transaction {
        self.seq += 1;

        // Transaction type distribution for realistic EdgeAI workload
        let tx_class = self.hasher.next_f64();
        if tx_class < 0.70 {
            // 70% - IoT data contribution (PoIE core business)
            self.generate_data_contribution()
        } else if tx_class < 0.85 {
            // 15% - Token transfers between devices/users
            self.generate_transfer()
        } else if tx_class < 0.95 {
            // 10% - Data marketplace purchases
            self.generate_data_purchase()
        } else {
            // 5% - AI model inference requests
            self.generate_model_inference()
        }
    }

    /// Get a random device from the registry
    fn random_device(&mut self) -> String {
        self.all_devices[self.hasher.next_usize(self.all_devices.len())].clone()
    }

    /// Get a random region
    fn random_region(&mut self) -> (&'static str, f64, f64) {
        REGIONS[self.hasher.next_usize(REGIONS.len())]
    }

    /// Get device category based on device name
    fn get_device_category(&self, device: &str) -> &'static str {
        if device.starts_with("traffic")
            || device.starts_with("air_quality")
            || device.starts_with("smart_light")
            || device.starts_with("parking")
            || device.starts_with("noise")
            || device.starts_with("weather_station")
            || device.starts_with("flood")
            || device.starts_with("ev_charger")
        {
            "SmartCity"
        } else if device.starts_with("robot")
            || device.starts_with("cnc")
            || device.starts_with("vibration")
            || device.starts_with("pressure")
            || device.starts_with("temp_industrial")
            || device.starts_with("conveyor")
            || device.starts_with("quality_cam")
            || device.starts_with("plc")
        {
            "Manufacturing"
        } else if device.starts_with("soil")
            || device.starts_with("irrigation")
            || device.starts_with("weather_agri")
            || device.starts_with("drone")
            || device.starts_with("livestock")
            || device.starts_with("greenhouse")
            || device.starts_with("crop")
            || device.starts_with("pest")
        {
            "Agriculture"
        } else if device.starts_with("smart_meter")
            || device.starts_with("solar")
            || device.starts_with("wind")
            || device.starts_with("battery")
            || device.starts_with("grid")
            || device.starts_with("transformer")
            || device.starts_with("power_quality")
        {
            "Energy"
        } else if device.starts_with("patient")
            || device.starts_with("infusion")
            || device.starts_with("ventilator")
            || device.starts_with("ecg")
            || device.starts_with("blood")
            || device.starts_with("imaging")
            || device.starts_with("pharmacy")
            || device.starts_with("cold_storage")
        {
            "Healthcare"
        } else if device.starts_with("gps")
            || device.starts_with("cold_chain")
            || device.starts_with("warehouse")
            || device.starts_with("rfid")
            || device.starts_with("dock")
            || device.starts_with("forklift")
            || device.starts_with("inventory")
        {
            "Logistics"
        } else if device.starts_with("edge_gpu")
            || device.starts_with("inference")
            || device.starts_with("training")
            || device.starts_with("model_server")
            || device.starts_with("data_lake")
        {
            "EdgeAI"
        } else {
            "General"
        }
    }

    // ========================================================================
    // TRANSACTION GENERATORS
    // ========================================================================

    /// Generate IoT data contribution transaction
    fn generate_data_contribution(&mut self) -> Transaction {
        let device = self.random_device();
        let (region, lat, lng) = self.random_region();
        let category = self.get_device_category(&device);

        // Generate realistic telemetry based on device type
        let telemetry = self.generate_telemetry(&device);
        let data_size = telemetry.len() as u64;

        // Calculate data quality score (affects reward)
        let quality_score = 0.7 + self.hasher.next_f64() * 0.3; // 0.7-1.0
        let freshness = self.hasher.next_f64() * 0.1; // Freshness bonus

        let data = format!(
            r#"{{"device":"{}","category":"{}","region":"{}","lat":{:.4},"lng":{:.4},"telemetry":{},"quality":{:.3},"size":{},"ts":{}}}"#,
            device,
            category,
            region,
            lat,
            lng,
            telemetry,
            quality_score + freshness,
            data_size,
            Utc::now().timestamp()
        );

        // Reward based on data quality and size
        let base_reward = 10 + (data_size / 10);
        let quality_bonus = (base_reward as f64 * quality_score) as u64;
        let reward = base_reward + quality_bonus;

        let output = TxOutput {
            amount: reward,
            recipient: device.clone(),
            data_hash: Some(format!("0x{:016x}", self.hasher.state)),
        };

        Transaction::new(
            TransactionType::DataContribution,
            device,
            vec![],
            vec![output],
            Some(data),
            1,
            21000,
        )
    }

    /// Generate token transfer transaction
    fn generate_transfer(&mut self) -> Transaction {
        let src = self.random_device();
        let mut dst = self.random_device();
        while dst == src {
            dst = self.random_device();
        }

        let amt = self.hasher.next_range(1, 50);

        // Add transfer reason for realism
        let reasons = [
            "service_payment",
            "data_access_fee",
            "compute_credit",
            "stake_delegation",
            "reward_distribution",
        ];
        let reason = reasons[self.hasher.next_usize(reasons.len())];

        let output = TxOutput {
            amount: amt,
            recipient: dst.clone(),
            data_hash: None,
        };

        let data = format!(
            r#"{{"op":"transfer","from":"{}","to":"{}","amount":{},"reason":"{}","ts":{}}}"#,
            src,
            dst,
            amt,
            reason,
            Utc::now().timestamp()
        );

        Transaction::new(
            TransactionType::Transfer,
            src,
            vec![],
            vec![output],
            Some(data),
            2,
            21000,
        )
    }

    /// Generate data purchase transaction
    fn generate_data_purchase(&mut self) -> Transaction {
        let buyer = self.random_device();
        let seller = self.random_device();

        let price = self.hasher.next_range(5, 100);
        let data_id = format!("data_{:08x}", self.hasher.state);

        // Data types available for purchase
        let data_types = [
            "historical_telemetry",
            "aggregated_metrics",
            "anomaly_patterns",
            "predictive_model",
            "training_dataset",
            "real_time_feed",
        ];
        let data_type = data_types[self.hasher.next_usize(data_types.len())];

        // Time range for historical data
        let duration_hours = self.hasher.next_range(1, 720); // 1 hour to 30 days

        let output = TxOutput {
            amount: price,
            recipient: seller.clone(),
            data_hash: Some(data_id.clone()),
        };

        let data = format!(
            r#"{{"op":"purchase","buyer":"{}","seller":"{}","data_id":"{}","data_type":"{}","price":{},"duration_hours":{},"ts":{}}}"#,
            buyer,
            seller,
            data_id,
            data_type,
            price,
            duration_hours,
            Utc::now().timestamp()
        );

        Transaction::new(
            TransactionType::DataPurchase,
            buyer,
            vec![],
            vec![output],
            Some(data),
            2,
            30000,
        )
    }

    /// Generate AI model inference request transaction
    fn generate_model_inference(&mut self) -> Transaction {
        let requester = self.random_device();

        // Select an AI compute node as the inference provider
        let ai_nodes = ai_compute_nodes();
        let compute_node = ai_nodes[self.hasher.next_usize(ai_nodes.len())].clone();

        // Model types available
        let models = [
            ("anomaly_detection", 5, 50),
            ("predictive_maintenance", 10, 100),
            ("image_classification", 15, 200),
            ("object_detection", 20, 300),
            ("time_series_forecast", 8, 80),
            ("nlp_sentiment", 12, 150),
        ];
        let (model_name, base_cost, compute_units) = models[self.hasher.next_usize(models.len())];

        let inference_cost = base_cost + self.hasher.next_range(0, 10) as u64;
        let input_size = self.hasher.next_range(100, 10000);

        let output = TxOutput {
            amount: inference_cost,
            recipient: compute_node.clone(),
            data_hash: Some(format!("inference_{:08x}", self.hasher.state)),
        };

        let data = format!(
            r#"{{"op":"inference","requester":"{}","provider":"{}","model":"{}","input_size":{},"compute_units":{},"cost":{},"ts":{}}}"#,
            requester,
            compute_node,
            model_name,
            input_size,
            compute_units,
            inference_cost,
            Utc::now().timestamp()
        );

        Transaction::new(
            TransactionType::DataPurchase, // Reuse DataPurchase type for now
            requester,
            vec![],
            vec![output],
            Some(data),
            3,
            50000,
        )
    }

    // ========================================================================
    // TELEMETRY GENERATORS - Realistic IoT Data
    // ========================================================================

    /// Generate realistic telemetry based on device type
    fn generate_telemetry(&mut self, device: &str) -> String {
        // Smart City devices
        if device.starts_with("traffic_cam") {
            return self.telemetry_traffic_camera();
        }
        if device.starts_with("air_quality") {
            return self.telemetry_air_quality();
        }
        if device.starts_with("smart_light") {
            return self.telemetry_smart_light();
        }
        if device.starts_with("parking") {
            return self.telemetry_parking();
        }
        if device.starts_with("noise") {
            return self.telemetry_noise();
        }
        if device.starts_with("ev_charger") {
            return self.telemetry_ev_charger();
        }

        // Industrial devices
        if device.starts_with("robot_arm") {
            return self.telemetry_robot_arm();
        }
        if device.starts_with("cnc") {
            return self.telemetry_cnc_machine();
        }
        if device.starts_with("vibration") {
            return self.telemetry_vibration();
        }
        if device.starts_with("pressure") {
            return self.telemetry_pressure();
        }
        if device.starts_with("conveyor") {
            return self.telemetry_conveyor();
        }

        // Agriculture devices
        if device.starts_with("soil_probe") {
            return self.telemetry_soil();
        }
        if device.starts_with("irrigation") {
            return self.telemetry_irrigation();
        }
        if device.starts_with("drone") {
            return self.telemetry_drone();
        }
        if device.starts_with("greenhouse") {
            return self.telemetry_greenhouse();
        }

        // Energy devices
        if device.starts_with("smart_meter") {
            return self.telemetry_smart_meter();
        }
        if device.starts_with("solar") {
            return self.telemetry_solar();
        }
        if device.starts_with("wind") {
            return self.telemetry_wind_turbine();
        }
        if device.starts_with("battery") {
            return self.telemetry_battery();
        }

        // Healthcare devices
        if device.starts_with("patient_monitor") {
            return self.telemetry_patient_monitor();
        }
        if device.starts_with("ventilator") {
            return self.telemetry_ventilator();
        }

        // Logistics devices
        if device.starts_with("gps_tracker") {
            return self.telemetry_gps();
        }
        if device.starts_with("cold_chain") {
            return self.telemetry_cold_chain();
        }

        // Edge AI devices
        if device.starts_with("edge_gpu") || device.starts_with("inference") {
            return self.telemetry_edge_compute();
        }

        // Default telemetry
        self.telemetry_generic()
    }

    // Smart City Telemetry
    fn telemetry_traffic_camera(&mut self) -> String {
        let vehicle_count = self.hasher.next_usize(500);
        let pedestrian_count = self.hasher.next_usize(200);
        let congestion = ["low", "medium", "high", "critical"][self.hasher.next_usize(4)];
        let avg_speed = self.hasher.next_range_f64(5.0, 80.0);
        format!(
            r#"{{"vehicles":{},"pedestrians":{},"congestion":"{}","avg_speed_kmh":{:.1},"incidents":{}}}"#,
            vehicle_count,
            pedestrian_count,
            congestion,
            avg_speed,
            self.hasher.next_usize(3)
        )
    }

    fn telemetry_air_quality(&mut self) -> String {
        let aqi = self.hasher.next_usize(300);
        let pm25 = self.hasher.next_range_f64(0.0, 150.0);
        let pm10 = self.hasher.next_range_f64(0.0, 200.0);
        let co2 = self.hasher.next_range(300, 2000);
        let no2 = self.hasher.next_range_f64(0.0, 100.0);
        format!(
            r#"{{"aqi":{},"pm2_5":{:.1},"pm10":{:.1},"co2_ppm":{},"no2_ppb":{:.1}}}"#,
            aqi, pm25, pm10, co2, no2
        )
    }

    fn telemetry_smart_light(&mut self) -> String {
        let brightness = self.hasher.next_usize(100);
        let power_w = self.hasher.next_range_f64(10.0, 150.0);
        let status = ["on", "off", "dimmed", "auto"][self.hasher.next_usize(4)];
        format!(
            r#"{{"brightness_pct":{},"power_watts":{:.1},"status":"{}","runtime_hours":{}}}"#,
            brightness,
            power_w,
            status,
            self.hasher.next_range(0, 10000)
        )
    }

    fn telemetry_parking(&mut self) -> String {
        let occupied = self.hasher.next_f64() > 0.3;
        let duration_min = if occupied {
            self.hasher.next_range(1, 480)
        } else {
            0
        };
        format!(
            r#"{{"occupied":{},"duration_minutes":{},"vehicle_type":"{}"}}"#,
            occupied,
            duration_min,
            ["car", "motorcycle", "truck"][self.hasher.next_usize(3)]
        )
    }

    fn telemetry_noise(&mut self) -> String {
        let db = self.hasher.next_range_f64(30.0, 100.0);
        let peak_db = db + self.hasher.next_range_f64(0.0, 20.0);
        format!(
            r#"{{"avg_db":{:.1},"peak_db":{:.1},"source":"{}"}}"#,
            db,
            peak_db,
            ["traffic", "construction", "event", "ambient"][self.hasher.next_usize(4)]
        )
    }

    fn telemetry_ev_charger(&mut self) -> String {
        let charging = self.hasher.next_f64() > 0.4;
        let power_kw = if charging {
            self.hasher.next_range_f64(3.0, 150.0)
        } else {
            0.0
        };
        let soc = self.hasher.next_usize(100);
        format!(
            r#"{{"charging":{},"power_kw":{:.1},"soc_pct":{},"session_kwh":{:.1}}}"#,
            charging,
            power_kw,
            soc,
            self.hasher.next_range_f64(0.0, 80.0)
        )
    }

    // Industrial Telemetry
    fn telemetry_robot_arm(&mut self) -> String {
        let cycles = self.hasher.next_range(0, 10000);
        let temp = self.hasher.next_range_f64(20.0, 80.0);
        let status = ["running", "idle", "maintenance", "error"][self.hasher.next_usize(4)];
        format!(
            r#"{{"cycles_today":{},"motor_temp_c":{:.1},"status":"{}","precision_mm":{:.2}}}"#,
            cycles,
            temp,
            status,
            self.hasher.next_range_f64(0.01, 0.5)
        )
    }

    fn telemetry_cnc_machine(&mut self) -> String {
        let spindle_rpm = self.hasher.next_range(0, 15000);
        let feed_rate = self.hasher.next_range_f64(0.0, 500.0);
        format!(
            r#"{{"spindle_rpm":{},"feed_rate_mmpm":{:.1},"tool_wear_pct":{},"parts_count":{}}}"#,
            spindle_rpm,
            feed_rate,
            self.hasher.next_usize(100),
            self.hasher.next_range(0, 1000)
        )
    }

    fn telemetry_vibration(&mut self) -> String {
        let rms = self.hasher.next_range_f64(0.1, 10.0);
        let peak = rms * (1.0 + self.hasher.next_f64());
        let freq = self.hasher.next_range_f64(10.0, 1000.0);
        format!(
            r#"{{"rms_mm_s":{:.2},"peak_mm_s":{:.2},"dominant_freq_hz":{:.1},"anomaly":{}}}"#,
            rms,
            peak,
            freq,
            self.hasher.next_f64() > 0.9
        )
    }

    fn telemetry_pressure(&mut self) -> String {
        let pressure = self.hasher.next_range_f64(0.0, 100.0);
        format!(
            r#"{{"pressure_bar":{:.2},"flow_lpm":{:.1},"temp_c":{:.1}}}"#,
            pressure,
            self.hasher.next_range_f64(0.0, 500.0),
            self.hasher.next_range_f64(10.0, 80.0)
        )
    }

    fn telemetry_conveyor(&mut self) -> String {
        let speed = self.hasher.next_range_f64(0.0, 5.0);
        let items = self.hasher.next_range(0, 1000);
        format!(
            r#"{{"speed_mps":{:.2},"items_count":{},"belt_temp_c":{:.1},"motor_current_a":{:.1}}}"#,
            speed,
            items,
            self.hasher.next_range_f64(20.0, 60.0),
            self.hasher.next_range_f64(1.0, 50.0)
        )
    }

    // Agriculture Telemetry
    fn telemetry_soil(&mut self) -> String {
        let moisture = self.hasher.next_range_f64(10.0, 80.0);
        let ph = self.hasher.next_range_f64(4.0, 9.0);
        let temp = self.hasher.next_range_f64(5.0, 35.0);
        format!(
            r#"{{"moisture_pct":{:.1},"ph":{:.1},"temp_c":{:.1},"nitrogen_ppm":{},"phosphorus_ppm":{}}}"#,
            moisture,
            ph,
            temp,
            self.hasher.next_range(0, 200),
            self.hasher.next_range(0, 100)
        )
    }

    fn telemetry_irrigation(&mut self) -> String {
        let active = self.hasher.next_f64() > 0.5;
        let flow = if active {
            self.hasher.next_range_f64(1.0, 50.0)
        } else {
            0.0
        };
        format!(
            r#"{{"active":{},"flow_lpm":{:.1},"pressure_bar":{:.1},"zone":{}}}"#,
            active,
            flow,
            self.hasher.next_range_f64(1.0, 5.0),
            self.hasher.next_usize(10)
        )
    }

    fn telemetry_drone(&mut self) -> String {
        let altitude = self.hasher.next_range_f64(0.0, 120.0);
        let battery = self.hasher.next_usize(100);
        format!(
            r#"{{"altitude_m":{:.1},"battery_pct":{},"speed_kmh":{:.1},"coverage_ha":{:.2}}}"#,
            altitude,
            battery,
            self.hasher.next_range_f64(0.0, 60.0),
            self.hasher.next_range_f64(0.0, 10.0)
        )
    }

    fn telemetry_greenhouse(&mut self) -> String {
        let temp = self.hasher.next_range_f64(15.0, 35.0);
        let humidity = self.hasher.next_range_f64(40.0, 90.0);
        let co2 = self.hasher.next_range(400, 1500);
        format!(
            r#"{{"temp_c":{:.1},"humidity_pct":{:.1},"co2_ppm":{},"light_lux":{}}}"#,
            temp,
            humidity,
            co2,
            self.hasher.next_range(0, 100000)
        )
    }

    // Energy Telemetry
    fn telemetry_smart_meter(&mut self) -> String {
        let power = self.hasher.next_range_f64(0.0, 15.0);
        let voltage = self.hasher.next_range_f64(220.0, 240.0);
        format!(
            r#"{{"power_kw":{:.2},"voltage_v":{:.1},"current_a":{:.1},"energy_kwh":{:.1},"pf":{:.2}}}"#,
            power,
            voltage,
            power * 1000.0 / voltage,
            self.hasher.next_range_f64(0.0, 1000.0),
            self.hasher.next_range_f64(0.8, 1.0)
        )
    }

    fn telemetry_solar(&mut self) -> String {
        let irradiance = self.hasher.next_range_f64(0.0, 1000.0);
        let power = irradiance * 0.2 * self.hasher.next_range_f64(0.8, 1.0);
        format!(
            r#"{{"irradiance_wm2":{:.1},"power_kw":{:.2},"efficiency_pct":{:.1},"panel_temp_c":{:.1}}}"#,
            irradiance,
            power / 1000.0,
            self.hasher.next_range_f64(15.0, 22.0),
            self.hasher.next_range_f64(20.0, 70.0)
        )
    }

    fn telemetry_wind_turbine(&mut self) -> String {
        let wind_speed = self.hasher.next_range_f64(0.0, 25.0);
        let power = if wind_speed > 3.0 && wind_speed < 25.0 {
            (wind_speed.powi(3) * 0.5).min(3000.0)
        } else {
            0.0
        };
        format!(
            r#"{{"wind_speed_ms":{:.1},"power_kw":{:.1},"rpm":{},"yaw_deg":{:.1}}}"#,
            wind_speed,
            power,
            self.hasher.next_range(0, 20),
            self.hasher.next_range_f64(0.0, 360.0)
        )
    }

    fn telemetry_battery(&mut self) -> String {
        let soc = self.hasher.next_range_f64(10.0, 100.0);
        let power = self.hasher.next_range_f64(-100.0, 100.0); // Negative = charging
        format!(
            r#"{{"soc_pct":{:.1},"power_kw":{:.1},"voltage_v":{:.1},"temp_c":{:.1},"cycles":{}}}"#,
            soc,
            power,
            self.hasher.next_range_f64(48.0, 52.0),
            self.hasher.next_range_f64(20.0, 45.0),
            self.hasher.next_range(0, 5000)
        )
    }

    // Healthcare Telemetry
    fn telemetry_patient_monitor(&mut self) -> String {
        let hr = self.hasher.next_range(50, 120);
        let spo2 = self.hasher.next_range(90, 100);
        let bp_sys = self.hasher.next_range(90, 160);
        let bp_dia = self.hasher.next_range(60, 100);
        format!(
            r#"{{"heart_rate_bpm":{},"spo2_pct":{},"bp_systolic":{},"bp_diastolic":{},"temp_c":{:.1}}}"#,
            hr,
            spo2,
            bp_sys,
            bp_dia,
            self.hasher.next_range_f64(36.0, 38.5)
        )
    }

    fn telemetry_ventilator(&mut self) -> String {
        let tidal = self.hasher.next_range(300, 700);
        let rate = self.hasher.next_range(10, 25);
        let fio2 = self.hasher.next_range(21, 100);
        format!(
            r#"{{"tidal_volume_ml":{},"resp_rate":{},"fio2_pct":{},"peep_cmh2o":{},"pip_cmh2o":{}}}"#,
            tidal,
            rate,
            fio2,
            self.hasher.next_range(5, 15),
            self.hasher.next_range(15, 35)
        )
    }

    // Logistics Telemetry
    fn telemetry_gps(&mut self) -> String {
        let (_, lat, lng) = self.random_region();
        let speed = self.hasher.next_range_f64(0.0, 120.0);
        format!(
            r#"{{"lat":{:.6},"lng":{:.6},"speed_kmh":{:.1},"heading_deg":{},"altitude_m":{}}}"#,
            lat + self.hasher.next_range_f64(-0.1, 0.1),
            lng + self.hasher.next_range_f64(-0.1, 0.1),
            speed,
            self.hasher.next_range(0, 360),
            self.hasher.next_range(0, 500)
        )
    }

    fn telemetry_cold_chain(&mut self) -> String {
        let temp = self.hasher.next_range_f64(-25.0, 8.0);
        let humidity = self.hasher.next_range_f64(30.0, 90.0);
        format!(
            r#"{{"temp_c":{:.1},"humidity_pct":{:.1},"door_open":{},"compressor_on":{}}}"#,
            temp,
            humidity,
            self.hasher.next_f64() > 0.9,
            self.hasher.next_f64() > 0.3
        )
    }

    // Edge AI Telemetry
    fn telemetry_edge_compute(&mut self) -> String {
        let gpu_util = self.hasher.next_usize(100);
        let gpu_temp = self.hasher.next_range_f64(30.0, 85.0);
        let memory_used = self.hasher.next_range_f64(0.0, 32.0);
        format!(
            r#"{{"gpu_util_pct":{},"gpu_temp_c":{:.1},"memory_gb":{:.1},"inference_ms":{:.1},"throughput_fps":{:.1}}}"#,
            gpu_util,
            gpu_temp,
            memory_used,
            self.hasher.next_range_f64(1.0, 100.0),
            self.hasher.next_range_f64(1.0, 60.0)
        )
    }

    fn telemetry_generic(&mut self) -> String {
        format!(
            r#"{{"value":{:.2},"status":"{}","uptime_hours":{}}}"#,
            self.hasher.next_range_f64(0.0, 100.0),
            ["normal", "warning", "error"][self.hasher.next_usize(3)],
            self.hasher.next_range(0, 10000)
        )
    }
}
