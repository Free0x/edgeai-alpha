//! Blockchain module for EdgeAI
//!
//! This module contains the core blockchain data structures including blocks,
//! transactions, chain state management, mempool, and persistent storage.

pub mod block;
pub mod transaction;
pub mod chain;
pub mod mempool;
pub mod storage;
pub mod oceanbase;

// Core blockchain exports - only export what's actually used externally
pub use block::Block;
pub use transaction::{Transaction, TransactionType};
pub use chain::Blockchain;
pub use mempool::MempoolManager;
