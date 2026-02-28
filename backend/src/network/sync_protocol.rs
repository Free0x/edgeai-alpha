//! Block Synchronization Protocol for EdgeAI Blockchain
//!
//! This module implements a robust block synchronization protocol that handles
//! initial sync, catch-up sync, and real-time block propagation with proper
//! validation and peer selection.

#![allow(dead_code)]

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};

use crate::blockchain::Block;

/// Sync state machine states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    /// Node is idle, fully synced
    Idle,
    /// Discovering peers and their chain heights
    Discovering,
    /// Downloading block headers
    DownloadingHeaders,
    /// Downloading block bodies
    DownloadingBlocks,
    /// Validating downloaded blocks
    Validating,
    /// Applying validated blocks to chain
    Applying,
    /// Sync completed
    Completed,
    /// Sync failed
    Failed(String),
}

/// Sync request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Request chain height from peer
    GetHeight,
    /// Request block headers in range
    GetHeaders { start: u64, count: u64 },
    /// Request specific blocks by hash
    GetBlocks { hashes: Vec<String> },
    /// Request blocks in range
    GetBlockRange { start: u64, end: u64 },
    /// Request specific block by height
    GetBlockByHeight { height: u64 },
}

/// Sync response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    /// Chain height response
    Height { height: u64, best_hash: String },
    /// Block headers response
    Headers { headers: Vec<BlockHeader> },
    /// Blocks response
    Blocks { blocks: Vec<Block> },
    /// Block not found
    NotFound { requested: String },
    /// Error response
    Error { message: String },
}

/// Simplified block header for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub index: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub merkle_root: String,
}

impl From<&Block> for BlockHeader {
    fn from(block: &Block) -> Self {
        Self {
            index: block.index,
            hash: block.hash.clone(),
            previous_hash: block.header.previous_hash.clone(),
            timestamp: block.header.timestamp.timestamp(),
            merkle_root: block.header.merkle_root.clone(),
        }
    }
}

/// Peer sync info
#[derive(Debug, Clone)]
pub struct PeerSyncInfo {
    pub peer_id: String,
    pub height: u64,
    pub best_hash: String,
    pub last_updated: Instant,
    pub sync_speed: f64, // blocks per second
    pub failed_requests: u32,
    pub is_syncing: bool,
}

impl PeerSyncInfo {
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            height: 0,
            best_hash: String::new(),
            last_updated: Instant::now(),
            sync_speed: 0.0,
            failed_requests: 0,
            is_syncing: false,
        }
    }

    /// Calculate peer quality score for sync selection
    pub fn quality_score(&self) -> f64 {
        let freshness = 1.0 / (1.0 + self.last_updated.elapsed().as_secs() as f64 / 60.0);
        let reliability = 1.0 / (1.0 + self.failed_requests as f64);
        let speed = self.sync_speed.min(100.0) / 100.0;

        freshness * 0.3 + reliability * 0.4 + speed * 0.3
    }
}

/// Block download task
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub start_height: u64,
    pub end_height: u64,
    pub assigned_peer: Option<String>,
    pub started_at: Option<Instant>,
    pub retries: u32,
    pub max_retries: u32,
}

impl DownloadTask {
    pub fn new(start: u64, end: u64) -> Self {
        Self {
            start_height: start,
            end_height: end,
            assigned_peer: None,
            started_at: None,
            retries: 0,
            max_retries: 3,
        }
    }

    pub fn block_count(&self) -> u64 {
        self.end_height - self.start_height + 1
    }

    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.started_at
            .map(|t| t.elapsed() > timeout)
            .unwrap_or(false)
    }
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum blocks per request
    pub max_blocks_per_request: u64,
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Minimum peers required to start sync
    pub min_peers_for_sync: usize,
    /// Maximum retries per task
    pub max_retries: u32,
    /// Batch size for block validation
    pub validation_batch_size: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_blocks_per_request: 100,
            max_concurrent_downloads: 4,
            request_timeout: Duration::from_secs(30),
            min_peers_for_sync: 1,
            max_retries: 3,
            validation_batch_size: 50,
        }
    }
}

/// Sync progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub state: SyncState,
    pub current_height: u64,
    pub target_height: u64,
    pub downloaded_blocks: u64,
    pub validated_blocks: u64,
    pub applied_blocks: u64,
    pub peers_syncing: usize,
    pub download_speed: f64, // blocks per second
    pub eta_seconds: Option<u64>,
    pub started_at: Option<i64>,
}

