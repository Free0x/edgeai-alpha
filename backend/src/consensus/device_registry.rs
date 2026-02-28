//! Device Registry module for EdgeAI Blockchain
//!
//! This module manages IoT device registration, reputation tracking,
//! and contribution scoring for the PoIE 2.0 consensus mechanism.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Device type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// IoT sensors (temperature, humidity, etc.)
    Sensor,
    /// Cameras and image capture devices
    Camera,
    /// Audio recording devices
    Audio,
    /// GPS and location trackers
    Location,
    /// Industrial equipment
    Industrial,
    /// Smart home devices
    SmartHome,
    /// Wearable devices
    Wearable,
    /// Vehicles and mobility devices
    Vehicle,
    /// Environmental monitoring
    Environmental,
    /// Medical devices
    Medical,
    /// Agricultural sensors
    Agricultural,
    /// Energy monitoring
    Energy,
    /// Custom/Other
    Custom(String),
}

impl DeviceType {
    /// Get the base contribution multiplier for this device type
    pub fn base_multiplier(&self) -> f64 {
        match self {
            DeviceType::Medical => 2.0,       // High value, regulated data
            DeviceType::Industrial => 1.8,    // High value industrial data
            DeviceType::Environmental => 1.5, // Important for climate
            DeviceType::Vehicle => 1.4,       // Mobility data
            DeviceType::Agricultural => 1.3,  // Food security data
            DeviceType::Energy => 1.3,        // Energy grid data
            DeviceType::Camera => 1.2,        // Visual data
            DeviceType::Audio => 1.1,         // Audio data
            DeviceType::Sensor => 1.0,        // Base sensor data
            DeviceType::Location => 1.0,      // Location data
            DeviceType::SmartHome => 0.9,     // Consumer data
            DeviceType::Wearable => 0.9,      // Personal data
            DeviceType::Custom(_) => 1.0,     // Default
        }
    }
}

/// Geographic region for data scarcity calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GeoRegion {
    /// ISO 3166-1 alpha-2 country code
    pub country_code: String,
    /// Optional region/state code
    pub region_code: Option<String>,
    /// Latitude (approximate)
    pub latitude: Option<i32>,
    /// Longitude (approximate)
    pub longitude: Option<i32>,
}

impl GeoRegion {
    pub fn new(country_code: &str) -> Self {
        GeoRegion {
            country_code: country_code.to_uppercase(),
            region_code: None,
            latitude: None,
            longitude: None,
        }
    }

    pub fn with_coordinates(country_code: &str, lat: i32, lon: i32) -> Self {
        GeoRegion {
            country_code: country_code.to_uppercase(),
            region_code: None,
            latitude: Some(lat),
            longitude: Some(lon),
        }
    }

    /// Get region key for scarcity calculation
    pub fn region_key(&self) -> String {
        match &self.region_code {
            Some(region) => format!("{}:{}", self.country_code, region),
            None => self.country_code.clone(),
        }
    }
}

/// Registered device in the EdgeAI network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device ID (derived from public key)
    pub device_id: String,
    /// Device public key for verification
    pub public_key: String,
    /// Device type
    pub device_type: DeviceType,
    /// Geographic region
    pub region: GeoRegion,
    /// Device metadata (manufacturer, model, etc.)
    pub metadata: HashMap<String, String>,
    /// Registration timestamp
    pub registered_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
    /// Is device currently active
    pub is_active: bool,
    /// Device reputation score (0-100)
    pub reputation: f64,
    /// Total data contributions
    pub total_contributions: u64,
    /// Total contribution points earned
    pub contribution_points: f64,
    /// Data quality history (last N contributions)
    pub quality_history: Vec<f64>,
    /// Is device verified (KYC or hardware attestation)
    pub is_verified: bool,
}

impl Device {
    pub fn new(public_key: String, device_type: DeviceType, region: GeoRegion) -> Self {
        // Generate device ID from public key
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let hash = hasher.finalize();
        let device_id = format!("DEV_{}", hex::encode(&hash[..8]));

        let now = Utc::now();

        Device {
            device_id,
            public_key,
            device_type,
            region,
            metadata: HashMap::new(),
            registered_at: now,
            last_active: now,
            is_active: true,
            reputation: 50.0, // Start with neutral reputation
            total_contributions: 0,
            contribution_points: 0.0,
            quality_history: Vec::new(),
            is_verified: false,
        }
    }

