//! Peer Scoring and Reputation System for EdgeAI Blockchain
//!
//! This module implements a comprehensive peer scoring system that evaluates
//! nodes based on their behavior, reliability, and contribution to the network.
//! It includes blacklisting capabilities for malicious actors and rate limiting
//! to prevent abuse.

#![allow(dead_code)]

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Peer score thresholds
pub mod thresholds {
    /// Minimum score to remain connected
    pub const MIN_SCORE: f64 = -100.0;
    /// Score below which peer is considered suspicious
    pub const SUSPICIOUS_SCORE: f64 = -50.0;
    /// Score above which peer is considered trusted
    pub const TRUSTED_SCORE: f64 = 100.0;
    /// Maximum score a peer can achieve
    pub const MAX_SCORE: f64 = 500.0;
    /// Initial score for new peers
    pub const INITIAL_SCORE: f64 = 0.0;
}

/// Score adjustments for various behaviors
pub mod adjustments {
    /// Valid block propagated
    pub const VALID_BLOCK: f64 = 10.0;
    /// Invalid block propagated
    pub const INVALID_BLOCK: f64 = -50.0;
    /// Valid transaction propagated
    pub const VALID_TRANSACTION: f64 = 1.0;
    /// Invalid transaction propagated
    pub const INVALID_TRANSACTION: f64 = -10.0;
    /// Successful ping response
    pub const PING_SUCCESS: f64 = 0.1;
    /// Failed ping response
    pub const PING_FAILURE: f64 = -5.0;
    /// Timely block delivery
    pub const TIMELY_DELIVERY: f64 = 2.0;
    /// Late block delivery
    pub const LATE_DELIVERY: f64 = -1.0;
    /// Duplicate message sent
    pub const DUPLICATE_MESSAGE: f64 = -2.0;
    /// Spam behavior detected
    pub const SPAM_DETECTED: f64 = -20.0;
    /// Protocol violation
    pub const PROTOCOL_VIOLATION: f64 = -30.0;
    /// Double signing detected
    pub const DOUBLE_SIGN: f64 = -100.0;
    /// Successful sync contribution
    pub const SYNC_CONTRIBUTION: f64 = 5.0;
    /// Data contribution
    pub const DATA_CONTRIBUTION: f64 = 3.0;
}

/// Reasons for blacklisting a peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BlacklistReason {
    /// Score dropped below minimum threshold
    LowScore,
    /// Double signing detected
    DoubleSigning,
    /// Spam behavior
    Spamming,
    /// Protocol violation
    ProtocolViolation,
    /// Manual ban by operator
    ManualBan,
    /// Sybil attack suspected
    SybilAttack,
    /// Eclipse attack attempted
    EclipseAttack,
    /// Invalid data submitted
    InvalidData,
}

/// Blacklist entry for a banned peer
#[derive(Debug, Clone)]
pub struct BlacklistEntry {
    /// Peer ID
    pub peer_id: String,
    /// IP address (if known)
    pub ip_address: Option<IpAddr>,
    /// Reason for blacklisting
    pub reason: BlacklistReason,
    /// When the peer was blacklisted
    pub blacklisted_at: Instant,
    /// Duration of the ban (None = permanent)
    pub ban_duration: Option<Duration>,
    /// Number of times this peer has been banned
    pub ban_count: u32,
}

