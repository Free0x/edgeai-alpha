//! Cryptographic module for EdgeAI Blockchain
//!
//! This module provides wallet management, key generation,
//! and signature verification using ed25519 cryptography.

pub mod wallet;

// Core crypto exports - only export what's actually used
pub use wallet::{address_from_public_key, verify_signature, Wallet, WalletError};
