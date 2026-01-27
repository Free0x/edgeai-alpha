//! Enhanced IoT Device Management API
//!
//! Provides comprehensive IoT device registration, authentication,
//! data upload, and lifecycle management for EdgeAI blockchain.

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use log::{info, warn};
use sha2::{Sha256, Digest};

use super::rest::ApiResponse;

// ============ Constants ============

/// Maximum data payload size (1 MB)
const MAX_DATA_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Device heartbeat timeout (5 minutes)
const HEARTBEAT_TIMEOUT_SECS: u64 = 300;

/// Minimum contribution interval (10 seconds)
const MIN_CONTRIBUTION_INTERVAL_SECS: u64 = 10;

// ============ Data Types ============

/// IoT device types with EdgeAI-specific categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IoTDeviceType {
    // Environmental sensors
    TemperatureSensor,
    HumiditySensor,
    AirQualitySensor,
    WeatherStation,
    
    // Visual/Audio
    Camera,
    Microphone,
    LidarSensor,
    
    // Location/Movement
    GpsTracker,
    Accelerometer,
    MotionSensor,
    
    // Industrial
    IndustrialSensor,
    EnergyMeter,
    WaterMeter,
    
    // Smart Home
    SmartThermostat,
    SmartLock,
    SmartLight,
    
    // Wearables
    HealthMonitor,
    FitnessTracker,
    
    // Vehicles
    VehicleTelematics,
    DroneController,
    
    // Agriculture
    SoilSensor,
    CropMonitor,
    LivestockTracker,
    
    // Medical
    MedicalDevice,
    PatientMonitor,
    
    // AI Edge Devices
    EdgeAIProcessor,
    NeuralAccelerator,
    
    // Custom type
    Custom(String),
}

impl Default for IoTDeviceType {
    fn default() -> Self {
        IoTDeviceType::Custom("unknown".to_string())
    }
}

/// Device status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Pending,      // Registered but not verified
    Active,       // Verified and actively contributing
    Inactive,     // No recent activity
    Suspended,    // Temporarily suspended
    Banned,       // Permanently banned
}

impl Default for DeviceStatus {
    fn default() -> Self {
        DeviceStatus::Pending
    }
}

/// Geographic location
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoLocation {
    pub country_code: String,
    pub region_code: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCapabilities {
    pub data_types: Vec<String>,        // Types of data this device can provide
    pub sampling_rate_hz: Option<f64>,  // Data sampling rate
    pub accuracy: Option<f64>,          // Measurement accuracy
    pub resolution: Option<String>,     // For cameras/sensors
    pub connectivity: Vec<String>,      // wifi, cellular, lora, satellite, etc.
    pub edge_compute: bool,             // Can perform edge AI inference
    pub storage_mb: Option<u64>,        // Local storage capacity
    pub battery_powered: bool,          // Battery or mains powered
}

/// IoT Device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub device_id: String,
    pub owner_address: String,
    pub public_key: String,
    pub device_type: IoTDeviceType,
    pub status: DeviceStatus,
    pub location: GeoLocation,
    pub capabilities: DeviceCapabilities,
    pub metadata: HashMap<String, String>,
    
    // Statistics
    pub total_contributions: u64,
    pub total_data_bytes: u64,
    pub contribution_points: f64,
    pub reputation_score: f64,
    pub quality_score_avg: f64,
    
    // Timestamps
    pub registered_at: u64,
    pub last_seen_at: u64,
    pub last_contribution_at: u64,
    pub verified_at: Option<u64>,
    
    // Rewards
    pub pending_rewards: f64,
    pub total_rewards_earned: f64,
    pub total_rewards_claimed: f64,
}

impl IoTDevice {
    pub fn new(
        owner_address: String,
        public_key: String,
        device_type: IoTDeviceType,
        location: GeoLocation,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Generate device ID from public key hash
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hasher.update(now.to_le_bytes());
        let hash = hasher.finalize();
        let device_id = format!("iot_{}", hex::encode(&hash[..16]));
        
        Self {
            device_id,
            owner_address,
            public_key,
            device_type,
            status: DeviceStatus::Pending,
            location,
            capabilities: DeviceCapabilities::default(),
            metadata: HashMap::new(),
            total_contributions: 0,
            total_data_bytes: 0,
            contribution_points: 0.0,
            reputation_score: 50.0, // Start with neutral reputation
            quality_score_avg: 0.0,
            registered_at: now,
            last_seen_at: now,
            last_contribution_at: 0,
            verified_at: None,
            pending_rewards: 0.0,
            total_rewards_earned: 0.0,
            total_rewards_claimed: 0.0,
        }
    }