impl BlacklistEntry {
    pub fn new(peer_id: String, reason: BlacklistReason, duration: Option<Duration>) -> Self {
        Self {
            peer_id,
            ip_address: None,
            reason,
            blacklisted_at: Instant::now(),
            ban_duration: duration,
            ban_count: 1,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(duration) = self.ban_duration {
            self.blacklisted_at.elapsed() > duration
        } else {
            false // Permanent ban never expires
        }
    }
}

/// Peer behavior record for scoring
#[derive(Debug, Clone)]
pub struct PeerBehavior {
    /// Peer ID
    pub peer_id: String,
    /// Current score
    pub score: f64,
    /// Number of valid blocks propagated
    pub valid_blocks: u64,
    /// Number of invalid blocks propagated
    pub invalid_blocks: u64,
    /// Number of valid transactions propagated
    pub valid_transactions: u64,
    /// Number of invalid transactions propagated
    pub invalid_transactions: u64,
    /// Number of successful pings
    pub successful_pings: u64,
    /// Number of failed pings
    pub failed_pings: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Number of duplicate messages sent
    pub duplicate_messages: u64,
    /// Number of protocol violations
    pub protocol_violations: u64,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// When the peer was first seen
    pub first_seen: Instant,
    /// Recent message timestamps for rate limiting
    pub recent_messages: VecDeque<Instant>,
}

impl PeerBehavior {
    pub fn new(peer_id: String) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            score: thresholds::INITIAL_SCORE,
            valid_blocks: 0,
            invalid_blocks: 0,
            valid_transactions: 0,
            invalid_transactions: 0,
            successful_pings: 0,
            failed_pings: 0,
            avg_latency_ms: 0.0,
            duplicate_messages: 0,
            protocol_violations: 0,
            last_activity: now,
            first_seen: now,
            recent_messages: VecDeque::with_capacity(100),
        }
    }

    /// Apply a score adjustment, clamping to valid range
    pub fn adjust_score(&mut self, adjustment: f64) {
        self.score = (self.score + adjustment)
            .max(thresholds::MIN_SCORE - 50.0) // Allow going slightly below for blacklisting
            .min(thresholds::MAX_SCORE);
        self.last_activity = Instant::now();
    }

    /// Check if peer is trusted
    pub fn is_trusted(&self) -> bool {
        self.score >= thresholds::TRUSTED_SCORE
    }

    /// Check if peer is suspicious
    pub fn is_suspicious(&self) -> bool {
        self.score <= thresholds::SUSPICIOUS_SCORE
    }

    /// Check if peer should be disconnected
    pub fn should_disconnect(&self) -> bool {
        self.score < thresholds::MIN_SCORE
    }

    /// Calculate reliability ratio
    pub fn reliability(&self) -> f64 {
        let total_pings = self.successful_pings + self.failed_pings;
        if total_pings == 0 {
            return 1.0;
        }
        self.successful_pings as f64 / total_pings as f64
    }

    /// Calculate block validity ratio
    pub fn block_validity(&self) -> f64 {
        let total_blocks = self.valid_blocks + self.invalid_blocks;
        if total_blocks == 0 {
            return 1.0;
        }
        self.valid_blocks as f64 / total_blocks as f64
    }
}

/// Rate limiter for preventing spam and DDoS
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum messages per second
    pub max_messages_per_second: u32,
    /// Maximum messages per minute
    pub max_messages_per_minute: u32,
    /// Window size for rate calculation
    pub window_size: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            max_messages_per_second: 100,
            max_messages_per_minute: 1000,
            window_size: Duration::from_secs(60),
        }
    }
}

impl RateLimiter {
    /// Check if a peer is rate limited based on their recent messages
    pub fn is_rate_limited(&self, recent_messages: &VecDeque<Instant>) -> bool {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);
        let one_minute_ago = now - Duration::from_secs(60);

        let messages_last_second = recent_messages
            .iter()
            .filter(|t| **t > one_second_ago)
            .count() as u32;

        let messages_last_minute = recent_messages
            .iter()
            .filter(|t| **t > one_minute_ago)
            .count() as u32;

        messages_last_second > self.max_messages_per_second
            || messages_last_minute > self.max_messages_per_minute
    }
}

/// Message deduplication cache
#[derive(Debug)]
pub struct MessageCache {
    /// Seen message hashes with timestamps
    seen_messages: HashMap<String, Instant>,
    /// Maximum cache size
    max_size: usize,
    /// TTL for cached messages
    ttl: Duration,
}

