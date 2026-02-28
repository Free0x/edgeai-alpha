//! API module for EdgeAI Blockchain
//!
//! This module provides RESTful API endpoints for blockchain operations,
//! wallet management, data marketplace, device registry, staking, smart contracts,
//! IoT device management, rewards distribution, and on-chain governance.
//!
//! ## Performance Features
//! - **Caching**: In-memory response caching for frequently accessed endpoints
//! - **Rate Limiting**: Token bucket algorithm for fair usage
//! - **Metrics**: Prometheus-compatible performance metrics
//!
//! ## IoT Features
//! - **Device Registration**: Register and manage IoT devices
//! - **Data Contributions**: Submit and track data contributions
//! - **Rewards Distribution**: Calculate and distribute rewards

pub mod auth;
pub mod bridge;
pub mod cache;
pub mod contracts;
pub mod data;
pub mod device;
pub mod dex;
pub mod governance;
pub mod iot;
pub mod metrics;
pub mod prometheus;
pub mod rate_limit;
pub mod rest;
pub mod rewards;
pub mod security;
pub mod staking;
pub mod wallet;

// Authentication exports
pub use auth::{create_sign_message, verify_signed_request, AuthData, SignedRequest};

// REST API exports
pub use rest::{configure_routes, AppState};

// Route configuration exports
pub use bridge::{configure_bridge_routes, BridgeState};
pub use contracts::{configure_contract_routes, ContractState};
pub use data::configure_data_routes;
pub use device::{configure_device_routes, DeviceState};
pub use dex::{configure_dex_routes, DexState};
pub use governance::{configure_governance_routes, GovernanceState};
pub use iot::{configure_iot_routes, IoTRegistry, IoTState};
pub use rewards::{configure_rewards_routes, RewardsState, RewardsSystem};
pub use staking::{configure_staking_routes, StakingState};
pub use wallet::configure_wallet_routes;

// Performance module exports
pub use cache::{create_chain_stats_cache, ApiCache, CacheStats, ChainStatsCache};
pub use metrics::{create_metrics, GlobalMetrics, Metrics, MetricsSnapshot};
pub use prometheus::{create_blockchain_metrics, BlockchainMetrics, GlobalBlockchainMetrics};
pub use rate_limit::{
    create_rate_limiter, GlobalRateLimiter, RateLimitResult, RateLimitTier, RateLimiter,
};
pub use security::{
    create_security_manager, GlobalSecurityManager, SecurityConfig, SecurityError, SecurityManager,
};