impl SyncProgress {
    pub fn new() -> Self {
        Self {
            state: SyncState::Idle,
            current_height: 0,
            target_height: 0,
            downloaded_blocks: 0,
            validated_blocks: 0,
            applied_blocks: 0,
            peers_syncing: 0,
            download_speed: 0.0,
            eta_seconds: None,
            started_at: None,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.target_height == 0 {
            return 100.0;
        }
        (self.current_height as f64 / self.target_height as f64 * 100.0).min(100.0)
    }
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Block Sync Manager
pub struct SyncManager {
    /// Current sync state
    state: Arc<RwLock<SyncState>>,
    /// Sync configuration
    config: SyncConfig,
    /// Peer sync information
    peers: Arc<RwLock<HashMap<String, PeerSyncInfo>>>,
    /// Pending download tasks
    pending_tasks: Arc<RwLock<VecDeque<DownloadTask>>>,
    /// Active download tasks
    active_tasks: Arc<RwLock<HashMap<String, DownloadTask>>>,
    /// Downloaded blocks waiting for validation
    downloaded_blocks: Arc<RwLock<HashMap<u64, Block>>>,
    /// Validated blocks waiting for application
    validated_blocks: Arc<RwLock<VecDeque<Block>>>,
    /// Current chain height
    current_height: Arc<RwLock<u64>>,
    /// Target chain height
    target_height: Arc<RwLock<u64>>,
    /// Semaphore for concurrent downloads
    download_semaphore: Arc<Semaphore>,
    /// Sync progress
    progress: Arc<RwLock<SyncProgress>>,
    /// Request sender channel
    request_tx: mpsc::Sender<(String, SyncRequest)>,
    /// Response receiver channel
    response_rx: Arc<RwLock<mpsc::Receiver<(String, SyncResponse)>>>,
}

impl SyncManager {
    pub fn new(
        config: SyncConfig,
    ) -> (
        Self,
        mpsc::Receiver<(String, SyncRequest)>,
        mpsc::Sender<(String, SyncResponse)>,
    ) {
        let (request_tx, request_rx) = mpsc::channel(100);
        let (response_tx, response_rx) = mpsc::channel(100);

        let manager = Self {
            state: Arc::new(RwLock::new(SyncState::Idle)),
            config: config.clone(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            pending_tasks: Arc::new(RwLock::new(VecDeque::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            downloaded_blocks: Arc::new(RwLock::new(HashMap::new())),
            validated_blocks: Arc::new(RwLock::new(VecDeque::new())),
            current_height: Arc::new(RwLock::new(0)),
            target_height: Arc::new(RwLock::new(0)),
            download_semaphore: Arc::new(Semaphore::new(config.max_concurrent_downloads)),
            progress: Arc::new(RwLock::new(SyncProgress::new())),
            request_tx,
            response_rx: Arc::new(RwLock::new(response_rx)),
        };

        (manager, request_rx, response_tx)
    }

    /// Set current chain height
    pub async fn set_current_height(&self, height: u64) {
        *self.current_height.write().await = height;
        let mut progress = self.progress.write().await;
        progress.current_height = height;
    }

    /// Register a peer for sync
    pub async fn register_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if !peers.contains_key(peer_id) {
            peers.insert(peer_id.to_string(), PeerSyncInfo::new(peer_id.to_string()));
            debug!(
                "Registered peer for sync: {}",
                &peer_id[..8.min(peer_id.len())]
            );
        }
    }

    /// Update peer height information
    pub async fn update_peer_height(&self, peer_id: &str, height: u64, best_hash: String) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.height = height;
            peer.best_hash = best_hash;
            peer.last_updated = Instant::now();
        }

        // Update target height if this peer is ahead
        let mut target = self.target_height.write().await;
        if height > *target {
            *target = height;
            let mut progress = self.progress.write().await;
            progress.target_height = height;
        }
    }

    /// Remove a peer from sync
    pub async fn unregister_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);

        // Reassign any tasks from this peer
        let mut active = self.active_tasks.write().await;
        let mut pending = self.pending_tasks.write().await;

        let tasks_to_reassign: Vec<DownloadTask> = active
            .iter()
            .filter(|(_, task)| task.assigned_peer.as_deref() == Some(peer_id))
            .map(|(_, task)| {
                let mut t = task.clone();
                t.assigned_peer = None;
                t.started_at = None;
                t.retries += 1;
                t
            })
            .collect();

        for task in tasks_to_reassign {
            active.remove(&format!("{}-{}", task.start_height, task.end_height));
            if task.retries < task.max_retries {
                pending.push_back(task);
            }
        }
    }