    /// Update device activity
    pub fn record_activity(&mut self) {
        self.last_active = Utc::now();
        self.is_active = true;
    }

    /// Record a data contribution
    pub fn record_contribution(&mut self, quality_score: f64, points: f64) {
        self.total_contributions += 1;
        self.contribution_points += points;
        self.last_active = Utc::now();

        // Update quality history (keep last 100)
        self.quality_history.push(quality_score);
        if self.quality_history.len() > 100 {
            self.quality_history.remove(0);
        }

        // Update reputation based on quality
        self.update_reputation(quality_score);
    }

    /// Update reputation based on contribution quality
    fn update_reputation(&mut self, quality_score: f64) {
        // Weighted moving average
        let weight = 0.1;
        let target_reputation = quality_score * 100.0;
        self.reputation = self.reputation * (1.0 - weight) + target_reputation * weight;

        // Clamp to 0-100
        self.reputation = self.reputation.clamp(0.0, 100.0);
    }

    /// Get average quality score
    pub fn average_quality(&self) -> f64 {
        if self.quality_history.is_empty() {
            return 0.5; // Default neutral
        }
        self.quality_history.iter().sum::<f64>() / self.quality_history.len() as f64
    }

    /// Calculate device weight for validator selection
    pub fn validator_weight(&self) -> f64 {
        if !self.is_active || self.reputation < 20.0 {
            return 0.0;
        }

        let base = self.device_type.base_multiplier();
        let reputation_factor = self.reputation / 100.0;
        let contribution_factor = (self.contribution_points / 1000.0).min(2.0);
        let verified_bonus = if self.is_verified { 1.5 } else { 1.0 };

        base * reputation_factor * (1.0 + contribution_factor) * verified_bonus
    }
}

