//! Block module for EdgeAI Blockchain
//!
//! This module defines the Block structure and related operations
//! for the EdgeAI blockchain.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::blockchain::transaction::Transaction;

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub previous_hash: String,
    pub merkle_root: String,
    pub timestamp: DateTime<Utc>,
    pub difficulty: u64,
    pub nonce: u64,
    pub data_entropy: f64, // PoIE: Information entropy of data in this block
}

/// A block in the EdgeAI blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub hash: String,
    pub validator: String, // Node that validated this block
}

impl Block {
    /// Create a new block
    pub fn new(
        index: u64,
        previous_hash: String,
        transactions: Vec<Transaction>,
        difficulty: u64,
        validator: String,
    ) -> Self {
        let timestamp = Utc::now();
        let merkle_root = Self::calculate_merkle_root(&transactions);
        let data_entropy = Self::calculate_data_entropy(&transactions);

        let header = BlockHeader {
            version: 1,
            previous_hash,
            merkle_root,
            timestamp,
            difficulty,
            nonce: 0,
            data_entropy,
        };

        let mut block = Block {
            index,
            header,
            transactions,
            hash: String::new(),
            validator,
        };

        block.hash = block.calculate_hash();
        block
    }

    /// Create the genesis block
    pub fn genesis() -> Self {
        let genesis_tx = Transaction::genesis();
        Block::new(
            0,
            "0".repeat(64),
            vec![genesis_tx],
            1,
            "genesis".to_string(),
        )
    }

    /// Calculate the hash of the block
    pub fn calculate_hash(&self) -> String {
        let header_data = serde_json::to_string(&self.header).unwrap();
        let tx_data = serde_json::to_string(&self.transactions).unwrap();
        let data = format!("{}{}{}", self.index, header_data, tx_data);

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Calculate merkle root of transactions
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }

        let mut hashes: Vec<String> = transactions.iter().map(|tx| tx.hash.clone()).collect();

        while hashes.len() > 1 {
            let mut new_hashes = Vec::new();
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    format!("{}{}", chunk[0], chunk[0])
                };

                let mut hasher = Sha256::new();
                hasher.update(combined.as_bytes());
                new_hashes.push(hex::encode(hasher.finalize()));
            }
            hashes = new_hashes;
        }

        hashes.pop().unwrap_or_else(|| "0".repeat(64))
    }

    /// Calculate information entropy of data in transactions (PoIE)
    pub fn calculate_data_entropy(transactions: &[Transaction]) -> f64 {
        if transactions.is_empty() {
            return 0.0;
        }

        // Collect all data bytes from transactions
        let mut all_data: Vec<u8> = Vec::new();
        for tx in transactions {
            if let Some(ref data) = tx.data {
                all_data.extend(data.as_bytes());
            }
        }

        if all_data.is_empty() {
            return 0.0;
        }

        // Calculate Shannon entropy
        let mut frequency = [0u64; 256];
        for &byte in &all_data {
            frequency[byte as usize] += 1;
        }

        let len = all_data.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Mine the block (find valid nonce for PoIE)
    pub fn mine(&mut self, difficulty: u64) {
        let target = "0".repeat(difficulty as usize);

        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
            self.header.nonce += 1;
        }
    }

    /// Verify the block's hash
    pub fn verify(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Get block size in bytes
    pub fn size(&self) -> usize {
        serde_json::to_string(self).unwrap().len()
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} [Hash: {}..., Txs: {}, Entropy: {:.4}]",
            self.index,
            &self.hash[..8],
            self.transactions.len(),
            self.header.data_entropy
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.index, 0);
        assert!(genesis.verify());
    }

    #[test]
    fn test_block_mining() {
        let mut block = Block::new(1, "0".repeat(64), vec![], 1, "test_validator".to_string());
        block.mine(1);
        assert!(block.hash.starts_with("0"));
    }
}
