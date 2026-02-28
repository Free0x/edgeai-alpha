//! API Response Cache Module
//!
//! Provides in-memory caching for frequently accessed API endpoints
//! to reduce database queries and improve response times.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache entry with expiration
#[derive(Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl<T: Clone> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            created_at: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Generic cache for API responses
pub struct ApiCache<T> {
    entries: RwLock<HashMap<String, CacheEntry<T>>>,
    default_ttl: Duration,
    max_entries: usize,
}

impl<T: Clone + Send + Sync> ApiCache<T> {
    pub fn new(default_ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            default_ttl: Duration::from_secs(default_ttl_secs),
            max_entries,
        }
    }

    /// Get cached value if exists and not expired
    pub async fn get(&self, key: &str) -> Option<T> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Set cache value with default TTL
    pub async fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl).await;
    }

    /// Set cache value with custom TTL
    pub async fn set_with_ttl(&self, key: String, value: T, ttl: Duration) {
        let mut entries = self.entries.write().await;

        // Evict expired entries if cache is full
        if entries.len() >= self.max_entries {
            entries.retain(|_, v| !v.is_expired());
        }

        // If still full, remove oldest entry
        if entries.len() >= self.max_entries {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }

        entries.insert(key, CacheEntry::new(value, ttl));
    }

    /// Invalidate specific cache entry
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Invalidate entries matching a prefix
    pub async fn invalidate_prefix(&self, prefix: &str) {
        let mut entries = self.entries.write().await;
        entries.retain(|k, _| !k.starts_with(prefix));
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total = entries.len();
        let expired = entries.values().filter(|e| e.is_expired()).count();
        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

/// Cached chain statistics
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedChainStats {
    pub height: u64,
    pub total_transactions: u64,
    pub total_supply: u64,
    pub total_staked: u64,
    pub active_accounts: usize,
    pub data_entries: usize,
    pub difficulty: u64,
    pub last_block_time: i64,
    pub network_entropy: f64,
    pub avg_tx_per_block: f64,
    pub data_throughput: f64,
    pub tps: f64,
    pub validator_power: f64,
}

/// Global cache instance for chain stats
pub type ChainStatsCache = Arc<ApiCache<CachedChainStats>>;

/// Create a new chain stats cache
pub fn create_chain_stats_cache() -> ChainStatsCache {
    Arc::new(ApiCache::new(5, 100)) // 5 second TTL, max 100 entries
}

/// Cache for block data
pub type BlockCache = Arc<ApiCache<String>>; // JSON string

pub fn create_block_cache() -> BlockCache {
    Arc::new(ApiCache::new(60, 1000)) // 60 second TTL, max 1000 entries
}

/// Cache for account data
pub type AccountCache = Arc<ApiCache<String>>; // JSON string

pub fn create_account_cache() -> AccountCache {
    Arc::new(ApiCache::new(10, 5000)) // 10 second TTL, max 5000 entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache: ApiCache<String> = ApiCache::new(60, 100);

        cache.set("key1".to_string(), "value1".to_string()).await;

        let result = cache.get("key1").await;
        assert_eq!(result, Some("value1".to_string()));

        let missing = cache.get("key2").await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache: ApiCache<String> = ApiCache::new(60, 100);

        cache.set("test:1".to_string(), "value1".to_string()).await;
        cache.set("test:2".to_string(), "value2".to_string()).await;
        cache.set("other:1".to_string(), "value3".to_string()).await;

        cache.invalidate_prefix("test:").await;

        assert_eq!(cache.get("test:1").await, None);
        assert_eq!(cache.get("test:2").await, None);
        assert_eq!(cache.get("other:1").await, Some("value3".to_string()));
    }
}