    /// Get best peers for sync (sorted by quality)
    pub async fn get_best_peers(&self, count: usize) -> Vec<String> {
        let peers = self.peers.read().await;
        let current_height = *self.current_height.read().await;

        let mut eligible: Vec<_> = peers
            .iter()
            .filter(|(_, p)| p.height > current_height && !p.is_syncing)
            .collect();

        eligible
            .sort_by(|(_, a), (_, b)| b.quality_score().partial_cmp(&a.quality_score()).unwrap());

        eligible
            .into_iter()
            .take(count)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check if sync is needed
    pub async fn needs_sync(&self) -> bool {
        let current = *self.current_height.read().await;
        let target = *self.target_height.read().await;
        target > current
    }

    /// Start synchronization
    pub async fn start_sync(&self) -> Result<(), String> {
        let peers = self.peers.read().await;
        if peers.len() < self.config.min_peers_for_sync {
            return Err(format!(
                "Not enough peers for sync (have {}, need {})",
                peers.len(),
                self.config.min_peers_for_sync
            ));
        }
        drop(peers);

        let mut state = self.state.write().await;
        *state = SyncState::Discovering;

        let mut progress = self.progress.write().await;
        progress.state = SyncState::Discovering;
        progress.started_at = Some(chrono::Utc::now().timestamp());

        info!("Starting block synchronization");

        // Request heights from all peers
        let peers = self.peers.read().await;
        for peer_id in peers.keys() {
            let _ = self
                .request_tx
                .send((peer_id.clone(), SyncRequest::GetHeight))
                .await;
        }

        Ok(())
    }

    /// Create download tasks for missing blocks
    pub async fn create_download_tasks(&self) {
        let current = *self.current_height.read().await;
        let target = *self.target_height.read().await;

        if target <= current {
            return;
        }

        let mut pending = self.pending_tasks.write().await;
        let downloaded = self.downloaded_blocks.read().await;

        let mut height = current + 1;
        while height <= target {
            // Skip already downloaded blocks
            if downloaded.contains_key(&height) {
                height += 1;
                continue;
            }

            let end = (height + self.config.max_blocks_per_request - 1).min(target);
            pending.push_back(DownloadTask::new(height, end));
            height = end + 1;
        }

        info!(
            "Created {} download tasks for blocks {} to {}",
            pending.len(),
            current + 1,
            target
        );
    }

    /// Assign pending tasks to available peers
    pub async fn assign_tasks(&self) {
        let mut pending = self.pending_tasks.write().await;
        let mut active = self.active_tasks.write().await;
        let mut peers = self.peers.write().await;

        let available_peers: Vec<String> = peers
            .iter()
            .filter(|(_, p)| !p.is_syncing && p.failed_requests < 5)
            .map(|(id, _)| id.clone())
            .collect();

        for peer_id in available_peers {
            if pending.is_empty() {
                break;
            }

            if let Some(mut task) = pending.pop_front() {
                task.assigned_peer = Some(peer_id.clone());
                task.started_at = Some(Instant::now());

                let task_key = format!("{}-{}", task.start_height, task.end_height);

                // Mark peer as syncing
                if let Some(peer) = peers.get_mut(&peer_id) {
                    peer.is_syncing = true;
                }

                // Send request
                let _ = self
                    .request_tx
                    .send((
                        peer_id.clone(),
                        SyncRequest::GetBlockRange {
                            start: task.start_height,
                            end: task.end_height,
                        },
                    ))
                    .await;

                active.insert(task_key, task);
            }
        }
    }

    /// Handle sync response from peer
    pub async fn handle_response(&self, peer_id: &str, response: SyncResponse) {
        match response {
            SyncResponse::Height { height, best_hash } => {
                self.update_peer_height(peer_id, height, best_hash).await;
            }

            SyncResponse::Blocks { blocks } => {
                if blocks.is_empty() {
                    return;
                }

                let start = blocks.first().map(|b| b.index).unwrap_or(0);
                let end = blocks.last().map(|b| b.index).unwrap_or(0);
                let task_key = format!("{}-{}", start, end);

                // Store downloaded blocks
                let mut downloaded = self.downloaded_blocks.write().await;
                for block in blocks {
                    downloaded.insert(block.index, block);
                }

                // Complete the task
                let mut active = self.active_tasks.write().await;
                active.remove(&task_key);

                // Update peer status
                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(peer_id) {
                    peer.is_syncing = false;
                    // Update sync speed
                    if let Some(started) = active.get(&task_key).and_then(|t| t.started_at) {
                        let elapsed = started.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            peer.sync_speed = (end - start + 1) as f64 / elapsed;
                        }
                    }
                }

                // Update progress
                let mut progress = self.progress.write().await;
                progress.downloaded_blocks = downloaded.len() as u64;

                debug!(
                    "Downloaded blocks {} to {} from {}",
                    start,
                    end,
                    &peer_id[..8.min(peer_id.len())]
                );
            }

            SyncResponse::NotFound { requested } => {
                warn!(
                    "Block not found: {} (from {})",
                    requested,
                    &peer_id[..8.min(peer_id.len())]
                );

                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(peer_id) {
                    peer.failed_requests += 1;
                    peer.is_syncing = false;
                }
            }

            SyncResponse::Error { message } => {
                error!(
                    "Sync error from {}: {}",
                    &peer_id[..8.min(peer_id.len())],
                    message
                );

                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(peer_id) {
                    peer.failed_requests += 1;
                    peer.is_syncing = false;
                }
            }

            _ => {}
        }
    }

