//! Performance Metrics Module
//!
//! Collects and exposes performance metrics for monitoring.
//! Compatible with Prometheus format.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Counter metric (monotonically increasing)
#[derive(Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Gauge metric (can go up or down)
#[derive(Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, val: u64) {
        self.value.store(val, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// Histogram for tracking distributions
pub struct Histogram {
    buckets: Vec<(f64, AtomicU64)>, // (upper bound, count)
    sum: AtomicU64,                  // Sum of all values (as micros)
    count: AtomicU64,
}

impl Histogram {
    pub fn new(bucket_bounds: Vec<f64>) -> Self {
        let buckets = bucket_bounds
            .into_iter()
            .map(|b| (b, AtomicU64::new(0)))
            .collect();
        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    /// Default buckets for HTTP request latencies (in seconds)
    pub fn http_latency_buckets() -> Self {
        Self::new(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    pub fn observe(&self, value: f64) {
        // Update sum (store as micros for precision)
        let micros = (value * 1_000_000.0) as u64;
        self.sum.fetch_add(micros, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update bucket counts
        for (bound, count) in &self.buckets {
            if value <= *bound {
                count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn get_stats(&self) -> HistogramStats {
        let count = self.count.load(Ordering::Relaxed);
        let sum_micros = self.sum.load(Ordering::Relaxed);
        let sum = sum_micros as f64 / 1_000_000.0;
        
        let buckets: Vec<(f64, u64)> = self.buckets
            .iter()
            .map(|(bound, count)| (*bound, count.load(Ordering::Relaxed)))
            .collect();

        HistogramStats {
            count,
            sum,
            avg: if count > 0 { sum / count as f64 } else { 0.0 },
            buckets,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramStats {
    pub count: u64,
    pub sum: f64,
    pub avg: f64,
    pub buckets: Vec<(f64, u64)>,
}

/// Request timing helper
pub struct RequestTimer {
    start: Instant,
    histogram: Arc<Histogram>,
}

impl RequestTimer {
    pub fn new(histogram: Arc<Histogram>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }

    pub fn observe_duration(self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

/// Application metrics collection
pub struct Metrics {
    // HTTP metrics
    pub http_requests_total: Counter,
    pub http_requests_in_flight: Gauge,
    pub http_request_duration: Arc<Histogram>,
    pub http_requests_by_path: RwLock<HashMap<String, Counter>>,
    pub http_errors_total: Counter,
    
    // Blockchain metrics
    pub blocks_produced: Counter,
    pub transactions_processed: Counter,
    pub pending_transactions: Gauge,
    pub chain_height: Gauge,
    pub active_accounts: Gauge,
    
    // P2P metrics
    pub p2p_peers_connected: Gauge,
    pub p2p_messages_sent: Counter,
    pub p2p_messages_received: Counter,
    pub p2p_bytes_sent: Counter,
    pub p2p_bytes_received: Counter,
    
    // Cache metrics
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub cache_size: Gauge,
    
    // System metrics
    pub start_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            http_requests_total: Counter::new(),
            http_requests_in_flight: Gauge::new(),
            http_request_duration: Arc::new(Histogram::http_latency_buckets()),
            http_requests_by_path: RwLock::new(HashMap::new()),
            http_errors_total: Counter::new(),
            
            blocks_produced: Counter::new(),
            transactions_processed: Counter::new(),
            pending_transactions: Gauge::new(),
            chain_height: Gauge::new(),
            active_accounts: Gauge::new(),
            
            p2p_peers_connected: Gauge::new(),
            p2p_messages_sent: Counter::new(),
            p2p_messages_received: Counter::new(),
            p2p_bytes_sent: Counter::new(),
            p2p_bytes_received: Counter::new(),
            
            cache_hits: Counter::new(),
            cache_misses: Counter::new(),
            cache_size: Gauge::new(),
            
            start_time: Instant::now(),
        }
    }

    /// Record an HTTP request
    pub async fn record_request(&self, path: &str) {
        self.http_requests_total.inc();
        
        let mut by_path = self.http_requests_by_path.write().await;
        by_path
            .entry(path.to_string())
            .or_insert_with(Counter::new)
            .inc();
    }

    /// Start timing a request
    pub fn start_request_timer(&self) -> RequestTimer {
        self.http_requests_in_flight.inc();
        RequestTimer::new(self.http_request_duration.clone())
    }

    /// End request timing
    pub fn end_request(&self, timer: RequestTimer) {
        self.http_requests_in_flight.dec();
        timer.observe_duration();
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // HTTP metrics
        output.push_str(&format!(
            "# HELP edgeai_http_requests_total Total HTTP requests\n\
             # TYPE edgeai_http_requests_total counter\n\
             edgeai_http_requests_total {}\n\n",
            self.http_requests_total.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_http_requests_in_flight Current HTTP requests being processed\n\
             # TYPE edgeai_http_requests_in_flight gauge\n\
             edgeai_http_requests_in_flight {}\n\n",
            self.http_requests_in_flight.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_http_errors_total Total HTTP errors\n\
             # TYPE edgeai_http_errors_total counter\n\
             edgeai_http_errors_total {}\n\n",
            self.http_errors_total.get()
        ));
        
        // Blockchain metrics
        output.push_str(&format!(
            "# HELP edgeai_blocks_produced_total Total blocks produced\n\
             # TYPE edgeai_blocks_produced_total counter\n\
             edgeai_blocks_produced_total {}\n\n",
            self.blocks_produced.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_transactions_processed_total Total transactions processed\n\
             # TYPE edgeai_transactions_processed_total counter\n\
             edgeai_transactions_processed_total {}\n\n",
            self.transactions_processed.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_chain_height Current blockchain height\n\
             # TYPE edgeai_chain_height gauge\n\
             edgeai_chain_height {}\n\n",
            self.chain_height.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_pending_transactions Number of pending transactions\n\
             # TYPE edgeai_pending_transactions gauge\n\
             edgeai_pending_transactions {}\n\n",
            self.pending_transactions.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_active_accounts Number of active accounts\n\
             # TYPE edgeai_active_accounts gauge\n\
             edgeai_active_accounts {}\n\n",
            self.active_accounts.get()
        ));
        
        // P2P metrics
        output.push_str(&format!(
            "# HELP edgeai_p2p_peers_connected Number of connected P2P peers\n\
             # TYPE edgeai_p2p_peers_connected gauge\n\
             edgeai_p2p_peers_connected {}\n\n",
            self.p2p_peers_connected.get()
        ));
        
        // Cache metrics
        output.push_str(&format!(
            "# HELP edgeai_cache_hits_total Total cache hits\n\
             # TYPE edgeai_cache_hits_total counter\n\
             edgeai_cache_hits_total {}\n\n",
            self.cache_hits.get()
        ));
        
        output.push_str(&format!(
            "# HELP edgeai_cache_misses_total Total cache misses\n\
             # TYPE edgeai_cache_misses_total counter\n\
             edgeai_cache_misses_total {}\n\n",
            self.cache_misses.get()
        ));
        
        // Uptime
        output.push_str(&format!(
            "# HELP edgeai_uptime_seconds Node uptime in seconds\n\
             # TYPE edgeai_uptime_seconds gauge\n\
             edgeai_uptime_seconds {}\n\n",
            self.uptime_secs()
        ));
        
        // Request duration histogram
        let duration_stats = self.http_request_duration.get_stats();
        output.push_str("# HELP edgeai_http_request_duration_seconds HTTP request latency\n");
        output.push_str("# TYPE edgeai_http_request_duration_seconds histogram\n");
        for (bound, count) in &duration_stats.buckets {
            output.push_str(&format!(
                "edgeai_http_request_duration_seconds_bucket{{le=\"{}\"}} {}\n",
                bound, count
            ));
        }
        output.push_str(&format!(
            "edgeai_http_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
            duration_stats.count
        ));
        output.push_str(&format!(
            "edgeai_http_request_duration_seconds_sum {}\n",
            duration_stats.sum
        ));
        output.push_str(&format!(
            "edgeai_http_request_duration_seconds_count {}\n\n",
            duration_stats.count
        ));
        
        output
    }

    /// Export metrics as JSON
    pub async fn export_json(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            http: HttpMetrics {
                requests_total: self.http_requests_total.get(),
                requests_in_flight: self.http_requests_in_flight.get(),
                errors_total: self.http_errors_total.get(),
                request_duration: self.http_request_duration.get_stats(),
            },
            blockchain: BlockchainMetrics {
                blocks_produced: self.blocks_produced.get(),
                transactions_processed: self.transactions_processed.get(),
                pending_transactions: self.pending_transactions.get(),
                chain_height: self.chain_height.get(),
                active_accounts: self.active_accounts.get(),
            },
            p2p: P2PMetrics {
                peers_connected: self.p2p_peers_connected.get(),
                messages_sent: self.p2p_messages_sent.get(),
                messages_received: self.p2p_messages_received.get(),
                bytes_sent: self.p2p_bytes_sent.get(),
                bytes_received: self.p2p_bytes_received.get(),
            },
            cache: CacheMetrics {
                hits: self.cache_hits.get(),
                misses: self.cache_misses.get(),
                size: self.cache_size.get(),
                hit_rate: {
                    let total = self.cache_hits.get() + self.cache_misses.get();
                    if total > 0 {
                        self.cache_hits.get() as f64 / total as f64
                    } else {
                        0.0
                    }
                },
            },
            uptime_secs: self.uptime_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub http: HttpMetrics,
    pub blockchain: BlockchainMetrics,
    pub p2p: P2PMetrics,
    pub cache: CacheMetrics,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMetrics {
    pub requests_total: u64,
    pub requests_in_flight: u64,
    pub errors_total: u64,
    pub request_duration: HistogramStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainMetrics {
    pub blocks_produced: u64,
    pub transactions_processed: u64,
    pub pending_transactions: u64,
    pub chain_height: u64,
    pub active_accounts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PMetrics {
    pub peers_connected: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub size: u64,
    pub hit_rate: f64,
}

/// Global metrics instance
pub type GlobalMetrics = Arc<Metrics>;

pub fn create_metrics() -> GlobalMetrics {
    Arc::new(Metrics::new())
}
