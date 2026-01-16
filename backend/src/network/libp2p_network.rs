//! EdgeAI Blockchain - libp2p Network Layer
//! 
//! This module implements the P2P networking layer using libp2p,
//! providing node discovery, gossip-based message propagation,
//! and peer management.

#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use libp2p::{
    futures::StreamExt,
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify,
    kad::{self, store::MemoryStore},
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use tokio::sync::mpsc;
use log::{info, debug, warn, error};
use serde::{Deserialize, Serialize};

use crate::blockchain::{Block, Transaction};

/// Gossip topics for EdgeAI network
pub mod topics {
    pub const TRANSACTIONS: &str = "edgeai/tx/1.0.0";
    pub const BLOCKS: &str = "edgeai/block/1.0.0";
    pub const CONTRIBUTIONS: &str = "edgeai/contribution/1.0.0";
}

/// Network events that can be emitted to the application layer
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer connected
    PeerConnected(PeerId),
    /// A peer disconnected
    PeerDisconnected(PeerId),
    /// Received a new transaction from the network
    NewTransaction(Transaction),
    /// Received a new block from the network
    NewBlock(Block),
    /// Received a contribution proof
    NewContribution(ContributionMessage),
    /// Network is ready
    Ready,
}

/// Commands that can be sent to the network layer
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    /// Broadcast a transaction to the network
    BroadcastTransaction(Transaction),
    /// Broadcast a block to the network
    BroadcastBlock(Block),
    /// Broadcast a contribution proof
    BroadcastContribution(ContributionMessage),
    /// Connect to a specific peer
    ConnectPeer(Multiaddr),
    /// Get current peer count
    GetPeerCount,
}

/// Contribution message for gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionMessage {
    pub device_id: String,
    pub data_hash: String,
    pub quality_score: f64,
    pub timestamp: i64,
}

/// Compact block for efficient P2P propagation
/// Contains header info and transaction hashes only, not full transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBlock {
    pub index: u64,
    pub hash: String,
    pub previous_hash: String,
    pub validator: String,
    pub timestamp: String,
    pub tx_count: u32,
    pub tx_hashes: Vec<String>,
}

/// Gossip message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    Transaction(Transaction),
    Block(Block),  // Full block (for backward compatibility)
    CompactBlock(CompactBlock),  // Compact block announcement
    Contribution(ContributionMessage),
}

/// Combined network behaviour for EdgeAI
#[derive(NetworkBehaviour)]
pub struct EdgeAIBehaviour {
    /// Gossipsub for pub/sub messaging
    pub gossipsub: gossipsub::Behaviour,
    /// Kademlia for peer discovery
    pub kademlia: kad::Behaviour<MemoryStore>,
    /// mDNS for local peer discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Identify protocol for peer identification
    pub identify: identify::Behaviour,
}

/// Configuration for the P2P network
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Port to listen on
    pub listen_port: u16,
    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Maximum number of peers
    pub max_peers: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 9000,
            bootstrap_nodes: vec![],
            enable_mdns: true,
            max_peers: 50,
        }
    }
}

/// P2P Network Service
pub struct P2PNetwork {
    /// Local peer ID
    pub local_peer_id: PeerId,
    /// Channel to send events to application
    event_tx: mpsc::Sender<NetworkEvent>,
    /// Channel to receive commands from application
    command_rx: mpsc::Receiver<NetworkCommand>,
    /// Network configuration
    config: NetworkConfig,
}

impl P2PNetwork {
    /// Create a new P2P network instance
    pub fn new(
        config: NetworkConfig,
    ) -> Result<(Self, mpsc::Sender<NetworkCommand>, mpsc::Receiver<NetworkEvent>), Box<dyn std::error::Error>> {
        // Generate a random keypair for this node
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        info!("Local peer ID: {}", local_peer_id);
        
        // Create channels for communication with application
        let (event_tx, event_rx) = mpsc::channel(1000);
        let (command_tx, command_rx) = mpsc::channel(1000);
        
        let network = Self {
            local_peer_id,
            event_tx,
            command_rx,
            config,
        };
        
        Ok((network, command_tx, event_rx))
    }
    
