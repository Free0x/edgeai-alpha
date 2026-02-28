//! P2P Networking module for EdgeAI Blockchain
//!
//! This module provides peer-to-peer networking capabilities including
//! peer discovery, message propagation, and network management.
//!
//! NOTE: Some structures are prepared for future features and may not
//! be fully integrated yet.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::blockchain::{Block, Transaction};

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub node_type: NodeType,
    pub version: String,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
    pub latency_ms: u64,
    pub block_height: u64,
}

impl Peer {
    pub fn new(id: String, address: String, port: u16, node_type: NodeType) -> Self {
        let now = Utc::now();
        Peer {
            id,
            address,
            port,
            node_type,
            version: "0.1.0".to_string(),
            connected_at: now,
            last_seen: now,
            is_active: true,
            latency_ms: 0,
            block_height: 0,
        }
    }

    pub fn full_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

/// Node types in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    /// Full node with complete blockchain
    FullNode,
    /// Validator node participating in consensus
    Validator,
    /// Light node for edge devices
    LightNode,
    /// Mining node contributing data
    MiningNode,
    /// API gateway node
    Gateway,
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Handshake message
    Handshake {
        node_id: String,
        node_type: NodeType,
        version: String,
        block_height: u64,
    },
    /// Handshake acknowledgment
    HandshakeAck {
        node_id: String,
        accepted: bool,
        reason: Option<String>,
    },
    /// Ping for keepalive
    Ping { timestamp: i64 },
    /// Pong response
    Pong { timestamp: i64 },
    /// Request peers list
    GetPeers,
    /// Peers list response
    Peers { peers: Vec<PeerInfo> },
    /// New block announcement
    NewBlock { block: Block },
    /// New transaction announcement
    NewTransaction { transaction: Transaction },
    /// Request block by index
    GetBlock { index: u64 },
    /// Block response
    BlockResponse { block: Option<Block> },
    /// Request blocks range
    GetBlocks { start_index: u64, end_index: u64 },
    /// Blocks response
    BlocksResponse { blocks: Vec<Block> },
    /// Request transaction by hash
    GetTransaction { hash: String },
    /// Transaction response
    TransactionResponse { transaction: Option<Transaction> },
    /// Sync request
    SyncRequest { from_height: u64 },
    /// Sync response
    SyncResponse { blocks: Vec<Block>, has_more: bool },
    /// Data contribution announcement
    DataContribution {
        data_hash: String,
        contributor: String,
        quality_score: f64,
    },
    /// Validator vote
    ValidatorVote {
        block_hash: String,
        validator: String,
        vote: bool,
        signature: String,
    },
}

/// Simplified peer info for sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub node_type: NodeType,
}

impl From<&Peer> for PeerInfo {
    fn from(peer: &Peer) -> Self {
        PeerInfo {
            id: peer.id.clone(),
            address: peer.address.clone(),
            port: peer.port,
            node_type: peer.node_type.clone(),
        }
    }
}

/// P2P Network Manager
pub struct NetworkManager {
    pub node_id: String,
    pub node_type: NodeType,
    pub listen_port: u16,
    pub peers: Arc<RwLock<HashMap<String, Peer>>>,
    pub max_peers: usize,
    pub message_tx: mpsc::Sender<(String, NetworkMessage)>,
    pub message_rx: Arc<RwLock<mpsc::Receiver<(String, NetworkMessage)>>>,
    pub block_height: Arc<RwLock<u64>>,
}

impl NetworkManager {
    pub fn new(node_id: String, node_type: NodeType, listen_port: u16) -> Self {
        let (tx, rx) = mpsc::channel(1000);

        NetworkManager {
            node_id,
            node_type,
            listen_port,
            peers: Arc::new(RwLock::new(HashMap::new())),
            max_peers: 50,
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(rx)),
            block_height: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a new peer
    pub async fn add_peer(&self, peer: Peer) -> Result<(), String> {
        let mut peers = self.peers.write().await;

        if peers.len() >= self.max_peers {
            return Err("Max peers reached".to_string());
        }

        if peers.contains_key(&peer.id) {
            return Err("Peer already connected".to_string());
        }

        info!(
            "Peer connected: {} ({})",
            &peer.id[..8],
            peer.full_address()
        );
        peers.insert(peer.id.clone(), peer);

        Ok(())
    }

    /// Remove a peer
    pub async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if peers.remove(peer_id).is_some() {
            info!("Peer disconnected: {}", &peer_id[..8]);
        }
    }

    /// Get peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Option<Peer> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    /// Get all active peers
    pub async fn get_active_peers(&self) -> Vec<Peer> {
        let peers = self.peers.read().await;
        peers.values().filter(|p| p.is_active).cloned().collect()
    }

