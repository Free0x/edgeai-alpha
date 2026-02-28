//! Blockchain module for EdgeAI
//!
//! This module contains the core blockchain data structures including blocks,
//! transactions, chain state management, mempool, and persistent storage.

pub mod block;
pub mod chain;
pub mod mempool;
pub mod oceanbase;
pub mod storage;
pub mod transaction;

// Core blockchain exports - only export what's actually used externally
pub use block::Block;
pub use chain::Blockchain;
pub use mempool::MempoolManager;
pub use transaction::{Transaction, TransactionType};
