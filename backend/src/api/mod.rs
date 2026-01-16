//! API module for EdgeAI Blockchain
//! 
//! This module provides RESTful API endpoints for blockchain operations,
//! wallet management, data marketplace, device registry, staking, smart contracts,
//! and on-chain governance.
//!
//! ## Performance Features
//! - **Caching**: In-memory response caching for frequently accessed endpoints
//! - **Rate Limiting**: Token bucket algorithm for fair usage
//! - **Metrics**: Prometheus-compatible performance metrics

pub mod auth;
pub mod rest;
pub mod wallet;
pub mod data;
pub mod device;
pub mod staking;
pub mod contracts;
pub mod governance;
pub mod dex;
pub mod cache;
pub mod rate_limit;
pub mod metrics;

// Authentication exports
pub use auth::{SignedRequest, AuthData, verify_signed_request, create_sign_message};

// REST API exports
pub use rest::{AppState, configure_routes};

// Route configuration exports
pub use wallet::configure_wallet_routes;
pub use data::configure_data_routes;
pub use device::{DeviceState, configure_device_routes};
pub use staking::{StakingState, configure_staking_routes};
pub use contracts::{ContractState, configure_contract_routes};
pub use governance::{GovernanceState, configure_governance_routes};
pub use dex::{DexState, configure_dex_routes};

// Performance module exports
pub use cache::{ApiCache, CacheStats, ChainStatsCache, create_chain_stats_cache};
pub use rate_limit::{RateLimiter, RateLimitTier, RateLimitResult, GlobalRateLimiter, create_rate_limiter};
pub use metrics::{Metrics, MetricsSnapshot, GlobalMetrics, create_metrics};
