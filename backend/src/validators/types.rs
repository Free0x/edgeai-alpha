use serde::{Deserialize, Serialize};

/// 验证者状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ValidatorStatus {
    Online,
    Offline,
    Maintenance,
}

/// 验证者节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorNode {
    pub id: String,
    pub name: String,
    pub status: ValidatorStatus,
    pub blocks_mined: u64,
    pub reputation: f64,
    pub uptime: f64,
    pub location: String,
    pub lat: f64,
    pub lng: f64,
}

/// 验证者列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorListResponse {
    pub validators: Vec<ValidatorNode>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
    pub stats: ValidatorStats,
}

/// 验证者统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStats {
    pub online: u64,
    pub offline: u64,
    pub maintenance: u64,
    /// Network Entropy - PoIE 核心指标，基于验证者贡献的信息熵计算
    pub network_entropy: f64,
    /// 总挖矿区块数
    pub total_blocks_mined: u64,
    /// 平均信誉值
    pub avg_reputation: f64,
}

/// 地球标记点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobeMarker {
    pub location: (f64, f64),
    pub size: f64,
    pub tooltip: String,
    #[serde(rename = "type")]
    pub marker_type: String,
    pub validator_count: u64,
}

/// 验证者地图响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorMapResponse {
    pub markers: Vec<GlobeMarker>,
    pub total_validators: u64,
}

/// 验证者位置（数据中心）
#[derive(Debug, Clone)]
pub struct ValidatorLocation {
    pub name: &'static str,
    pub lat: f64,
    pub lng: f64,
}

impl ValidatorLocation {
    pub fn all() -> Vec<ValidatorLocation> {
        vec![
            // North America
            ValidatorLocation {
                name: "US-East (N. Virginia)",
                lat: 39.0438,
                lng: -77.4874,
            },
            ValidatorLocation {
                name: "US-East (New York)",
                lat: 40.7128,
                lng: -74.0060,
            },
            ValidatorLocation {
                name: "US-West (California)",
                lat: 37.3382,
                lng: -121.8863,
            },
            ValidatorLocation {
                name: "US-West (Oregon)",
                lat: 45.5152,
                lng: -122.6784,
            },
            ValidatorLocation {
                name: "US-Central (Texas)",
                lat: 30.2672,
                lng: -97.7431,
            },
            ValidatorLocation {
                name: "Canada (Toronto)",
                lat: 43.6532,
                lng: -79.3832,
            },
            ValidatorLocation {
                name: "Canada (Montreal)",
                lat: 45.5017,
                lng: -73.5673,
            },
            // Europe
            ValidatorLocation {
                name: "EU-Central (Frankfurt)",
                lat: 50.1109,
                lng: 8.6821,
            },
            ValidatorLocation {
                name: "EU-West (London)",
                lat: 51.5074,
                lng: -0.1278,
            },
            ValidatorLocation {
                name: "EU-West (Paris)",
                lat: 48.8566,
                lng: 2.3522,
            },
            ValidatorLocation {
                name: "EU-North (Stockholm)",
                lat: 59.3293,
                lng: 18.0686,
            },
            ValidatorLocation {
                name: "EU-West (Ireland)",
                lat: 53.3498,
                lng: -6.2603,
            },
            ValidatorLocation {
                name: "EU-South (Milan)",
                lat: 45.4642,
                lng: 9.1900,
            },
            // Asia Pacific
            ValidatorLocation {
                name: "Asia-East (Tokyo)",
                lat: 35.6762,
                lng: 139.6503,
            },
            ValidatorLocation {
                name: "Asia-East (Seoul)",
                lat: 37.5665,
                lng: 126.9780,
            },
            ValidatorLocation {
                name: "Asia-East (Hong Kong)",
                lat: 22.3193,
                lng: 114.1694,
            },
            ValidatorLocation {
                name: "Asia-South (Singapore)",
                lat: 1.3521,
                lng: 103.8198,
            },
            ValidatorLocation {
                name: "Asia-South (Mumbai)",
                lat: 19.0760,
                lng: 72.8777,
            },
            ValidatorLocation {
                name: "AU-East (Sydney)",
                lat: -33.8688,
                lng: 151.2093,
            },
            ValidatorLocation {
                name: "AU-South (Melbourne)",
                lat: -37.8136,
                lng: 144.9631,
            },
            // South America
            ValidatorLocation {
                name: "SA-East (Sao Paulo)",
                lat: -23.5505,
                lng: -46.6333,
            },
            ValidatorLocation {
                name: "SA-West (Santiago)",
                lat: -33.4489,
                lng: -70.6693,
            },
            // Middle East & Africa
            ValidatorLocation {
                name: "ME-Central (Dubai)",
                lat: 25.2048,
                lng: 55.2708,
            },
            ValidatorLocation {
                name: "AF-South (Cape Town)",
                lat: -33.9249,
                lng: 18.4241,
            },
        ]
    }
}