    /// Check if device is online (had recent heartbeat)
    pub fn is_online(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_seen_at < HEARTBEAT_TIMEOUT_SECS
    }

    /// Update last seen timestamp
    pub fn heartbeat(&mut self) {
        self.last_seen_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Record a data contribution
    pub fn record_contribution(&mut self, data_bytes: u64, quality_score: f64, points: f64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.total_contributions += 1;
        self.total_data_bytes += data_bytes;
        self.contribution_points += points;
        
        // Update average quality score (exponential moving average)
        let alpha = 0.1;
        if self.quality_score_avg == 0.0 {
            self.quality_score_avg = quality_score;
        } else {
            self.quality_score_avg = alpha * quality_score + (1.0 - alpha) * self.quality_score_avg;
        }
        
        // Update reputation based on quality
        self.update_reputation(quality_score);
        
        self.last_contribution_at = now;
        self.last_seen_at = now;
    }

    /// Update reputation score
    fn update_reputation(&mut self, quality_score: f64) {
        // Reputation changes based on quality relative to threshold
        let threshold = 0.7;
        let change = if quality_score >= threshold {
            (quality_score - threshold) * 2.0 // Positive change
        } else {
            (quality_score - threshold) * 5.0 // Negative change (penalize more)
        };
        
        self.reputation_score = (self.reputation_score + change).clamp(0.0, 100.0);
    }

    /// Calculate reward multiplier based on various factors
    pub fn reward_multiplier(&self) -> f64 {
        let mut multiplier = 1.0;
        
        // Reputation bonus (0.5x to 1.5x)
        multiplier *= 0.5 + (self.reputation_score / 100.0);
        
        // Quality bonus (0.8x to 1.2x)
        multiplier *= 0.8 + (self.quality_score_avg * 0.4);
        
        // Consistency bonus (more contributions = higher multiplier, up to 1.5x)
        let consistency = (self.total_contributions as f64 / 1000.0).min(1.0);
        multiplier *= 1.0 + (consistency * 0.5);
        
        // Edge compute bonus
        if self.capabilities.edge_compute {
            multiplier *= 1.2;
        }
        
        multiplier
    }

    /// Verify the device
    pub fn verify(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.status = DeviceStatus::Active;
        self.verified_at = Some(now);
    }

    /// Add pending rewards
    pub fn add_rewards(&mut self, amount: f64) {
        self.pending_rewards += amount;
        self.total_rewards_earned += amount;
    }

    /// Claim pending rewards
    pub fn claim_rewards(&mut self) -> f64 {
        let amount = self.pending_rewards;
        self.pending_rewards = 0.0;
        self.total_rewards_claimed += amount;
        amount
    }
}

/// Data contribution payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataContribution {
    pub contribution_id: String,
    pub device_id: String,
    pub data_type: String,
    pub data_hash: String,           // SHA256 hash of the data
    pub data_size_bytes: u64,
    pub timestamp: u64,
    pub location: Option<GeoLocation>,
    pub metadata: HashMap<String, String>,
    
    // Quality metrics
    pub quality_score: f64,          // 0.0 to 1.0
    pub freshness_score: f64,        // How recent the data is
    pub uniqueness_score: f64,       // How unique compared to existing data
    
    // Rewards
    pub base_reward: f64,
    pub bonus_reward: f64,
    pub total_reward: f64,
}

/// IoT Device Registry
pub struct IoTRegistry {
    pub devices: HashMap<String, IoTDevice>,
    pub contributions: Vec<DataContribution>,
    pub device_by_owner: HashMap<String, Vec<String>>,
    pub device_by_type: HashMap<IoTDeviceType, Vec<String>>,
    
    // Statistics
    pub total_devices: u64,
    pub active_devices: u64,
    pub total_contributions: u64,
    pub total_data_bytes: u64,
    pub total_rewards_distributed: f64,
}

