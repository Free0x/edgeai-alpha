//! Consensus module for EdgeAI Blockchain
//!
//! This module contains the Proof of Information Entropy (PoIE) consensus mechanism,
//! device registry for IoT device management, data quality scoring algorithms,
//! enhanced staking system with delegation and slashing, and on-chain governance.

pub mod data_quality;
pub mod device_registry;
pub mod governance;
pub mod poie;
pub mod staking;

// Core consensus exports
pub use poie::PoIEConsensus;

// Device registry exports - used in main.rs and api/device.rs
pub use device_registry::{Device, DeviceRegistry, DeviceType, GeoRegion};

// Staking exports
pub use staking::{
    Delegation, SlashEvent, SlashReason, StakingConfig, StakingManager, StakingStats,
    StakingValidator, UnbondingEntry, ValidatorDescription, ValidatorStatus,
};

// Governance exports
pub use governance::{
    GovernanceConfig, GovernanceManager, GovernanceStats, Proposal, ProposalStatus, ProposalType,
    VoteOption, VoteTally,
};