    /// Get peer count
    pub async fn peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }

    /// Broadcast message to all peers
    pub async fn broadcast(&self, message: NetworkMessage) {
        let peers = self.peers.read().await;
        for peer_id in peers.keys() {
            if let Err(e) = self
                .message_tx
                .send((peer_id.clone(), message.clone()))
                .await
            {
                warn!("Failed to queue message for {}: {}", &peer_id[..8], e);
            }
        }
    }

    /// Send message to specific peer
    pub async fn send_to_peer(&self, peer_id: &str, message: NetworkMessage) -> Result<(), String> {
        let peers = self.peers.read().await;
        if !peers.contains_key(peer_id) {
            return Err("Peer not found".to_string());
        }

        self.message_tx
            .send((peer_id.to_string(), message))
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle incoming message
    pub async fn handle_message(
        &self,
        from_peer: &str,
        message: NetworkMessage,
    ) -> Option<NetworkMessage> {
        match message {
            NetworkMessage::Handshake {
                node_id,
                node_type,
                version: _,
                block_height,
            } => {
                debug!(
                    "Handshake from {} (type: {:?}, height: {})",
                    &node_id[..8],
                    node_type,
                    block_height
                );

                // Update peer info
                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(from_peer) {
                    peer.block_height = block_height;
                    peer.last_seen = Utc::now();
                }

                Some(NetworkMessage::HandshakeAck {
                    node_id: self.node_id.clone(),
                    accepted: true,
                    reason: None,
                })
            }

            NetworkMessage::Ping { timestamp } => Some(NetworkMessage::Pong { timestamp }),

            NetworkMessage::Pong { timestamp } => {
                let latency = Utc::now().timestamp_millis() - timestamp;
                let mut peers = self.peers.write().await;
                if let Some(peer) = peers.get_mut(from_peer) {
                    peer.latency_ms = latency as u64;
                    peer.last_seen = Utc::now();
                }
                None
            }

            NetworkMessage::GetPeers => {
                let peers = self.peers.read().await;
                let peer_infos: Vec<PeerInfo> = peers
                    .values()
                    .filter(|p| p.is_active)
                    .map(|p| p.into())
                    .collect();

                Some(NetworkMessage::Peers { peers: peer_infos })
            }

            NetworkMessage::NewBlock { block } => {
                info!(
                    "Received new block #{} from {}",
                    block.index,
                    &from_peer[..8]
                );
                // Block will be processed by the node
                None
            }

            NetworkMessage::NewTransaction { transaction } => {
                debug!(
                    "Received new transaction {} from {}",
                    &transaction.hash[..8],
                    &from_peer[..8]
                );
                // Transaction will be processed by the node
                None
            }

            NetworkMessage::DataContribution {
                data_hash,
                contributor,
                quality_score,
            } => {
                info!(
                    "Data contribution: {} from {} (quality: {:.2})",
                    &data_hash[..8],
                    &contributor[..8],
                    quality_score
                );
                None
            }

            _ => None,
        }
    }

    /// Update peer's last seen timestamp
    pub async fn update_peer_activity(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.last_seen = Utc::now();
        }
    }

    /// Check and remove inactive peers
    pub async fn cleanup_inactive_peers(&self, timeout_seconds: i64) {
        let mut peers = self.peers.write().await;
        let now = Utc::now();

        let inactive: Vec<String> = peers
            .iter()
            .filter(|(_, peer)| (now - peer.last_seen).num_seconds() > timeout_seconds)
            .map(|(id, _)| id.clone())
            .collect();

        for id in inactive {
            info!("Removing inactive peer: {}", &id[..8]);
            peers.remove(&id);
        }
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> NetworkStats {
        let peers = self.peers.read().await;
        let block_height = *self.block_height.read().await;

        let active_peers = peers.values().filter(|p| p.is_active).count();
        let validators = peers
            .values()
            .filter(|p| p.node_type == NodeType::Validator)
            .count();
        let mining_nodes = peers
            .values()
            .filter(|p| p.node_type == NodeType::MiningNode)
            .count();

        let avg_latency = if active_peers > 0 {
            peers
                .values()
                .filter(|p| p.is_active)
                .map(|p| p.latency_ms)
                .sum::<u64>()
                / active_peers as u64
        } else {
            0
        };

        NetworkStats {
            node_id: self.node_id.clone(),
            node_type: self.node_type.clone(),
            listen_port: self.listen_port,
            total_peers: peers.len() as u64,
            active_peers: active_peers as u64,
            validators: validators as u64,
            mining_nodes: mining_nodes as u64,
            average_latency_ms: avg_latency,
            block_height,
        }
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub node_id: String,
    pub node_type: NodeType,
    pub listen_port: u16,
    pub total_peers: u64,
    pub active_peers: u64,
    pub validators: u64,
    pub mining_nodes: u64,
    pub average_latency_ms: u64,
    pub block_height: u64,
}

/// Discovery service for finding peers
pub struct DiscoveryService {
    pub bootstrap_nodes: Vec<String>,
    pub discovered_peers: Arc<RwLock<Vec<PeerInfo>>>,
}

impl DiscoveryService {
    pub fn new(bootstrap_nodes: Vec<String>) -> Self {
        DiscoveryService {
            bootstrap_nodes,
            discovered_peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add discovered peer
    pub async fn add_discovered(&self, peer: PeerInfo) {
        let mut peers = self.discovered_peers.write().await;
        if !peers.iter().any(|p| p.id == peer.id) {
            peers.push(peer);
        }
    }

    /// Get discovered peers
    pub async fn get_discovered(&self) -> Vec<PeerInfo> {
        let peers = self.discovered_peers.read().await;
        peers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager() {
        let manager = NetworkManager::new("node1".to_string(), NodeType::FullNode, 8333);

        let peer = Peer::new(
            "peer1".to_string(),
            "127.0.0.1".to_string(),
            8334,
            NodeType::FullNode,
        );

        manager.add_peer(peer).await.unwrap();
        assert_eq!(manager.peer_count().await, 1);

        manager.remove_peer("peer1").await;
        assert_eq!(manager.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_message_handling() {
        let manager = NetworkManager::new("node1".to_string(), NodeType::FullNode, 8333);

        let response = manager
            .handle_message(
                "peer1",
                NetworkMessage::Ping {
                    timestamp: Utc::now().timestamp_millis(),
                },
            )
            .await;

        assert!(matches!(response, Some(NetworkMessage::Pong { .. })));
    }
}