impl IoTRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            contributions: Vec::new(),
            device_by_owner: HashMap::new(),
            device_by_type: HashMap::new(),
            total_devices: 0,
            active_devices: 0,
            total_contributions: 0,
            total_data_bytes: 0,
            total_rewards_distributed: 0.0,
        }
    }

    /// Register a new device
    pub fn register_device(&mut self, device: IoTDevice) -> Result<IoTDevice, String> {
        if self.devices.contains_key(&device.device_id) {
            return Err("Device already registered".to_string());
        }

        let device_id = device.device_id.clone();
        let owner = device.owner_address.clone();
        let device_type = device.device_type.clone();

        // Add to indices
        self.device_by_owner
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(device_id.clone());
        
        self.device_by_type
            .entry(device_type)
            .or_insert_with(Vec::new)
            .push(device_id.clone());

        self.devices.insert(device_id, device.clone());
        self.total_devices += 1;

        Ok(device)
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: &str) -> Option<&IoTDevice> {
        self.devices.get(device_id)
    }

    /// Get mutable device by ID
    pub fn get_device_mut(&mut self, device_id: &str) -> Option<&mut IoTDevice> {
        self.devices.get_mut(device_id)
    }

    /// Get devices by owner
    pub fn get_devices_by_owner(&self, owner: &str) -> Vec<&IoTDevice> {
        self.device_by_owner
            .get(owner)
            .map(|ids| ids.iter().filter_map(|id| self.devices.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get devices by type
    pub fn get_devices_by_type(&self, device_type: &IoTDeviceType) -> Vec<&IoTDevice> {
        self.device_by_type
            .get(device_type)
            .map(|ids| ids.iter().filter_map(|id| self.devices.get(id)).collect())
            .unwrap_or_default()
    }

    /// Record a data contribution
    pub fn record_contribution(&mut self, contribution: DataContribution) -> Result<DataContribution, String> {
        let device = self.devices.get_mut(&contribution.device_id)
            .ok_or("Device not found")?;

        // Check device status
        if device.status != DeviceStatus::Active {
            return Err(format!("Device is not active: {:?}", device.status));
        }

        // Check contribution interval
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now - device.last_contribution_at < MIN_CONTRIBUTION_INTERVAL_SECS {
            return Err("Contribution too frequent".to_string());
        }

        // Record contribution on device
        device.record_contribution(
            contribution.data_size_bytes,
            contribution.quality_score,
            contribution.total_reward,
        );

        // Add rewards to device
        device.add_rewards(contribution.total_reward);

        // Update registry stats
        self.total_contributions += 1;
        self.total_data_bytes += contribution.data_size_bytes;
        self.total_rewards_distributed += contribution.total_reward;

        // Store contribution record
        self.contributions.push(contribution.clone());

        // Keep only last 10000 contributions in memory
        if self.contributions.len() > 10000 {
            self.contributions.remove(0);
        }

        Ok(contribution)
    }

    /// Update active device count
    pub fn update_active_count(&mut self) {
        self.active_devices = self.devices.values()
            .filter(|d| d.is_online() && d.status == DeviceStatus::Active)
            .count() as u64;
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> IoTRegistryStats {
        let online_count = self.devices.values().filter(|d| d.is_online()).count() as u64;
        let verified_count = self.devices.values().filter(|d| d.verified_at.is_some()).count() as u64;
        
        let type_distribution: HashMap<String, u64> = self.device_by_type
            .iter()
            .map(|(k, v)| (format!("{:?}", k), v.len() as u64))
            .collect();

        IoTRegistryStats {
            total_devices: self.total_devices,
            active_devices: self.active_devices,
            online_devices: online_count,
            verified_devices: verified_count,
            total_contributions: self.total_contributions,
            total_data_bytes: self.total_data_bytes,
            total_rewards_distributed: self.total_rewards_distributed,
            device_type_distribution: type_distribution,
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTRegistryStats {
    pub total_devices: u64,
    pub active_devices: u64,
    pub online_devices: u64,
    pub verified_devices: u64,
    pub total_contributions: u64,
    pub total_data_bytes: u64,
    pub total_rewards_distributed: f64,
    pub device_type_distribution: HashMap<String, u64>,
}

// ============ API State ============

pub struct IoTState {
    pub registry: Arc<RwLock<IoTRegistry>>,
}

// ============ Request/Response Types ============

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub owner_address: String,
    pub public_key: String,
    pub device_type: String,
    pub location: GeoLocation,
    pub capabilities: Option<DeviceCapabilities>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitDataRequest {
    pub device_id: String,
    pub signature: String,           // Signature of data hash using device private key
    pub data_type: String,
    pub data_hash: String,
    pub data_size_bytes: u64,
    pub location: Option<GeoLocation>,
    pub metadata: Option<HashMap<String, String>>,
}

// ============ External/Simplified API Types ============

/// Telemetry data from external devices (simplified format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTelemetry {
    pub accuracy_m: f64,
    pub altitude_m: f64,
    pub heading_deg: f64,
    pub source: String,
    pub speed_accuracy_mps: f64,
    pub speed_mps: f64,
    pub ts: i64,
}

/// Simplified external data submission request
/// This format is designed for easy integration by external developers
#[derive(Debug, Deserialize)]
pub struct ExternalSubmitRequest {
    pub device: String,
    pub category: String,
    pub telemetry: ExternalTelemetry,
    pub lat: f64,
    pub lng: f64,
    pub ts: i64,
    pub source: String,
}

/// Response for external data submission
#[derive(Debug, Serialize)]
pub struct ExternalSubmitResponse {
    pub contribution_id: String,
    pub device: String,
    pub category: String,
    pub quality_score: f64,
    pub reward_points: f64,
    pub timestamp: u64,
    pub data_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub device_id: String,
    pub signature: String,
    pub status: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimRewardsRequest {
    pub device_id: String,
    pub signature: String,
    pub destination_address: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub device_id: String,
    pub owner_address: String,
    pub device_type: String,
    pub status: String,
    pub location: GeoLocation,
    pub capabilities: DeviceCapabilities,
    pub is_online: bool,
    pub total_contributions: u64,
    pub total_data_bytes: u64,
    pub contribution_points: f64,
    pub reputation_score: f64,
    pub quality_score_avg: f64,
    pub pending_rewards: f64,
    pub total_rewards_earned: f64,
    pub reward_multiplier: f64,
    pub registered_at: u64,
    pub last_seen_at: u64,
    pub verified_at: Option<u64>,
}

impl From<&IoTDevice> for DeviceResponse {
    fn from(device: &IoTDevice) -> Self {
        Self {
            device_id: device.device_id.clone(),
            owner_address: device.owner_address.clone(),
            device_type: format!("{:?}", device.device_type),
            status: format!("{:?}", device.status),
            location: device.location.clone(),
            capabilities: device.capabilities.clone(),
            is_online: device.is_online(),
            total_contributions: device.total_contributions,
            total_data_bytes: device.total_data_bytes,
            contribution_points: device.contribution_points,
            reputation_score: device.reputation_score,
            quality_score_avg: device.quality_score_avg,
            pending_rewards: device.pending_rewards,
            total_rewards_earned: device.total_rewards_earned,
            reward_multiplier: device.reward_multiplier(),
            registered_at: device.registered_at,
            last_seen_at: device.last_seen_at,
            verified_at: device.verified_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContributionResponse {
    pub contribution_id: String,
    pub device_id: String,
    pub data_type: String,
    pub data_hash: String,
    pub data_size_bytes: u64,
    pub quality_score: f64,
    pub total_reward: f64,
    pub timestamp: u64,
}

impl From<&DataContribution> for ContributionResponse {
    fn from(c: &DataContribution) -> Self {
        Self {
            contribution_id: c.contribution_id.clone(),
            device_id: c.device_id.clone(),
            data_type: c.data_type.clone(),
            data_hash: c.data_hash.clone(),
            data_size_bytes: c.data_size_bytes,
            quality_score: c.quality_score,
            total_reward: c.total_reward,
            timestamp: c.timestamp,
        }
    }
}

// ============ Helper Functions ============

fn parse_device_type(type_str: &str) -> IoTDeviceType {
    match type_str.to_lowercase().as_str() {
        "temperature_sensor" | "temperature" => IoTDeviceType::TemperatureSensor,
        "humidity_sensor" | "humidity" => IoTDeviceType::HumiditySensor,
        "air_quality_sensor" | "air_quality" => IoTDeviceType::AirQualitySensor,
        "weather_station" | "weather" => IoTDeviceType::WeatherStation,
        "camera" => IoTDeviceType::Camera,
        "microphone" | "audio" => IoTDeviceType::Microphone,
        "lidar" | "lidar_sensor" => IoTDeviceType::LidarSensor,
        "gps" | "gps_tracker" => IoTDeviceType::GpsTracker,
        "accelerometer" => IoTDeviceType::Accelerometer,
        "motion" | "motion_sensor" => IoTDeviceType::MotionSensor,
        "industrial" | "industrial_sensor" => IoTDeviceType::IndustrialSensor,
        "energy" | "energy_meter" => IoTDeviceType::EnergyMeter,
        "water" | "water_meter" => IoTDeviceType::WaterMeter,
        "thermostat" | "smart_thermostat" => IoTDeviceType::SmartThermostat,
        "lock" | "smart_lock" => IoTDeviceType::SmartLock,
        "light" | "smart_light" => IoTDeviceType::SmartLight,
        "health" | "health_monitor" => IoTDeviceType::HealthMonitor,
        "fitness" | "fitness_tracker" => IoTDeviceType::FitnessTracker,
        "vehicle" | "telematics" => IoTDeviceType::VehicleTelematics,
        "drone" | "drone_controller" => IoTDeviceType::DroneController,
        "soil" | "soil_sensor" => IoTDeviceType::SoilSensor,
        "crop" | "crop_monitor" => IoTDeviceType::CropMonitor,
        "livestock" | "livestock_tracker" => IoTDeviceType::LivestockTracker,
        "medical" | "medical_device" => IoTDeviceType::MedicalDevice,
        "patient" | "patient_monitor" => IoTDeviceType::PatientMonitor,
        "edge_ai" | "edge_processor" => IoTDeviceType::EdgeAIProcessor,
        "neural" | "neural_accelerator" => IoTDeviceType::NeuralAccelerator,
        _ => IoTDeviceType::Custom(type_str.to_string()),
    }
}

fn calculate_quality_score(data_size: u64, data_type: &str) -> f64 {
    // Base quality score
    let mut score = 0.5;
    
    // Size bonus (larger data generally more valuable, up to a point)
    let size_factor = (data_size as f64 / 10000.0).min(1.0);
    score += size_factor * 0.2;
    
    // Type bonus (some data types are more valuable)
    let type_bonus = match data_type {
        "video" | "image" => 0.2,
        "audio" => 0.15,
        "sensor_array" => 0.15,
        "location_trace" => 0.1,
        _ => 0.05,
    };
    score += type_bonus;
    
    // Add some randomness for demo
    score += (rand::random::<f64>() - 0.5) * 0.1;
    
    score.clamp(0.0, 1.0)
}

fn calculate_base_reward(data_size: u64, quality_score: f64) -> f64 {
    // Base reward: 0.001 EDGE per KB
    let size_reward = (data_size as f64 / 1024.0) * 0.001;
    
    // Quality multiplier: 0.5x to 2x based on quality
    let quality_multiplier = 0.5 + (quality_score * 1.5);
    
    size_reward * quality_multiplier
}

// ============ API Endpoints ============

/// Register a new IoT device
pub async fn register_iot_device(
    data: web::Data<IoTState>,
    body: web::Json<RegisterDeviceRequest>,
) -> impl Responder {
    let device_type = parse_device_type(&body.device_type);
    
    let mut device = IoTDevice::new(
        body.owner_address.clone(),
        body.public_key.clone(),
        device_type,
        body.location.clone(),
    );
    
    if let Some(caps) = &body.capabilities {
        device.capabilities = caps.clone();
    }
    
    if let Some(meta) = &body.metadata {
        device.metadata = meta.clone();
    }
    
    let mut registry = data.registry.write().await;
    
    match registry.register_device(device) {
        Ok(device) => {
            info!("IoT device registered: {} ({:?}) by {}", 
                &device.device_id, device.device_type, &device.owner_address);
            
            let response = DeviceResponse::from(&device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Get device by ID
pub async fn get_iot_device(
    data: web::Data<IoTState>,
    path: web::Path<String>,
) -> impl Responder {
    let device_id = path.into_inner();
    let registry = data.registry.read().await;
    
    match registry.get_device(&device_id) {
        Some(device) => {
            let response = DeviceResponse::from(device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Device not found")),
    }
}

/// Get devices by owner
pub async fn get_devices_by_owner(
    data: web::Data<IoTState>,
    path: web::Path<String>,
) -> impl Responder {
    let owner = path.into_inner();
    let registry = data.registry.read().await;
    
    let devices: Vec<DeviceResponse> = registry.get_devices_by_owner(&owner)
        .into_iter()
        .map(DeviceResponse::from)
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(devices))
}

/// Submit data contribution
pub async fn submit_data(
    data: web::Data<IoTState>,
    body: web::Json<SubmitDataRequest>,
) -> impl Responder {
    // TODO: Verify signature in production
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Generate contribution ID
    let mut hasher = Sha256::new();
    hasher.update(body.device_id.as_bytes());
    hasher.update(body.data_hash.as_bytes());
    hasher.update(now.to_le_bytes());
    let hash = hasher.finalize();
    let contribution_id = format!("contrib_{}", hex::encode(&hash[..16]));
    
    // Calculate scores and rewards
    let quality_score = calculate_quality_score(body.data_size_bytes, &body.data_type);
    let base_reward = calculate_base_reward(body.data_size_bytes, quality_score);
    
    // Get device multiplier
    let registry = data.registry.read().await;
    let device_multiplier = registry.get_device(&body.device_id)
        .map(|d| d.reward_multiplier())
        .unwrap_or(1.0);
    drop(registry);
    
    let bonus_reward = base_reward * (device_multiplier - 1.0);
    let total_reward = base_reward + bonus_reward;
    
    let contribution = DataContribution {
        contribution_id: contribution_id.clone(),
        device_id: body.device_id.clone(),
        data_type: body.data_type.clone(),
        data_hash: body.data_hash.clone(),
        data_size_bytes: body.data_size_bytes,
        timestamp: now,
        location: body.location.clone(),
        metadata: body.metadata.clone().unwrap_or_default(),
        quality_score,
        freshness_score: 1.0, // Fresh data
        uniqueness_score: 0.8, // Assume mostly unique
        base_reward,
        bonus_reward,
        total_reward,
    };
    
    let mut registry = data.registry.write().await;
    
    match registry.record_contribution(contribution) {
        Ok(contribution) => {
            info!("Data contribution recorded: {} from {} ({} bytes, {:.4} EDGE)", 
                &contribution.contribution_id, &contribution.device_id, 
                contribution.data_size_bytes, contribution.total_reward);
            
            let response = ContributionResponse::from(&contribution);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Device heartbeat
pub async fn device_heartbeat(
    data: web::Data<IoTState>,
    body: web::Json<HeartbeatRequest>,
) -> impl Responder {
    let mut registry = data.registry.write().await;
    
    match registry.get_device_mut(&body.device_id) {
        Some(device) => {
            device.heartbeat();
            
            let response = DeviceResponse::from(&*device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Device not found")),
    }
}

/// Verify a device (admin only)
pub async fn verify_device(
    data: web::Data<IoTState>,
    path: web::Path<String>,
) -> impl Responder {
    let device_id = path.into_inner();
    let mut registry = data.registry.write().await;
    
    match registry.get_device_mut(&device_id) {
        Some(device) => {
            device.verify();
            info!("Device verified: {}", &device_id);
            
            let response = DeviceResponse::from(&*device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Device not found")),
    }
}

/// Claim rewards
pub async fn claim_rewards(
    data: web::Data<IoTState>,
    body: web::Json<ClaimRewardsRequest>,
) -> impl Responder {
    // TODO: Verify signature and process actual transfer
    
    let mut registry = data.registry.write().await;
    
    match registry.get_device_mut(&body.device_id) {
        Some(device) => {
            let amount = device.claim_rewards();
            
            if amount > 0.0 {
                info!("Rewards claimed: {} EDGE from {} to {}", 
                    amount, &body.device_id, &body.destination_address);
                
                #[derive(Serialize)]
                struct ClaimResponse {
                    device_id: String,
                    amount_claimed: f64,
                    destination: String,
                    tx_hash: String,
                }
                
                // Generate mock tx hash
                let tx_hash = format!("0x{}", hex::encode(&[0u8; 32][..16]));
                
                HttpResponse::Ok().json(ApiResponse::success(ClaimResponse {
                    device_id: body.device_id.clone(),
                    amount_claimed: amount,
                    destination: body.destination_address.clone(),
                    tx_hash,
                }))
            } else {
                HttpResponse::BadRequest().json(ApiResponse::<()>::error("No rewards to claim"))
            }
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Device not found")),
    }
}

/// Get IoT registry statistics
pub async fn get_iot_stats(
    data: web::Data<IoTState>,
) -> impl Responder {
    let registry = data.registry.read().await;
    let stats = registry.get_stats();
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Get recent contributions
pub async fn get_recent_contributions(
    data: web::Data<IoTState>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let limit = query.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50usize)
        .min(100);
    
    let registry = data.registry.read().await;
    
    let contributions: Vec<ContributionResponse> = registry.contributions
        .iter()
        .rev()
        .take(limit)
        .map(ContributionResponse::from)
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(contributions))
}

/// Get device leaderboard
pub async fn get_device_leaderboard(
    data: web::Data<IoTState>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let limit = query.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20usize)
        .min(100);
    
    let sort_by = query.get("sort").map(|s| s.as_str()).unwrap_or("rewards");
    
    let registry = data.registry.read().await;
    
    let mut devices: Vec<&IoTDevice> = registry.devices.values().collect();
    
    match sort_by {
        "contributions" => devices.sort_by(|a, b| b.total_contributions.cmp(&a.total_contributions)),
        "reputation" => devices.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap()),
        "data" => devices.sort_by(|a, b| b.total_data_bytes.cmp(&a.total_data_bytes)),
        _ => devices.sort_by(|a, b| b.total_rewards_earned.partial_cmp(&a.total_rewards_earned).unwrap()),
    }
    
    let leaderboard: Vec<DeviceResponse> = devices
        .into_iter()
        .take(limit)
        .map(DeviceResponse::from)
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(leaderboard))
}

// ============ External/Simplified API Endpoint ============

/// POST /api/external/submit - Simplified data submission for external developers
/// 
/// This endpoint provides a simplified interface for external applications to submit
/// IoT data without requiring device registration or cryptographic signatures.
/// 
/// Features:
/// - No device registration required
/// - No signature verification
/// - Automatic device tracking
/// - Quality scoring and reward calculation
pub async fn external_submit(
    data: web::Data<IoTState>,
    body: web::Json<ExternalSubmitRequest>,
) -> impl Responder {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Validate source field
    if body.source != "external" {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Invalid source: must be 'external' for this endpoint"
        ));
    }
    
    // Validate coordinates
    if body.lat < -90.0 || body.lat > 90.0 || body.lng < -180.0 || body.lng > 180.0 {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Invalid coordinates: lat must be -90 to 90, lng must be -180 to 180"
        ));
    }
    
    // Serialize the request data for hashing
    let data_json = serde_json::json!({
        "device": body.device,
        "category": body.category,
        "telemetry": body.telemetry,
        "lat": body.lat,
        "lng": body.lng,
        "ts": body.ts,
        "source": body.source,
    });
    let data_str = serde_json::to_string(&data_json).unwrap_or_default();
    let data_size = data_str.len() as u64;
    
    // Generate data hash
    let mut hasher = Sha256::new();
    hasher.update(data_str.as_bytes());
    let hash = hasher.finalize();
    let data_hash = format!("ext_{:08x}", u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]));
    
    // Generate contribution ID
    let mut id_hasher = Sha256::new();
    id_hasher.update(body.device.as_bytes());
    id_hasher.update(data_hash.as_bytes());
    id_hasher.update(now.to_le_bytes());
    let id_hash = id_hasher.finalize();
    let contribution_id = format!("ext_contrib_{}", hex::encode(&id_hash[..12]));
    
    // Calculate quality score based on telemetry data
    let quality_score = calculate_external_quality_score(&body);
    
    // Calculate reward points (simplified calculation)
    let base_points = 10.0;
    let quality_multiplier = 0.5 + (quality_score * 1.5);
    let reward_points = base_points * quality_multiplier;
    
    // Create location from request
    let location = GeoLocation {
        country_code: "XX".to_string(), // Unknown, could be determined by reverse geocoding
        region_code: None,
        city: None,
        latitude: Some(body.lat),
        longitude: Some(body.lng),
        altitude: Some(body.telemetry.altitude_m),
    };
    
    // Create contribution record
    let contribution = DataContribution {
        contribution_id: contribution_id.clone(),
        device_id: body.device.clone(),
        data_type: body.category.clone(),
        data_hash: data_hash.clone(),
        data_size_bytes: data_size,
        timestamp: now,
        location: Some(location),
        metadata: HashMap::new(),
        quality_score,
        freshness_score: 1.0,
        uniqueness_score: 0.8,
        base_reward: reward_points,
        bonus_reward: 0.0,
        total_reward: reward_points,
    };
    
    // Record the contribution
    let mut registry = data.registry.write().await;
    
    // Check if device exists, if not create a temporary record
    if registry.get_device(&body.device).is_none() {
        // Auto-register the external device
        let mut device = IoTDevice::new(
            body.device.clone(), // Use device ID as owner for external devices
            format!("external_key_{}", &body.device),
            IoTDeviceType::Custom(body.category.clone()),
            GeoLocation {
                country_code: "XX".to_string(),
                region_code: None,
                city: None,
                latitude: Some(body.lat),
                longitude: Some(body.lng),
                altitude: Some(body.telemetry.altitude_m),
            },
        );
        // Set device as active for external devices (skip verification)
        device.status = DeviceStatus::Active;
        device.last_heartbeat = now;
        let _ = registry.register_device(device);
        info!("Auto-registered external device: {}", &body.device);
    } else {
        // Update heartbeat for existing device
        if let Some(device) = registry.get_device_mut(&body.device) {
            device.heartbeat();
            // Ensure device is active
            if device.status != DeviceStatus::Active {
                device.status = DeviceStatus::Active;
            }
        }
    }
    
    match registry.record_contribution(contribution) {
        Ok(contrib) => {
            info!("External contribution recorded: {} from {} (category: {}, quality: {:.2}, points: {:.2})",
                &contrib.contribution_id, &body.device, &body.category, quality_score, reward_points);
            
            let response = ExternalSubmitResponse {
                contribution_id: contrib.contribution_id,
                device: body.device.clone(),
                category: body.category.clone(),
                quality_score,
                reward_points,
                timestamp: now,
                data_hash,
            };
            
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            warn!("External contribution failed: {} - {}", &body.device, e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e))
        }
    }
}

/// Calculate quality score for external submissions
fn calculate_external_quality_score(req: &ExternalSubmitRequest) -> f64 {
    let mut score: f64 = 0.5; // Base score
    
    // GPS accuracy bonus (better accuracy = higher score)
    // accuracy_m < 10 is excellent, < 50 is good, < 100 is acceptable
    let accuracy_bonus = if req.telemetry.accuracy_m < 10.0 {
        0.25
    } else if req.telemetry.accuracy_m < 50.0 {
        0.15
    } else if req.telemetry.accuracy_m < 100.0 {
        0.08
    } else {
        0.0
    };
    score += accuracy_bonus;
    
    // Category bonus (some categories are more valuable)
    let category_bonus = match req.category.as_str() {
        "SmartCity" => 0.15,
        "DePIN" => 0.15,
        "Healthcare" => 0.20,
        "Agriculture" => 0.12,
        "Transportation" => 0.12,
        "Environment" => 0.10,
        _ => 0.05,
    };
    score += category_bonus;
    
    // Freshness bonus (data submitted close to collection time)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let age_secs = (now - req.ts).abs();
    let freshness_bonus = if age_secs < 60 {
        0.10 // Less than 1 minute old
    } else if age_secs < 300 {
        0.05 // Less than 5 minutes old
    } else {
        0.0
    };
    score += freshness_bonus;
    
    score.clamp(0.0, 1.0)
}

// ============ Router Configuration ============

pub fn configure_iot_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Device management
        .route("/api/iot/devices", web::get().to(get_iot_stats))
        .route("/api/iot/devices/register", web::post().to(register_iot_device))
        .route("/api/iot/devices/{device_id}", web::get().to(get_iot_device))
        .route("/api/iot/devices/{device_id}/verify", web::post().to(verify_device))
        .route("/api/iot/devices/owner/{owner}", web::get().to(get_devices_by_owner))
        .route("/api/iot/devices/leaderboard", web::get().to(get_device_leaderboard))
        
        // Data contributions
        .route("/api/iot/data/submit", web::post().to(submit_data))
        .route("/api/iot/data/recent", web::get().to(get_recent_contributions))
        
        // Device operations
        .route("/api/iot/heartbeat", web::post().to(device_heartbeat))
        .route("/api/iot/rewards/claim", web::post().to(claim_rewards))
        
        // External/Simplified API for third-party developers
        .route("/api/external/submit", web::post().to(external_submit));
}
