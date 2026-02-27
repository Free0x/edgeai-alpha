use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::api::rest::ApiResponse;
use crate::iot::{IoTGenerator, IoTTransactionListResponse};
use crate::validators::{ValidatorGenerator, ValidatorListResponse};

// ============ Query Types ============

#[derive(Debug, Deserialize)]
pub struct IoTQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    pub sector: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ValidatorQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    pub status: Option<String>,
}

// ============ IoT Endpoints ============

/// 获取 IoT 交易列表
pub async fn get_iot_transactions(query: web::Query<IoTQuery>) -> impl Responder {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);

    let generator = IoTGenerator::new();
    let transactions = generator.generate_transactions(page, limit);

    // 如果指定了行业过滤
    let filtered_transactions = if let Some(ref sector) = query.sector {
        transactions
            .into_iter()
            .filter(|tx| tx.sector.to_lowercase().contains(&sector.to_lowercase()))
            .collect()
    } else {
        transactions
    };

    let response = IoTTransactionListResponse {
        transactions: filtered_transactions,
        total: 100000, // 模拟总数
        page,
        limit,
    };

    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// 获取 IoT 行业统计
pub async fn get_iot_sectors() -> impl Responder {
    use crate::iot::types::IoTSector;

    let sectors: Vec<serde_json::Value> = IoTSector::all()
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": format!("{:?}", s).to_lowercase(),
                "name": s.display_name(),
                "icon": s.icon(),
                "devices": s.devices(),
            })
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(sectors))
}

// ============ Validator Endpoints ============

/// 获取验证者节点列表
pub async fn get_validator_nodes(query: web::Query<ValidatorQuery>) -> impl Responder {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(100).min(1000);

    let generator = ValidatorGenerator::new();
    let mut validators = generator.generate_validators(page, limit);

    // 如果指定了状态过滤
    if let Some(ref status) = query.status {
        use crate::validators::types::ValidatorStatus;
        let target_status = match status.to_lowercase().as_str() {
            "online" => Some(ValidatorStatus::Online),
            "offline" => Some(ValidatorStatus::Offline),
            "maintenance" => Some(ValidatorStatus::Maintenance),
            _ => None,
        };

        if let Some(target) = target_status {
            validators = validators
                .into_iter()
                .filter(|v| v.status == target)
                .collect();
        }
    }

    let stats = generator.get_stats();

    let response = ValidatorListResponse {
        validators,
        total: generator.total_count(),
        page,
        limit,
        stats,
    };

    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// 获取验证者地图数据
pub async fn get_validator_map() -> impl Responder {
    let generator = ValidatorGenerator::new();
    let map_response = generator.generate_map_markers();

    HttpResponse::Ok().json(ApiResponse::success(map_response))
}

/// 获取网络统计（增强版）
pub async fn get_network_stats_enhanced() -> impl Responder {
    use crate::iot::types::IoTSector;

    let validator_generator = ValidatorGenerator::new();
    let validator_stats = validator_generator.get_stats();

    // 模拟行业交易统计
    let mut sector_stats = std::collections::HashMap::new();
    for sector in IoTSector::all() {
        let count = match sector {
            IoTSector::SmartCity => 15234,
            IoTSector::Industrial => 12456,
            IoTSector::Agriculture => 8765,
            IoTSector::Healthcare => 6543,
            IoTSector::Logistics => 5432,
            IoTSector::Energy => 4321,
        };
        sector_stats.insert(format!("{:?}", sector).to_lowercase(), count);
    }

    let response = serde_json::json!({
        "height": 12045,
        "difficulty": 2.14,
        "avg_block_time": 10.2,
        "tps": 0.6,
        "total_transactions": 589201,
        "total_validators": validator_generator.total_count(),
        "active_validators": validator_stats.online,
        "sectors": sector_stats,
    });

    HttpResponse::Ok().json(ApiResponse::success(response))
}

// ============ Router Configuration ============

pub fn configure_data_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // IoT routes
        .route("/api/iot/transactions", web::get().to(get_iot_transactions))
        .route("/api/iot/sectors", web::get().to(get_iot_sectors))
        
        // Validator routes (new)
        .route("/api/validators/nodes", web::get().to(get_validator_nodes))
        .route("/api/validators/map", web::get().to(get_validator_map))
        
        // Enhanced network stats
        .route("/api/network/stats", web::get().to(get_network_stats_enhanced));
}