/// Device Registry - manages all registered devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    /// All registered devices
    pub devices: HashMap<String, Device>,
    /// Device count by type
    pub type_counts: HashMap<String, u64>,
    /// Device count by region
    pub region_counts: HashMap<String, u64>,
    /// Total registered devices
    pub total_devices: u64,
    /// Total active devices
    pub active_devices: u64,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        DeviceRegistry {
            devices: HashMap::new(),
            type_counts: HashMap::new(),
            region_counts: HashMap::new(),
            total_devices: 0,
            active_devices: 0,
        }
    }

    /// Register a new device
    pub fn register_device(
        &mut self,
        public_key: String,
        device_type: DeviceType,
        region: GeoRegion,
    ) -> Result<Device, String> {
        // Check if device already registered
        let device_id = Self::compute_device_id(&public_key);
        if self.devices.contains_key(&device_id) {
            return Err("Device already registered".to_string());
        }

        let device = Device::new(public_key, device_type.clone(), region.clone());

        // Update counts
        let type_key = format!("{:?}", device_type);
        *self.type_counts.entry(type_key).or_insert(0) += 1;
        *self.region_counts.entry(region.region_key()).or_insert(0) += 1;
        self.total_devices += 1;
        self.active_devices += 1;

        let device_clone = device.clone();
        self.devices.insert(device.device_id.clone(), device);

        info!(
            "Device {} registered: {:?} in {}",
            &device_clone.device_id, device_type, region.country_code
        );

        Ok(device_clone)
    }

    /// Get device by ID
    pub fn get_device(&self, device_id: &str) -> Option<&Device> {
        self.devices.get(device_id)
    }

    /// Get device by public key
    pub fn get_device_by_pubkey(&self, public_key: &str) -> Option<&Device> {
        let device_id = Self::compute_device_id(public_key);
        self.devices.get(&device_id)
    }

    /// Get mutable device by ID
    pub fn get_device_mut(&mut self, device_id: &str) -> Option<&mut Device> {
        self.devices.get_mut(device_id)
    }

    /// Compute device ID from public key
    fn compute_device_id(public_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let hash = hasher.finalize();
        format!("DEV_{}", hex::encode(&hash[..8]))
    }

    /// Get scarcity multiplier for a region
    /// Regions with fewer devices get higher multipliers
    pub fn get_region_scarcity(&self, region: &GeoRegion) -> f64 {
        let region_key = region.region_key();
        let region_count = *self.region_counts.get(&region_key).unwrap_or(&0);

        if region_count == 0 {
            return 3.0; // Very high scarcity for new regions
        }

        let avg_per_region = self.total_devices as f64 / self.region_counts.len().max(1) as f64;
        let scarcity = avg_per_region / region_count as f64;

        // Clamp between 0.5 and 3.0
        scarcity.clamp(0.5, 3.0)
    }

    /// Get scarcity multiplier for a device type
    pub fn get_type_scarcity(&self, device_type: &DeviceType) -> f64 {
        let type_key = format!("{:?}", device_type);
        let type_count = *self.type_counts.get(&type_key).unwrap_or(&0);

        if type_count == 0 {
            return 2.0; // High scarcity for new types
        }

        let avg_per_type = self.total_devices as f64 / self.type_counts.len().max(1) as f64;
        let scarcity = avg_per_type / type_count as f64;

        // Clamp between 0.5 and 2.0
        scarcity.clamp(0.5, 2.0)
    }

    /// Get all active devices eligible for validation
    pub fn get_eligible_validators(&self) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|d| d.is_active && d.reputation >= 30.0 && d.total_contributions >= 10)
            .collect()
    }

    /// Update device activity status (mark inactive if no activity)
    pub fn update_activity_status(&mut self, inactive_threshold_hours: i64) {
        let now = Utc::now();
        let mut newly_inactive = 0;

        for device in self.devices.values_mut() {
            if device.is_active {
                let hours_since_active = (now - device.last_active).num_hours();
                if hours_since_active > inactive_threshold_hours {
                    device.is_active = false;
                    newly_inactive += 1;
                }
            }
        }

        if newly_inactive > 0 {
            self.active_devices = self.active_devices.saturating_sub(newly_inactive);
            debug!("{} devices marked as inactive", newly_inactive);
        }
    }

    /// Get network statistics
    pub fn get_stats(&self) -> DeviceRegistryStats {
        let total_contribution_points: f64 =
            self.devices.values().map(|d| d.contribution_points).sum();

        let avg_reputation: f64 = if self.devices.is_empty() {
            0.0
        } else {
            self.devices.values().map(|d| d.reputation).sum::<f64>() / self.devices.len() as f64
        };

        DeviceRegistryStats {
            total_devices: self.total_devices,
            active_devices: self.active_devices,
            total_contribution_points,
            average_reputation: avg_reputation,
            regions_covered: self.region_counts.len() as u64,
            device_types: self.type_counts.len() as u64,
        }
    }
}

/// Device registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistryStats {
    pub total_devices: u64,
    pub active_devices: u64,
    pub total_contribution_points: f64,
    pub average_reputation: f64,
    pub regions_covered: u64,
    pub device_types: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_registration() {
        let mut registry = DeviceRegistry::new();

        let result = registry.register_device(
            "test_public_key_123".to_string(),
            DeviceType::Sensor,
            GeoRegion::new("US"),
        );

        assert!(result.is_ok());
        let device = result.unwrap();
        assert!(device.device_id.starts_with("DEV_"));
        assert_eq!(registry.total_devices, 1);
    }

    #[test]
    fn test_contribution_recording() {
        let mut device = Device::new(
            "test_key".to_string(),
            DeviceType::Sensor,
            GeoRegion::new("JP"),
        );

        device.record_contribution(0.8, 100.0);
        assert_eq!(device.total_contributions, 1);
        assert_eq!(device.contribution_points, 100.0);
        assert!(!device.quality_history.is_empty());
    }

    #[test]
    fn test_scarcity_calculation() {
        let mut registry = DeviceRegistry::new();

        // Register devices in US
        for i in 0..10 {
            let _ = registry.register_device(
                format!("key_us_{}", i),
                DeviceType::Sensor,
                GeoRegion::new("US"),
            );
        }

        // Register one device in JP
        let _ = registry.register_device(
            "key_jp_1".to_string(),
            DeviceType::Sensor,
            GeoRegion::new("JP"),
        );

        let us_scarcity = registry.get_region_scarcity(&GeoRegion::new("US"));
        let jp_scarcity = registry.get_region_scarcity(&GeoRegion::new("JP"));

        // JP should have higher scarcity (fewer devices)
        assert!(jp_scarcity > us_scarcity);
    }
}
