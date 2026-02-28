use serde::{Deserialize, Serialize};

/// IoT 行业分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IoTSector {
    #[serde(rename = "smart_city")]
    SmartCity,
    #[serde(rename = "industrial")]
    Industrial,
    #[serde(rename = "agriculture")]
    Agriculture,
    #[serde(rename = "healthcare")]
    Healthcare,
    #[serde(rename = "logistics")]
    Logistics,
    #[serde(rename = "energy")]
    Energy,
}

impl IoTSector {
    pub fn display_name(&self) -> &'static str {
        match self {
            IoTSector::SmartCity => "Smart City",
            IoTSector::Industrial => "Industrial IoT",
            IoTSector::Agriculture => "Smart Agriculture",
            IoTSector::Healthcare => "Healthcare",
            IoTSector::Logistics => "Logistics",
            IoTSector::Energy => "Energy Grid",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            IoTSector::SmartCity => "Building2",
            IoTSector::Industrial => "Factory",
            IoTSector::Agriculture => "Leaf",
            IoTSector::Healthcare => "HeartPulse",
            IoTSector::Logistics => "Truck",
            IoTSector::Energy => "Zap",
        }
    }

    pub fn devices(&self) -> Vec<&'static str> {
        match self {
            IoTSector::SmartCity => vec![
                "Traffic Cam",
                "Street Light",
                "Air Quality Sensor",
                "Parking Meter",
                "Waste Bin Sensor",
            ],
            IoTSector::Industrial => vec![
                "Pressure Valve",
                "Robotic Arm",
                "Temp Controller",
                "Conveyor Belt",
                "Hydraulic Pump",
            ],
            IoTSector::Agriculture => vec![
                "Soil Moisture Sensor",
                "Drone Scout",
                "Weather Station",
                "Irrigation Pump",
                "Livestock Tracker",
            ],
            IoTSector::Healthcare => vec![
                "Heart Rate Monitor",
                "Insulin Pump",
                "Sleep Tracker",
                "Smart Scale",
                "Activity Band",
            ],
            IoTSector::Logistics => vec![
                "Fleet Tracker",
                "Cargo Sensor",
                "Cold Chain Monitor",
                "RFID Scanner",
                "Warehouse Bot",
            ],
            IoTSector::Energy => vec![
                "Smart Meter",
                "Solar Inverter",
                "Grid Monitor",
                "Battery Storage",
                "EV Charger",
            ],
        }
    }

    pub fn all() -> Vec<IoTSector> {
        vec![
            IoTSector::SmartCity,
            IoTSector::Industrial,
            IoTSector::Agriculture,
            IoTSector::Healthcare,
            IoTSector::Logistics,
            IoTSector::Energy,
        ]
    }

    pub fn from_index(index: usize) -> IoTSector {
        let sectors = Self::all();
        sectors[index % sectors.len()].clone()
    }
}

/// IoT 交易数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTTransaction {
    pub id: String,
    pub hash: String,
    pub tx_type: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub timestamp: String,
    pub status: String,
    pub sector: String,
    pub device_type: String,
    pub location: String,
    pub coordinates: (f64, f64),
    pub data_payload: String,
}

/// IoT 交易列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTTransactionListResponse {
    pub transactions: Vec<IoTTransaction>,
    pub total: u64,
    pub page: u64,
    pub limit: u64,
}

/// 地理位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub coords: (f64, f64),
}

impl Location {
    pub fn all() -> Vec<Location> {
        vec![
            Location {
                name: "New York, USA".to_string(),
                coords: (40.7128, -74.0060),
            },
            Location {
                name: "London, UK".to_string(),
                coords: (51.5074, -0.1278),
            },
            Location {
                name: "Tokyo, Japan".to_string(),
                coords: (35.6762, 139.6503),
            },
            Location {
                name: "Singapore".to_string(),
                coords: (1.3521, 103.8198),
            },
            Location {
                name: "Berlin, Germany".to_string(),
                coords: (52.5200, 13.4050),
            },
            Location {
                name: "Shanghai, China".to_string(),
                coords: (31.2304, 121.4737),
            },
            Location {
                name: "Sydney, Australia".to_string(),
                coords: (-33.8688, 151.2093),
            },
            Location {
                name: "Toronto, Canada".to_string(),
                coords: (43.6532, -79.3832),
            },
            Location {
                name: "Paris, France".to_string(),
                coords: (48.8566, 2.3522),
            },
            Location {
                name: "Seoul, South Korea".to_string(),
                coords: (37.5665, 126.9780),
            },
            Location {
                name: "Mumbai, India".to_string(),
                coords: (19.0760, 72.8777),
            },
            Location {
                name: "Sao Paulo, Brazil".to_string(),
                coords: (-23.5505, -46.6333),
            },
            Location {
                name: "Dubai, UAE".to_string(),
                coords: (25.2048, 55.2708),
            },
            Location {
                name: "Amsterdam, Netherlands".to_string(),
                coords: (52.3676, 4.9041),
            },
            Location {
                name: "Stockholm, Sweden".to_string(),
                coords: (59.3293, 18.0686),
            },
            Location {
                name: "San Francisco, USA".to_string(),
                coords: (37.7749, -122.4194),
            },
            Location {
                name: "Bangalore, India".to_string(),
                coords: (12.9716, 77.5946),
            },
            Location {
                name: "Shenzhen, China".to_string(),
                coords: (22.5431, 114.0579),
            },
            Location {
                name: "Austin, USA".to_string(),
                coords: (30.2672, -97.7431),
            },
            Location {
                name: "Tel Aviv, Israel".to_string(),
                coords: (32.0853, 34.7818),
            },
        ]
    }
}
