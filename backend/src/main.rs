mod blockchain;
mod consensus;
mod contracts;
mod crypto;
mod data_market;
mod network;
mod api;
mod iot;
mod validators;

use std::sync::Arc;
use tokio::sync::RwLock;
use actix_web::{web, App, HttpServer, middleware};
use actix_cors::Cors;
use actix_web::http::header;
use actix_files::Files;
use log::{info, LevelFilter};
use env_logger::Builder;
use std::fs;
use std::path::Path;

use chrono::{Utc, Timelike};
use blockchain::{Blockchain, MempoolManager};
use consensus::{PoIEConsensus, DeviceRegistry, StakingManager, StakingConfig, GovernanceManager, GovernanceConfig};
use data_market::DataMarketplace;
use network::{NetworkManager, NodeType};
use network::libp2p_network::{NetworkConfig, NetworkCommand, NetworkEvent, start_p2p_network};
use api::{
    AppState, DeviceState, StakingState, ContractState, GovernanceState, DexState, IoTState,
    configure_routes, configure_wallet_routes, configure_data_routes, 
    configure_device_routes, configure_staking_routes, configure_contract_routes,
    configure_governance_routes, configure_dex_routes, configure_iot_routes
};
use contracts::WasmRuntime;

const DATA_DIR: &str = "/data";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp_secs()
        .init();
    
    info!("============================================");
    info!("   EdgeAI Blockchain Node v0.7.0");
    info!("   The Most Intelligent Data Chain");
    info!("   PoIE 2.0 + OceanBase Cloud Storage!");
    info!("============================================");
    
    // Ensure data directory exists
    if !Path::new(DATA_DIR).exists() {
        info!("Creating data directory at {}", DATA_DIR);
        fs::create_dir_all(DATA_DIR)?;
    } else {
        info!("Data directory found at {}", DATA_DIR);
    }

    // Initialize blockchain (will load from local disk first)
    let mut chain_instance = Blockchain::new();
    
    // Initialize OceanBase connection asynchronously
    chain_instance.init_oceanbase().await;
    
    let blockchain = Arc::new(RwLock::new(chain_instance));
    
    // Initialize consensus
    let consensus = Arc::new(RwLock::new(PoIEConsensus::new()));
    info!("PoIE consensus engine initialized");
    
    // Initialize device registry (PoIE 2.0)
    let device_registry = Arc::new(RwLock::new(DeviceRegistry::new()));
    info!("Device Registry initialized (PoIE 2.0)");
    
    // Initialize staking manager with custom config
    let staking_config = StakingConfig {
        min_validator_stake: 10_000,
        min_delegation: 100,
        unbonding_period: 7 * 24 * 60 * 60, // 7 days
        max_validators: 100,
        slash_double_sign: 0.05,  // 5%
        slash_downtime: 0.01,     // 1%
        min_uptime: 0.95,         // 95%
        downtime_window: 1000,
        commission_range: (0.0, 0.25), // 0-25%
    };
    // Create staking manager and register initial validators before wrapping in Arc
    let mut staking_mgr = StakingManager::new(staking_config);
    
    // Register initial validators for testnet
    {
        use consensus::ValidatorDescription;
        
        let initial_validators = vec![
            ("edge_validator_foundation", "EdgeAI Foundation", "Official foundation validator node", 15_000_000, 0.05),
            ("edge_validator_iot_hub", "IoT Network Hub", "High-performance edge computing node", 12_000_000, 0.08),
            ("edge_validator_datastream", "DataStream Validator", "Specialized in medical IoT data", 9_500_000, 0.10),
            ("edge_validator_smartcity", "Smart City Node", "Urban infrastructure data processing", 8_200_000, 0.07),
            ("edge_validator_green", "Green Energy Validator", "Renewable energy monitoring network", 7_100_000, 0.06),
        ];
        
        for (addr, name, desc, stake, commission) in initial_validators {
            let description = ValidatorDescription {
                moniker: name.to_string(),
                identity: None,
                website: Some(format!("https://{}.edgeai.network", addr)),
                security_contact: None,
                details: Some(desc.to_string()),
            };
            let _ = staking_mgr.register_validator(
                addr.to_string(),
                format!("{}_operator", addr),
                stake,
                commission,
                description,
            );
        }
        info!("Registered {} initial validators for testnet", 5);
    }
    
    let staking_manager = Arc::new(RwLock::new(staking_mgr));
    info!("Staking Manager initialized (Delegation + Slashing)");
    
    // Initialize governance manager with custom config
    let governance_config = GovernanceConfig {
        min_deposit: 10_000_000_000_000_000_000_000, // 10,000 EDGE
        voting_period: 7 * 24 * 60 * 60,             // 7 days
        quorum_percentage: 33,                       // 33% participation
        pass_threshold: 50,                          // 50% yes votes
        veto_threshold: 33,                          // 33% veto to reject
        execution_delay: 2 * 24 * 60 * 60,           // 2 days
        max_active_proposals: 10,
    };
    let governance_manager = Arc::new(RwLock::new(GovernanceManager::new(governance_config)));
    info!("Governance Manager initialized (On-chain DAO)");
    
    // Initialize marketplace
    let marketplace = Arc::new(RwLock::new(DataMarketplace::new()));
    info!("Data marketplace initialized");
    
    // Initialize WASM runtime for smart contracts
    let wasm_runtime = Arc::new(RwLock::new(WasmRuntime::new()));
    info!("WASM Smart Contract Runtime initialized");
    
    // Initialize network
    let node_id = format!("node_{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    let network = Arc::new(NetworkManager::new(
        node_id.clone(),
        NodeType::FullNode,
        8333,
    ));
    info!("Network manager initialized (Node ID: {})", &node_id);
    
    // Initialize libp2p P2P network
    let p2p_port: u16 = std::env::var("EDGEAI_P2P_PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .unwrap_or(9000);
    
    let bootstrap_nodes: Vec<String> = std::env::var("EDGEAI_BOOTSTRAP_NODES")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();
    
    if !bootstrap_nodes.is_empty() {
        info!("Bootstrap nodes: {:?}", bootstrap_nodes);
    }
    
    let p2p_config = NetworkConfig {
        listen_port: p2p_port,
        bootstrap_nodes,
        enable_mdns: true,
        max_peers: 50,
    };
    
    #[allow(unused_mut)]
    let (p2p_command_tx, mut p2p_event_rx) = match start_p2p_network(p2p_config).await {
        Ok((tx, rx)) => {
            info!("libp2p P2P network started on port {}", p2p_port);
            (Some(tx), Some(rx))
        }
        Err(e) => {
            log::warn!("Failed to start P2P network: {}. Running in standalone mode.", e);
            (None, None)
        }
    };
    
    let p2p_tx = Arc::new(tokio::sync::RwLock::new(p2p_command_tx));
    
    // Create app state
    let app_state = web::Data::new(AppState {
        blockchain: blockchain.clone(),
        consensus: consensus.clone(),
        marketplace: marketplace.clone(),
        network: network.clone(),
    });
    
    let device_state = web::Data::new(DeviceState {
        registry: device_registry.clone(),
    });
    
    let staking_state = web::Data::new(StakingState {
        manager: staking_manager.clone(),
    });
    
    let contract_state = web::Data::new(ContractState {
        runtime: wasm_runtime.clone(),
    });
    
    let governance_state: web::Data<GovernanceState> = web::Data::new(governance_manager.clone());

    let dex_state = web::Data::new(DexState::new());
    info!("DEX initialized with default trading pairs");

    let iot_state = web::Data::new(IoTState {
        registry: Arc::new(RwLock::new(api::iot::IoTRegistry::new())),
    });
    info!("IoT Registry initialized for external device integration");

    // Start P2P event handler
    if let Some(mut event_rx) = p2p_event_rx {
        let p2p_blockchain = blockchain.clone();
        let p2p_device_registry = device_registry.clone();
        tokio::spawn(async move {
            info!("P2P event handler started");
            while let Some(event) = event_rx.recv().await {
                match event {
                    NetworkEvent::PeerConnected(peer_id) => {
                        info!("P2P: Peer connected: {}", peer_id);
                    }
                    NetworkEvent::PeerDisconnected(peer_id) => {
                        info!("P2P: Peer disconnected: {}", peer_id);
                    }
                    NetworkEvent::NewTransaction(tx) => {
                        info!("P2P: Received transaction: {}", &tx.hash[..8]);
                        let mut chain = p2p_blockchain.write().await;
                        if let Err(e) = chain.add_transaction(tx) {
                            log::warn!("P2P: Transaction rejected: {}", e);
                        }
                    }
                    NetworkEvent::NewBlock(block) => {
                        info!("P2P: Received block #{}", block.index);
                        // TODO: Validate and add block from peer
                    }
                    NetworkEvent::NewContribution(contrib) => {
                        info!("P2P: Received contribution from {}", &contrib.device_id[..8]);
                        let mut registry = p2p_device_registry.write().await;
                        if let Some(device) = registry.get_device_mut(&contrib.device_id) {
                            let quality_score = 0.7;
                            let points = 10.0;
                            device.record_contribution(quality_score, points);
                        }
                    }
                    NetworkEvent::Ready => {
                        info!("P2P: Network ready");
                    }
                }
            }
        });
    }
    
    // Start background mining task with OceanBase async persistence
    let mining_blockchain = blockchain.clone();
    let mining_validator = node_id.clone();
    let mining_p2p_tx = p2p_tx.clone();
    let mining_device_registry = device_registry.clone();
    let mining_staking = staking_manager.clone();
    let mining_governance = governance_manager.clone();
    
    tokio::spawn(async move {
        info!("Block producer started (with OceanBase async sync)");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            let mut chain = mining_blockchain.write().await;
            let current_height = chain.chain.len() as u64;
            
            // Update device activity status every 100 blocks
            if current_height % 100 == 0 {
                let mut registry = mining_device_registry.write().await;
                registry.update_activity_status(24);
                let stats = registry.get_stats();
                info!("Device Registry: {} total, {} active, {} regions", 
                    stats.total_devices, stats.active_devices, stats.regions_covered);
                
                let mut staking = mining_staking.write().await;
                let completed = staking.process_unbonding();
                if !completed.is_empty() {
                    info!("Processed {} unbonding entries", completed.len());
                }
                
                let mut governance = mining_governance.write().await;
                governance.process_expired_deposits();
            }
            
            // Distribute staking rewards every block
            {
                let mut staking = mining_staking.write().await;
                let block_reward = 100;
                staking.distribute_rewards(block_reward);
            }
            
            // Collect pending transactions from mempool
            // Realistic transaction volume: 3-8 tx/block with diurnal variation
            let mut mempool = MempoolManager::with_block_context(current_height);
            let hour_of_day = (Utc::now().hour() + 8) % 24; // Approximate UTC+8 peak hours
            let diurnal_factor = match hour_of_day {
                6..=9   => 1.4,  // Morning ramp-up
                10..=13 => 1.8,  // Peak business hours
                14..=17 => 1.5,  // Afternoon
                18..=21 => 1.2,  // Evening wind-down
                22..=23 | 0..=1 => 0.6, // Late night
                _ => 0.4,        // Deep night (2-5 AM)
            };
            let base = 3;
            let variation = (current_height % 7) as usize; // 0-6 deterministic jitter
            let batch_size = ((base + variation) as f64 * diurnal_factor) as usize;
            let batch_size = batch_size.max(1).min(15); // Clamp to [1, 15]
            let pending_txs = mempool.collect_pending(batch_size);
            log::debug!("Mempool: {} pending transactions for block {}", pending_txs.len(), current_height);
            
            let mut added_count = 0;
            let mut failed_count = 0;
            for tx in pending_txs {
                match chain.add_transaction(tx) {
                    Ok(_) => added_count += 1,
                    Err(e) => {
                        failed_count += 1;
                        log::warn!("Transaction rejected: {}", e);
                    }
                }
            }
            if failed_count > 0 {
                log::warn!("Mempool: {} added, {} rejected", added_count, failed_count);
            }
            
            // Produce new block
            match chain.mine_block(mining_validator.clone()) {
                Ok((block, affected_accounts)) => {
                    info!("Produced block #{} with {} transactions", 
                          block.index, block.transactions.len());
                    
                    // Async persist to OceanBase (non-blocking)
                    chain.persist_block_async(&block).await;
                    
                    // Sync affected accounts to OceanBase every block
                    chain.persist_accounts_async(&affected_accounts).await;
                    
                    // Sync metadata to OceanBase every 10 blocks
                    if block.index % 10 == 0 {
                        chain.persist_state_async().await;
                    }
                    
                    // Broadcast block to P2P network
                    let p2p_guard = mining_p2p_tx.read().await;
                    if let Some(ref tx) = *p2p_guard {
                        let _ = tx.send(NetworkCommand::BroadcastBlock(block.clone())).await;
                    }
                },
                Err(e) => {
                    log::warn!("Block production failed: {}", e);
                }
            }
        }
    });
    
    let bind_address = "0.0.0.0:8080";
    info!("Starting HTTP server at http://{}", bind_address);
    info!("API endpoints available at http://{}/api/", bind_address);
    info!("Device Registry API at http://{}/api/devices/", bind_address);
    info!("Staking API at http://{}/api/staking/", bind_address);
    info!("Smart Contracts API at http://{}/api/contracts/", bind_address);
    info!("Governance API at http://{}/api/governance/", bind_address);
    info!("DEX API at http://{}/api/dex/", bind_address);
    info!("Block Explorer available at http://{}/", bind_address);
    
    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://edgeai-alpha.vercel.app")
            .allowed_origin("https://frontend-flame-kappa-42.vercel.app")
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().ends_with(b".vercel.app")
            })
            .allowed_origin("https://edgeai-chain.github.io")
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://127.0.0.1:3000")
            .allowed_origin("http://127.0.0.1:5173")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            .supports_credentials()
            .max_age(3600);
        
        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            .app_data(device_state.clone())
            .app_data(staking_state.clone())
            .app_data(contract_state.clone())
            .app_data(governance_state.clone())
            .app_data(dex_state.clone())
            .app_data(iot_state.clone())
            .configure(configure_routes)
            .configure(configure_wallet_routes)
            .configure(configure_data_routes)
            .configure(configure_device_routes)
            .configure(configure_staking_routes)
            .configure(configure_contract_routes)
            .configure(configure_governance_routes)
            .configure(|cfg| configure_dex_routes(cfg, dex_state.clone()))
            .configure(configure_iot_routes)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(bind_address)?
    .run()
    .await
}
