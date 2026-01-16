//! Prometheus Metrics Exporter
//!
//! Exposes blockchain and system metrics in Prometheus format for monitoring.

use std::sync::Arc;
use std::time::Instant;
use std::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Prometheus metric types
#[derive(Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

impl MetricType {
    fn as_str(&self) -> &'static str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
            MetricType::Summary => "summary",
        }
    }
}

/// A single metric definition
#[derive(Clone)]
pub struct MetricDef {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub labels: Vec<String>,
}

/// Metric value with labels
#[derive(Clone)]
pub struct MetricValue {
    pub labels: Vec<(String, String)>,
    pub value: f64,
}

/// Prometheus metrics registry
pub struct PrometheusRegistry {
    metrics: RwLock<Vec<(MetricDef, Vec<MetricValue>)>>,
    start_time: Instant,
}

impl PrometheusRegistry {
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(Vec::new()),
            start_time: Instant::now(),
        }
    }

    /// Register a new metric
    pub fn register(&self, def: MetricDef) {
        let mut metrics = self.metrics.write().unwrap();
        if !metrics.iter().any(|(d, _)| d.name == def.name) {
            metrics.push((def, Vec::new()));
        }
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64, labels: Vec<(String, String)>) {
        let mut metrics = self.metrics.write().unwrap();
        if let Some((_, values)) = metrics.iter_mut().find(|(d, _)| d.name == name) {
            // Find and update existing label combination or add new
            if let Some(v) = values.iter_mut().find(|v| v.labels == labels) {
                v.value = value;
            } else {
                values.push(MetricValue { labels, value });
            }
        }
    }

    /// Increment a counter
    pub fn inc_counter(&self, name: &str, delta: f64, labels: Vec<(String, String)>) {
        let mut metrics = self.metrics.write().unwrap();
        if let Some((_, values)) = metrics.iter_mut().find(|(d, _)| d.name == name) {
            if let Some(v) = values.iter_mut().find(|v| v.labels == labels) {
                v.value += delta;
            } else {
                values.push(MetricValue { labels, value: delta });
            }
        }
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> String {
        let metrics = self.metrics.read().unwrap();
        let mut output = String::new();

        for (def, values) in metrics.iter() {
            // HELP line
            output.push_str(&format!("# HELP {} {}\n", def.name, def.help));
            // TYPE line
            output.push_str(&format!("# TYPE {} {}\n", def.name, def.metric_type.as_str()));
            
            // Metric values
            for value in values {
                if value.labels.is_empty() {
                    output.push_str(&format!("{} {}\n", def.name, value.value));
                } else {
                    let labels: Vec<String> = value.labels
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect();
                    output.push_str(&format!(
                        "{}{{{}}} {}\n",
                        def.name,
                        labels.join(","),
                        value.value
                    ));
                }
            }
            output.push('\n');
        }

        output
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

/// Blockchain-specific metrics collector
pub struct BlockchainMetrics {
    registry: Arc<PrometheusRegistry>,
}

impl BlockchainMetrics {
    pub fn new() -> Self {
        let registry = Arc::new(PrometheusRegistry::new());
        
        // Register blockchain metrics
        let metrics = vec![
            MetricDef {
                name: "edgeai_block_height".to_string(),
                help: "Current blockchain height".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_transactions_total".to_string(),
                help: "Total number of transactions processed".to_string(),
                metric_type: MetricType::Counter,
                labels: vec!["type".to_string()],
            },
            MetricDef {
                name: "edgeai_tps".to_string(),
                help: "Current transactions per second".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_active_validators".to_string(),
                help: "Number of active validators".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_total_stake".to_string(),
                help: "Total staked tokens".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_network_entropy".to_string(),
                help: "Network entropy score".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_mempool_size".to_string(),
                help: "Number of pending transactions in mempool".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_peer_count".to_string(),
                help: "Number of connected P2P peers".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_api_requests_total".to_string(),
                help: "Total API requests".to_string(),
                metric_type: MetricType::Counter,
                labels: vec!["endpoint".to_string(), "status".to_string()],
            },
            MetricDef {
                name: "edgeai_api_latency_seconds".to_string(),
                help: "API request latency in seconds".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec!["endpoint".to_string()],
            },
            MetricDef {
                name: "edgeai_storage_bytes".to_string(),
                help: "Storage size in bytes".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec!["type".to_string()],
            },
            MetricDef {
                name: "edgeai_data_contributions_total".to_string(),
                help: "Total data contributions from IoT devices".to_string(),
                metric_type: MetricType::Counter,
                labels: vec!["category".to_string()],
            },
            MetricDef {
                name: "edgeai_data_quality_score".to_string(),
                help: "Average data quality score".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec!["category".to_string()],
            },
            MetricDef {
                name: "edgeai_bridge_transfers_total".to_string(),
                help: "Total bridge transfers".to_string(),
                metric_type: MetricType::Counter,
                labels: vec!["direction".to_string()],
            },
            MetricDef {
                name: "edgeai_bridge_volume".to_string(),
                help: "Total bridge transfer volume".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec!["direction".to_string()],
            },
            MetricDef {
                name: "edgeai_process_uptime_seconds".to_string(),
                help: "Process uptime in seconds".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
            MetricDef {
                name: "edgeai_rate_limit_hits_total".to_string(),
                help: "Total rate limit hits".to_string(),
                metric_type: MetricType::Counter,
                labels: vec!["tier".to_string()],
            },
            MetricDef {
                name: "edgeai_blocked_ips".to_string(),
                help: "Number of blocked IPs".to_string(),
                metric_type: MetricType::Gauge,
                labels: vec![],
            },
        ];

        for metric in metrics {
            registry.register(metric);
        }

        Self { registry }
    }

    /// Update blockchain metrics
    pub fn update_chain_stats(&self, height: u64, total_tx: u64, tps: f64, entropy: f64) {
        self.registry.set_gauge("edgeai_block_height", height as f64, vec![]);
        self.registry.set_gauge("edgeai_tps", tps, vec![]);
        self.registry.set_gauge("edgeai_network_entropy", entropy, vec![]);
        
        // Update transaction counter
        self.registry.set_gauge(
            "edgeai_transactions_total",
            total_tx as f64,
            vec![("type".to_string(), "all".to_string())],
        );
    }

    /// Update validator metrics
    pub fn update_validator_stats(&self, active_count: u64, total_stake: u64) {
        self.registry.set_gauge("edgeai_active_validators", active_count as f64, vec![]);
        self.registry.set_gauge("edgeai_total_stake", total_stake as f64, vec![]);
    }

    /// Update mempool metrics
    pub fn update_mempool(&self, size: usize) {
        self.registry.set_gauge("edgeai_mempool_size", size as f64, vec![]);
    }

    /// Update peer count
    pub fn update_peers(&self, count: usize) {
        self.registry.set_gauge("edgeai_peer_count", count as f64, vec![]);
    }

    /// Record API request
    pub fn record_api_request(&self, endpoint: &str, status: u16, latency_ms: f64) {
        self.registry.inc_counter(
            "edgeai_api_requests_total",
            1.0,
            vec![
                ("endpoint".to_string(), endpoint.to_string()),
                ("status".to_string(), status.to_string()),
            ],
        );
        self.registry.set_gauge(
            "edgeai_api_latency_seconds",
            latency_ms / 1000.0,
            vec![("endpoint".to_string(), endpoint.to_string())],
        );
    }

    /// Record data contribution
    pub fn record_data_contribution(&self, category: &str, quality_score: f64) {
        self.registry.inc_counter(
            "edgeai_data_contributions_total",
            1.0,
            vec![("category".to_string(), category.to_string())],
        );
        self.registry.set_gauge(
            "edgeai_data_quality_score",
            quality_score,
            vec![("category".to_string(), category.to_string())],
        );
    }

    /// Record bridge transfer
    pub fn record_bridge_transfer(&self, direction: &str, amount: f64) {
        self.registry.inc_counter(
            "edgeai_bridge_transfers_total",
            1.0,
            vec![("direction".to_string(), direction.to_string())],
        );
        self.registry.set_gauge(
            "edgeai_bridge_volume",
            amount,
            vec![("direction".to_string(), direction.to_string())],
        );
    }

    /// Update security metrics
    pub fn update_security(&self, blocked_ips: usize, rate_limit_hits: u64) {
        self.registry.set_gauge("edgeai_blocked_ips", blocked_ips as f64, vec![]);
        self.registry.set_gauge(
            "edgeai_rate_limit_hits_total",
            rate_limit_hits as f64,
            vec![("tier".to_string(), "all".to_string())],
        );
    }

    /// Update uptime
    pub fn update_uptime(&self) {
        self.registry.set_gauge(
            "edgeai_process_uptime_seconds",
            self.registry.uptime_seconds(),
            vec![],
        );
    }

    /// Export all metrics
    pub fn export(&self) -> String {
        self.update_uptime();
        self.registry.export()
    }

    /// Get registry reference
    pub fn registry(&self) -> Arc<PrometheusRegistry> {
        Arc::clone(&self.registry)
    }
}

/// Global metrics instance
pub type GlobalBlockchainMetrics = Arc<BlockchainMetrics>;

pub fn create_blockchain_metrics() -> GlobalBlockchainMetrics {
    Arc::new(BlockchainMetrics::new())
}

/// Grafana dashboard configuration (JSON)
pub fn grafana_dashboard_config() -> serde_json::Value {
    serde_json::json!({
        "dashboard": {
            "title": "EdgeAI Blockchain Dashboard",
            "tags": ["edgeai", "blockchain"],
            "timezone": "browser",
            "panels": [
                {
                    "title": "Block Height",
                    "type": "stat",
                    "targets": [{"expr": "edgeai_block_height"}]
                },
                {
                    "title": "Transactions Per Second",
                    "type": "graph",
                    "targets": [{"expr": "edgeai_tps"}]
                },
                {
                    "title": "Active Validators",
                    "type": "stat",
                    "targets": [{"expr": "edgeai_active_validators"}]
                },
                {
                    "title": "Network Entropy",
                    "type": "gauge",
                    "targets": [{"expr": "edgeai_network_entropy"}]
                },
                {
                    "title": "API Latency",
                    "type": "graph",
                    "targets": [{"expr": "edgeai_api_latency_seconds"}]
                },
                {
                    "title": "Data Contributions by Category",
                    "type": "piechart",
                    "targets": [{"expr": "edgeai_data_contributions_total"}]
                }
            ]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_export() {
        let metrics = BlockchainMetrics::new();
        metrics.update_chain_stats(1000, 50000, 15.5, 100.0);
        
        let output = metrics.export();
        assert!(output.contains("edgeai_block_height"));
        assert!(output.contains("1000"));
    }
}