impl MessageCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            seen_messages: HashMap::with_capacity(max_size),
            max_size,
            ttl,
        }
    }

    /// Check if a message has been seen before
    pub fn is_duplicate(&mut self, message_hash: &str) -> bool {
        self.cleanup_expired();

        if self.seen_messages.contains_key(message_hash) {
            return true;
        }

        // Add to cache
        if self.seen_messages.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest_key) = self
                .seen_messages
                .iter()
                .min_by_key(|(_, v)| *v)
                .map(|(k, _)| k.clone())
            {
                self.seen_messages.remove(&oldest_key);
            }
        }

        self.seen_messages
            .insert(message_hash.to_string(), Instant::now());
        false
    }

    /// Remove expired entries
    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.seen_messages
            .retain(|_, timestamp| now.duration_since(*timestamp) < self.ttl);
    }
}

/// Peer Scoring Manager
pub struct PeerScoringManager {
    /// Peer behavior records
    behaviors: Arc<RwLock<HashMap<String, PeerBehavior>>>,
    /// Blacklisted peers
    blacklist: Arc<RwLock<HashMap<String, BlacklistEntry>>>,
    /// Blacklisted IP addresses
    ip_blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Rate limiter configuration
    rate_limiter: RateLimiter,
    /// Message deduplication cache
    message_cache: Arc<RwLock<MessageCache>>,
}

impl PeerScoringManager {
    pub fn new() -> Self {
        Self {
            behaviors: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            ip_blacklist: Arc::new(RwLock::new(HashSet::new())),
            rate_limiter: RateLimiter::default(),
            message_cache: Arc::new(RwLock::new(MessageCache::new(
                10000,
                Duration::from_secs(300),
            ))),
        }
    }

    /// Register a new peer
    pub async fn register_peer(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if !behaviors.contains_key(peer_id) {
            behaviors.insert(peer_id.to_string(), PeerBehavior::new(peer_id.to_string()));
            debug!(
                "Registered new peer for scoring: {}",
                &peer_id[..8.min(peer_id.len())]
            );
        }
    }

    /// Remove a peer from scoring
    pub async fn unregister_peer(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        behaviors.remove(peer_id);
    }

    /// Get peer score
    pub async fn get_score(&self, peer_id: &str) -> Option<f64> {
        let behaviors = self.behaviors.read().await;
        behaviors.get(peer_id).map(|b| b.score)
    }

    /// Get peer behavior record
    pub async fn get_behavior(&self, peer_id: &str) -> Option<PeerBehavior> {
        let behaviors = self.behaviors.read().await;
        behaviors.get(peer_id).cloned()
    }