    /// Build the libp2p swarm
    fn build_swarm(&self) -> Result<Swarm<EdgeAIBehaviour>, Box<dyn std::error::Error + Send + Sync>> {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        // Configure gossipsub with optimized settings
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .max_transmit_size(4 * 1024 * 1024) // 4MB max message size (increased from default 1MB)
            .message_id_fn(|message: &gossipsub::Message| {
                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            })
            .build()
            .map_err(|e| format!("Failed to build gossipsub config: {}", e))?;
        
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        ).map_err(|e| format!("Failed to create gossipsub: {}", e))?;
        
        // Configure Kademlia
        let kademlia = kad::Behaviour::new(
            local_peer_id,
            MemoryStore::new(local_peer_id),
        );
        
        // Configure mDNS
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )?;
        
        // Configure Identify
        let identify = identify::Behaviour::new(
            identify::Config::new(
                "/edgeai/1.0.0".to_string(),
                local_key.public(),
            )
        );
        
        // Create the combined behaviour
        let behaviour = EdgeAIBehaviour {
            gossipsub,
            kademlia,
            mdns,
            identify,
        };
        
        // Build the swarm
        let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();
        
        Ok(swarm)
    }
    
    /// Run the P2P network event loop
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut swarm = self.build_swarm()?;
        
        // Subscribe to gossip topics
        let tx_topic = IdentTopic::new(topics::TRANSACTIONS);
        let block_topic = IdentTopic::new(topics::BLOCKS);
        let contribution_topic = IdentTopic::new(topics::CONTRIBUTIONS);
        
        swarm.behaviour_mut().gossipsub.subscribe(&tx_topic)?;
        swarm.behaviour_mut().gossipsub.subscribe(&block_topic)?;
        swarm.behaviour_mut().gossipsub.subscribe(&contribution_topic)?;
        
        // Start listening
        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", self.config.listen_port).parse()?;
        swarm.listen_on(listen_addr)?;
        
        info!("P2P network started on port {}", self.config.listen_port);
        
        // Connect to bootstrap nodes
        for addr_str in &self.config.bootstrap_nodes {
            if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                info!("Connecting to bootstrap node: {}", addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    warn!("Failed to dial bootstrap node {}: {}", addr, e);
                }
            }
        }
        
        // Notify that network is ready
        let _ = self.event_tx.send(NetworkEvent::Ready).await;
        
        // Main event loop
        loop {
            tokio::select! {
                // Handle swarm events
                event = swarm.select_next_some() => {
                    self.handle_swarm_event(&mut swarm, event).await;
                }
                
                // Handle commands from application
                Some(command) = self.command_rx.recv() => {
                    self.handle_command(&mut swarm, command).await;
                }
            }
        }
    }
    
    /// Handle swarm events
    async fn handle_swarm_event(
        &self,
        swarm: &mut Swarm<EdgeAIBehaviour>,
        event: SwarmEvent<EdgeAIBehaviourEvent>,
    ) {
        match event {
            SwarmEvent::Behaviour(EdgeAIBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            })) => {
                debug!("Received gossip message {} from {}", message_id, propagation_source);
                
                // Deserialize and handle the message
                if let Ok(gossip_msg) = serde_json::from_slice::<GossipMessage>(&message.data) {
                    match gossip_msg {
                        GossipMessage::Transaction(tx) => {
                            let _ = self.event_tx.send(NetworkEvent::NewTransaction(tx)).await;
                        }
                        GossipMessage::Block(block) => {
                            let _ = self.event_tx.send(NetworkEvent::NewBlock(block)).await;
                        }
                        GossipMessage::CompactBlock(compact) => {
                            // CompactBlock is for announcement only
                            // Peers can request full block if needed
                            debug!("Received compact block #{} with {} txs", compact.index, compact.tx_count);
                            // For now, we just log it - full block sync would be implemented separately
                        }
                        GossipMessage::Contribution(contrib) => {
                            let _ = self.event_tx.send(NetworkEvent::NewContribution(contrib)).await;
                        }
                    }
                }
            }
            
            SwarmEvent::Behaviour(EdgeAIBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, addr) in peers {
                    info!("mDNS discovered peer: {} at {}", peer_id, addr);
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            
            SwarmEvent::Behaviour(EdgeAIBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _addr) in peers {
                    debug!("mDNS peer expired: {}", peer_id);
                    swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                }
            }
            
            SwarmEvent::Behaviour(EdgeAIBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            })) => {
                info!("Identified peer {}: {} ({})", peer_id, info.protocol_version, info.agent_version);
                
                // Add peer addresses to Kademlia
                for addr in info.listen_addrs {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connection established with peer: {}", peer_id);
                let _ = self.event_tx.send(NetworkEvent::PeerConnected(peer_id)).await;
            }
            
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Connection closed with peer: {}", peer_id);
                let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(peer_id)).await;
            }
            
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            
            _ => {}
        }
    }
    
    /// Handle commands from application
    async fn handle_command(&self, swarm: &mut Swarm<EdgeAIBehaviour>, command: NetworkCommand) {
        match command {
            NetworkCommand::BroadcastTransaction(tx) => {
                let msg = GossipMessage::Transaction(tx);
                if let Ok(data) = serde_json::to_vec(&msg) {
                    let topic = IdentTopic::new(topics::TRANSACTIONS);
                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                        warn!("Failed to broadcast transaction: {}", e);
                    }
                }
            }
            
            NetworkCommand::BroadcastBlock(block) => {
                // Create a compact block announcement (header + tx hashes only)
                // Full block can be requested by peers if needed
                let compact_block = CompactBlock {
                    index: block.index,
                    hash: block.hash.clone(),
                    previous_hash: block.header.previous_hash.clone(),
                    validator: block.validator.clone(),
                    timestamp: block.header.timestamp.to_string(),
                    tx_count: block.transactions.len() as u32,
                    tx_hashes: block.transactions.iter().map(|tx| tx.hash.clone()).collect(),
                };
                let msg = GossipMessage::CompactBlock(compact_block);
                if let Ok(data) = serde_json::to_vec(&msg) {
                    let topic = IdentTopic::new(topics::BLOCKS);
                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                        warn!("Failed to broadcast block: {}", e);
                    } else {
                        debug!("Broadcast compact block #{} ({} tx hashes)", block.index, block.transactions.len());
                    }
                }
            }
            
            NetworkCommand::BroadcastContribution(contrib) => {
                let msg = GossipMessage::Contribution(contrib);
                if let Ok(data) = serde_json::to_vec(&msg) {
                    let topic = IdentTopic::new(topics::CONTRIBUTIONS);
                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                        warn!("Failed to broadcast contribution: {}", e);
                    }
                }
            }
            
            NetworkCommand::ConnectPeer(addr) => {
                info!("Connecting to peer: {}", addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    warn!("Failed to dial peer {}: {}", addr, e);
                }
            }
            
            NetworkCommand::GetPeerCount => {
                let count = swarm.connected_peers().count();
                debug!("Current peer count: {}", count);
            }
        }
    }
}

/// Helper function to create and start the P2P network
pub async fn start_p2p_network(
    config: NetworkConfig,
) -> Result<(mpsc::Sender<NetworkCommand>, mpsc::Receiver<NetworkEvent>), Box<dyn std::error::Error>> {
    let (network, command_tx, event_rx) = P2PNetwork::new(config)?;
    
    // Spawn the network event loop
    tokio::spawn(async move {
        if let Err(e) = network.run().await {
            error!("P2P network error: {}", e);
        }
    });
    
    Ok((command_tx, event_rx))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_network_creation() {
        let config = NetworkConfig::default();
        let result = P2PNetwork::new(config);
        assert!(result.is_ok());
    }
}