    /// Check for timed out tasks and retry
    pub async fn check_timeouts(&self) {
        let mut active = self.active_tasks.write().await;
        let mut pending = self.pending_tasks.write().await;
        let mut peers = self.peers.write().await;

        let timed_out: Vec<String> = active
            .iter()
            .filter(|(_, task)| task.is_timed_out(self.config.request_timeout))
            .map(|(key, _)| key.clone())
            .collect();

        for key in timed_out {
            if let Some(mut task) = active.remove(&key) {
                // Mark peer as failed
                if let Some(peer_id) = &task.assigned_peer {
                    if let Some(peer) = peers.get_mut(peer_id) {
                        peer.failed_requests += 1;
                        peer.is_syncing = false;
                    }
                }

                // Retry if possible
                task.retries += 1;
                task.assigned_peer = None;
                task.started_at = None;

                if task.retries < task.max_retries {
                    pending.push_back(task);
                    warn!("Task {} timed out, retrying", key);
                } else {
                    error!("Task {} failed after {} retries", key, task.max_retries);
                }
            }
        }
    }

    /// Get blocks ready for validation (in order)
    pub async fn get_blocks_for_validation(&self) -> Vec<Block> {
        let downloaded = self.downloaded_blocks.read().await;
        let current = *self.current_height.read().await;

        let mut blocks = Vec::new();
        let mut height = current + 1;

        while let Some(block) = downloaded.get(&height) {
            blocks.push(block.clone());
            height += 1;

            if blocks.len() >= self.config.validation_batch_size {
                break;
            }
        }

        blocks
    }

    /// Mark blocks as validated
    pub async fn mark_validated(&self, heights: &[u64]) {
        let mut downloaded = self.downloaded_blocks.write().await;
        let mut validated = self.validated_blocks.write().await;

        for height in heights {
            if let Some(block) = downloaded.remove(height) {
                validated.push_back(block);
            }
        }

        let mut progress = self.progress.write().await;
        progress.validated_blocks += heights.len() as u64;
    }

    /// Get validated blocks for application
    pub async fn get_validated_blocks(&self, count: usize) -> Vec<Block> {
        let mut validated = self.validated_blocks.write().await;
        let mut blocks = Vec::new();

        for _ in 0..count {
            if let Some(block) = validated.pop_front() {
                blocks.push(block);
            } else {
                break;
            }
        }

        blocks
    }

    /// Mark blocks as applied to chain
    pub async fn mark_applied(&self, height: u64) {
        *self.current_height.write().await = height;

        let mut progress = self.progress.write().await;
        progress.current_height = height;
        progress.applied_blocks += 1;
    }

    /// Get current sync progress
    pub async fn get_progress(&self) -> SyncProgress {
        self.progress.read().await.clone()
    }

    /// Check if sync is complete
    pub async fn is_complete(&self) -> bool {
        let current = *self.current_height.read().await;
        let target = *self.target_height.read().await;
        let pending = self.pending_tasks.read().await;
        let active = self.active_tasks.read().await;

        current >= target && pending.is_empty() && active.is_empty()
    }

    /// Complete synchronization
    pub async fn complete_sync(&self) {
        let mut state = self.state.write().await;
        *state = SyncState::Completed;

        let mut progress = self.progress.write().await;
        progress.state = SyncState::Completed;

        info!(
            "Block synchronization completed at height {}",
            progress.current_height
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let config = SyncConfig::default();
        let (manager, _, _) = SyncManager::new(config);

        assert!(!manager.needs_sync().await);
    }

    #[tokio::test]
    async fn test_peer_registration() {
        let config = SyncConfig::default();
        let (manager, _, _) = SyncManager::new(config);

        manager.register_peer("peer1").await;
        manager
            .update_peer_height("peer1", 100, "hash".to_string())
            .await;

        let peers = manager.get_best_peers(10).await;
        assert_eq!(peers.len(), 1);
    }

    #[tokio::test]
    async fn test_download_task_creation() {
        let config = SyncConfig::default();
        let (manager, _, _) = SyncManager::new(config);

        manager.set_current_height(0).await;
        manager.register_peer("peer1").await;
        manager
            .update_peer_height("peer1", 500, "hash".to_string())
            .await;

        manager.create_download_tasks().await;

        assert!(manager.needs_sync().await);
    }
}