    /// Record a valid block from peer
    pub async fn record_valid_block(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.valid_blocks += 1;
            behavior.adjust_score(adjustments::VALID_BLOCK);
        }
    }

    /// Record an invalid block from peer
    pub async fn record_invalid_block(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.invalid_blocks += 1;
            behavior.adjust_score(adjustments::INVALID_BLOCK);

            if behavior.should_disconnect() {
                drop(behaviors);
                self.blacklist_peer(
                    peer_id,
                    BlacklistReason::InvalidData,
                    Some(Duration::from_secs(3600)),
                )
                .await;
            }
        }
    }

    /// Record a valid transaction from peer
    pub async fn record_valid_transaction(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.valid_transactions += 1;
            behavior.adjust_score(adjustments::VALID_TRANSACTION);
        }
    }

    /// Record an invalid transaction from peer
    pub async fn record_invalid_transaction(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.invalid_transactions += 1;
            behavior.adjust_score(adjustments::INVALID_TRANSACTION);
        }
    }

    /// Record a successful ping
    pub async fn record_ping_success(&self, peer_id: &str, latency_ms: u64) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.successful_pings += 1;
            behavior.adjust_score(adjustments::PING_SUCCESS);

            // Update average latency
            let total_pings = behavior.successful_pings as f64;
            behavior.avg_latency_ms =
                (behavior.avg_latency_ms * (total_pings - 1.0) + latency_ms as f64) / total_pings;
        }
    }

    /// Record a failed ping
    pub async fn record_ping_failure(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.failed_pings += 1;
            behavior.adjust_score(adjustments::PING_FAILURE);
        }
    }

    /// Record a duplicate message
    pub async fn record_duplicate_message(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.duplicate_messages += 1;
            behavior.adjust_score(adjustments::DUPLICATE_MESSAGE);
        }
    }

    /// Record a protocol violation
    pub async fn record_protocol_violation(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.protocol_violations += 1;
            behavior.adjust_score(adjustments::PROTOCOL_VIOLATION);

            if behavior.protocol_violations >= 5 {
                drop(behaviors);
                self.blacklist_peer(
                    peer_id,
                    BlacklistReason::ProtocolViolation,
                    Some(Duration::from_secs(86400)),
                )
                .await;
            }
        }
    }

    /// Record double signing (severe offense)
    pub async fn record_double_sign(&self, peer_id: &str) {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            behavior.adjust_score(adjustments::DOUBLE_SIGN);
        }
        drop(behaviors);

        // Permanent ban for double signing
        self.blacklist_peer(peer_id, BlacklistReason::DoubleSigning, None)
            .await;
        warn!(
            "Double signing detected from peer {}, permanently banned",
            &peer_id[..8.min(peer_id.len())]
        );
    }

    /// Record a message for rate limiting
    pub async fn record_message(&self, peer_id: &str) -> bool {
        let mut behaviors = self.behaviors.write().await;
        if let Some(behavior) = behaviors.get_mut(peer_id) {
            let now = Instant::now();

            // Clean old messages
            while behavior
                .recent_messages
                .front()
                .map(|t| now.duration_since(*t) > Duration::from_secs(60))
                .unwrap_or(false)
            {
                behavior.recent_messages.pop_front();
            }

            // Check rate limit
            if self.rate_limiter.is_rate_limited(&behavior.recent_messages) {
                behavior.adjust_score(adjustments::SPAM_DETECTED);

                if behavior.score < thresholds::SUSPICIOUS_SCORE {
                    drop(behaviors);
                    self.blacklist_peer(
                        peer_id,
                        BlacklistReason::Spamming,
                        Some(Duration::from_secs(3600)),
                    )
                    .await;
                }
                return false; // Rate limited
            }

            behavior.recent_messages.push_back(now);
            behavior.last_activity = now;
        }
        true // Not rate limited
    }

    /// Check if a message is a duplicate
    pub async fn is_duplicate_message(&self, message_hash: &str) -> bool {
        let mut cache = self.message_cache.write().await;
        cache.is_duplicate(message_hash)
    }

    /// Blacklist a peer
    pub async fn blacklist_peer(
        &self,
        peer_id: &str,
        reason: BlacklistReason,
        duration: Option<Duration>,
    ) {
        let mut blacklist = self.blacklist.write().await;

        if let Some(entry) = blacklist.get_mut(peer_id) {
            entry.ban_count += 1;
            entry.blacklisted_at = Instant::now();
            // Increase ban duration for repeat offenders
            entry.ban_duration = duration.map(|d| d * entry.ban_count);
            info!(
                "Peer {} re-blacklisted (count: {})",
                &peer_id[..8.min(peer_id.len())],
                entry.ban_count
            );
        } else {
            let entry = BlacklistEntry::new(peer_id.to_string(), reason.clone(), duration);
            blacklist.insert(peer_id.to_string(), entry);
            info!(
                "Peer {} blacklisted for {:?}",
                &peer_id[..8.min(peer_id.len())],
                reason
            );
        }
    }

    /// Blacklist an IP address
    pub async fn blacklist_ip(&self, ip: IpAddr) {
        let mut ip_blacklist = self.ip_blacklist.write().await;
        ip_blacklist.insert(ip);
        info!("IP address {} blacklisted", ip);
    }

    /// Check if a peer is blacklisted
    pub async fn is_blacklisted(&self, peer_id: &str) -> bool {
        let blacklist = self.blacklist.read().await;
        if let Some(entry) = blacklist.get(peer_id) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Check if an IP is blacklisted
    pub async fn is_ip_blacklisted(&self, ip: &IpAddr) -> bool {
        let ip_blacklist = self.ip_blacklist.read().await;
        ip_blacklist.contains(ip)
    }

    /// Remove a peer from blacklist
    pub async fn unblacklist_peer(&self, peer_id: &str) {
        let mut blacklist = self.blacklist.write().await;
        blacklist.remove(peer_id);
        info!(
            "Peer {} removed from blacklist",
            &peer_id[..8.min(peer_id.len())]
        );
    }

    /// Clean up expired blacklist entries
    pub async fn cleanup_expired_bans(&self) {
        let mut blacklist = self.blacklist.write().await;
        let expired: Vec<String> = blacklist
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            blacklist.remove(&id);
            debug!("Removed expired ban for peer {}", &id[..8.min(id.len())]);
        }
    }

    /// Get all trusted peers
    pub async fn get_trusted_peers(&self) -> Vec<String> {
        let behaviors = self.behaviors.read().await;
        behaviors
            .iter()
            .filter(|(_, b)| b.is_trusted())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all suspicious peers
    pub async fn get_suspicious_peers(&self) -> Vec<String> {
        let behaviors = self.behaviors.read().await;
        behaviors
            .iter()
            .filter(|(_, b)| b.is_suspicious())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get peers that should be disconnected
    pub async fn get_peers_to_disconnect(&self) -> Vec<String> {
        let behaviors = self.behaviors.read().await;
        behaviors
            .iter()
            .filter(|(_, b)| b.should_disconnect())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get scoring statistics
    pub async fn get_stats(&self) -> ScoringStats {
        let behaviors = self.behaviors.read().await;
        let blacklist = self.blacklist.read().await;

        let total_peers = behaviors.len();
        let trusted_peers = behaviors.values().filter(|b| b.is_trusted()).count();
        let suspicious_peers = behaviors.values().filter(|b| b.is_suspicious()).count();
        let avg_score = if total_peers > 0 {
            behaviors.values().map(|b| b.score).sum::<f64>() / total_peers as f64
        } else {
            0.0
        };

        ScoringStats {
            total_peers,
            trusted_peers,
            suspicious_peers,
            blacklisted_peers: blacklist.len(),
            average_score: avg_score,
        }
    }
}

impl Default for PeerScoringManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringStats {
    pub total_peers: usize,
    pub trusted_peers: usize,
    pub suspicious_peers: usize,
    pub blacklisted_peers: usize,
    pub average_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_scoring() {
        let manager = PeerScoringManager::new();

        manager.register_peer("peer1").await;

        // Record some valid blocks
        for _ in 0..10 {
            manager.record_valid_block("peer1").await;
        }

        let score = manager.get_score("peer1").await.unwrap();
        assert!(score > 0.0);

        // Record invalid blocks
        for _ in 0..5 {
            manager.record_invalid_block("peer1").await;
        }

        let new_score = manager.get_score("peer1").await.unwrap();
        assert!(new_score < score);
    }

    #[tokio::test]
    async fn test_blacklisting() {
        let manager = PeerScoringManager::new();

        manager
            .blacklist_peer(
                "peer1",
                BlacklistReason::Spamming,
                Some(Duration::from_secs(1)),
            )
            .await;
        assert!(manager.is_blacklisted("peer1").await);

        // Wait for ban to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        manager.cleanup_expired_bans().await;
        assert!(!manager.is_blacklisted("peer1").await);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let manager = PeerScoringManager::new();
        manager.register_peer("peer1").await;

        // Should not be rate limited initially
        for _ in 0..50 {
            assert!(manager.record_message("peer1").await);
        }
    }

    #[test]
    fn test_message_cache() {
        let mut cache = MessageCache::new(100, Duration::from_secs(60));

        assert!(!cache.is_duplicate("hash1"));
        assert!(cache.is_duplicate("hash1")); // Now it's a duplicate
        assert!(!cache.is_duplicate("hash2")); // Different hash
    }
}
