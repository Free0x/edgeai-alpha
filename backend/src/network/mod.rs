//! Network module for EdgeAI Blockchain
//!
//! This module provides P2P networking capabilities using libp2p,
//! including peer discovery, message propagation, network management,
//! peer scoring, and block synchronization.

pub mod libp2p_network;
pub mod p2p;
pub mod peer_scoring;
pub mod sync_protocol;

// Core network exports
pub use p2p::{NetworkManager, NodeType};
pub use peer_scoring::{BlacklistReason, PeerScoringManager, ScoringStats};
pub use sync_protocol::{SyncConfig, SyncManager, SyncProgress, SyncState};
