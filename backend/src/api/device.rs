//! Device Registry API endpoints for EdgeAI Blockchain
//!
//! This module provides HTTP endpoints for IoT device registration,
//! contribution tracking, and scarcity calculations.

#![allow(dead_code)]

use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::rest::ApiResponse;
use crate::consensus::{Device, DeviceRegistry, DeviceType, GeoRegion};

/// Device registry state (shared across handlers)
pub struct DeviceState {
    pub registry: Arc<RwLock<DeviceRegistry>>,
}

// ============ Request/Response Types ============

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub public_key: String,
    pub device_type: String,
    pub country_code: String,
    pub region_code: Option<String>,
    pub latitude: Option<i32>,
    pub longitude: Option<i32>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordContributionRequest {
    pub device_id: String,
    pub quality_score: f64,
    pub points: f64,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub device_id: String,
    pub device_type: String,
    pub region: String,
    pub reputation: f64,
    pub total_contributions: u64,
    pub contribution_points: f64,
    pub is_active: bool,
    pub is_verified: bool,
    pub validator_weight: f64,
}

impl From<&Device> for DeviceResponse {
    fn from(device: &Device) -> Self {
        DeviceResponse {
            device_id: device.device_id.clone(),
            device_type: format!("{:?}", device.device_type),
            region: device.region.region_key(),
            reputation: device.reputation,
            total_contributions: device.total_contributions,
            contribution_points: device.contribution_points,
            is_active: device.is_active,
            is_verified: device.is_verified,
            validator_weight: device.validator_weight(),
        }
    }
}

// ============ Helper Functions ============

fn parse_device_type(type_str: &str) -> DeviceType {
    match type_str.to_lowercase().as_str() {
        "sensor" => DeviceType::Sensor,
        "camera" => DeviceType::Camera,
        "audio" => DeviceType::Audio,
        "location" => DeviceType::Location,
        "industrial" => DeviceType::Industrial,
        "smarthome" | "smart_home" => DeviceType::SmartHome,
        "wearable" => DeviceType::Wearable,
        "vehicle" => DeviceType::Vehicle,
        "environmental" => DeviceType::Environmental,
        "medical" => DeviceType::Medical,
        "agricultural" => DeviceType::Agricultural,
        "energy" => DeviceType::Energy,
        _ => DeviceType::Custom(type_str.to_string()),
    }
}

// ============ Device Registry Endpoints ============

/// Register a new device
pub async fn register_device(
    data: web::Data<DeviceState>,
    body: web::Json<RegisterDeviceRequest>,
) -> impl Responder {
    let device_type = parse_device_type(&body.device_type);

    let region = match (body.latitude, body.longitude) {
        (Some(lat), Some(lon)) => GeoRegion::with_coordinates(&body.country_code, lat, lon),
        _ => {
            let mut r = GeoRegion::new(&body.country_code);
            r.region_code = body.region_code.clone();
            r
        }
    };

    let mut registry = data.registry.write().await;

    match registry.register_device(body.public_key.clone(), device_type, region) {
        Ok(device) => {
            info!(
                "Device registered: {} ({:?}) in {}",
                &device.device_id, device.device_type, device.region.country_code
            );

            let response = DeviceResponse::from(&device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Get device by ID
pub async fn get_device(data: web::Data<DeviceState>, path: web::Path<String>) -> impl Responder {
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

/// Get all registered devices
pub async fn get_all_devices(data: web::Data<DeviceState>) -> impl Responder {
    let registry = data.registry.read().await;

    let devices: Vec<DeviceResponse> = registry
        .devices
        .values()
        .map(DeviceResponse::from)
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(devices))
}

/// Get eligible validators (devices that can validate blocks)
pub async fn get_eligible_validators(data: web::Data<DeviceState>) -> impl Responder {
    let registry = data.registry.read().await;

    let validators: Vec<DeviceResponse> = registry
        .get_eligible_validators()
        .into_iter()
        .map(DeviceResponse::from)
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(validators))
}

/// Get device registry statistics
pub async fn get_device_stats(data: web::Data<DeviceState>) -> impl Responder {
    let registry = data.registry.read().await;
    let stats = registry.get_stats();
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Record a data contribution for a device
pub async fn record_contribution(
    data: web::Data<DeviceState>,
    body: web::Json<RecordContributionRequest>,
) -> impl Responder {
    let mut registry = data.registry.write().await;

    match registry.get_device_mut(&body.device_id) {
        Some(device) => {
            device.record_contribution(body.quality_score, body.points);

            info!(
                "Contribution recorded for {}: quality={:.2}, points={:.2}",
                &body.device_id, body.quality_score, body.points
            );

            let response = DeviceResponse::from(&*device);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Device not found")),
    }
}

/// Get region scarcity multiplier
pub async fn get_region_scarcity(
    data: web::Data<DeviceState>,
    path: web::Path<String>,
) -> impl Responder {
    let country_code = path.into_inner();
    let registry = data.registry.read().await;

    let region = GeoRegion::new(&country_code);
    let scarcity = registry.get_region_scarcity(&region);

    #[derive(Serialize)]
    struct ScarcityResponse {
        country_code: String,
        scarcity_multiplier: f64,
    }

    HttpResponse::Ok().json(ApiResponse::success(ScarcityResponse {
        country_code,
        scarcity_multiplier: scarcity,
    }))
}

/// Get device type scarcity multiplier
pub async fn get_type_scarcity(
    data: web::Data<DeviceState>,
    path: web::Path<String>,
) -> impl Responder {
    let type_str = path.into_inner();
    let registry = data.registry.read().await;

    let device_type = parse_device_type(&type_str);
    let scarcity = registry.get_type_scarcity(&device_type);

    #[derive(Serialize)]
    struct ScarcityResponse {
        device_type: String,
        scarcity_multiplier: f64,
    }

    HttpResponse::Ok().json(ApiResponse::success(ScarcityResponse {
        device_type: type_str,
        scarcity_multiplier: scarcity,
    }))
}

// ============ Router Configuration ============

pub fn configure_device_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Device registry routes
        .route("/api/devices", web::get().to(get_all_devices))
        .route("/api/devices/register", web::post().to(register_device))
        .route("/api/devices/stats", web::get().to(get_device_stats))
        .route(
            "/api/devices/validators",
            web::get().to(get_eligible_validators),
        )
        .route("/api/devices/{device_id}", web::get().to(get_device))
        .route(
            "/api/devices/contribute",
            web::post().to(record_contribution),
        )
        .route(
            "/api/devices/scarcity/region/{country_code}",
            web::get().to(get_region_scarcity),
        )
        .route(
            "/api/devices/scarcity/type/{type}",
            web::get().to(get_type_scarcity),
        );
}
