//! Transaction module for EdgeAI Blockchain
//!
//! This module defines transaction types, data quality metrics,
//! and transaction processing logic.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

use crate::crypto::{address_from_public_key, verify_signature, WalletError};

/// Transaction types in EdgeAI blockchain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Transfer of tokens between accounts
    Transfer,
    /// Data contribution (IoT sensor data, AI model data, etc.)
    DataContribution,
    /// Data purchase from the marketplace
    DataPurchase,
    /// Smart contract deployment
    ContractDeploy,
    /// Smart contract execution
    ContractCall,
    /// Staking tokens for validation
    Stake,
    /// Unstaking tokens
    Unstake,
    /// Reward distribution
    Reward,
    /// Genesis transaction
    Genesis,
}

/// Data quality metrics for PoIE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuality {
    pub entropy_score: f64,      // Information entropy (0-8 for bytes)
    pub uniqueness_score: f64,   // How unique is this data (0-1)
    pub freshness_score: f64,    // How recent is this data (0-1)
    pub completeness_score: f64, // Data completeness (0-1)
    pub overall_score: f64,      // Weighted average
}

impl DataQuality {
    pub fn new(entropy: f64, uniqueness: f64, freshness: f64, completeness: f64) -> Self {
        let overall =
            (entropy / 8.0 * 0.4) + (uniqueness * 0.2) + (freshness * 0.2) + (completeness * 0.2);
        DataQuality {
            entropy_score: entropy,
            uniqueness_score: uniqueness,
            freshness_score: freshness,
            completeness_score: completeness,
            overall_score: overall,
        }
    }

    pub fn default_quality() -> Self {
        DataQuality::new(4.0, 0.5, 1.0, 1.0)
    }
}

/// Transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub tx_hash: String,
    pub output_index: u32,
    pub signature: String,
    pub public_key: String,
}

/// Transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub amount: u64,
    pub recipient: String,
    pub data_hash: Option<String>, // For data transactions
}

/// A transaction in the EdgeAI blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub tx_type: TransactionType,
    pub timestamp: DateTime<Utc>,
    pub sender: String,
    pub sender_public_key: Option<String>, // Public key for signature verification
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub data: Option<String>, // Actual data or reference
    pub data_quality: Option<DataQuality>,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub hash: String,
    pub signature: Option<String>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        tx_type: TransactionType,
        sender: String,
        inputs: Vec<TxInput>,
        outputs: Vec<TxOutput>,
        data: Option<String>,
        gas_price: u64,
        gas_limit: u64,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        // Calculate data quality if this is a data contribution
        let data_quality = if tx_type == TransactionType::DataContribution {
            if let Some(ref d) = data {
                Some(Self::calculate_data_quality(d))
            } else {
                Some(DataQuality::default_quality())
            }
        } else {
            None
        };

        let mut tx = Transaction {
            id,
            tx_type,
            timestamp,
            sender,
            sender_public_key: None,
            inputs,
            outputs,
            data,
            data_quality,
            gas_price,
            gas_limit,
            hash: String::new(),
            signature: None,
        };

        tx.hash = tx.calculate_hash();
        tx
    }

    /// Create a new signed transaction
    pub fn new_signed(
        tx_type: TransactionType,
        sender: String,
        sender_public_key: String,
        inputs: Vec<TxInput>,
        outputs: Vec<TxOutput>,
        data: Option<String>,
        gas_price: u64,
        gas_limit: u64,
        signature: String,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        // Calculate data quality if this is a data contribution
        let data_quality = if tx_type == TransactionType::DataContribution {
            if let Some(ref d) = data {
                Some(Self::calculate_data_quality(d))
            } else {
                Some(DataQuality::default_quality())
            }
        } else {
            None
        };

        let mut tx = Transaction {
            id,
            tx_type,
            timestamp,
            sender,
            sender_public_key: Some(sender_public_key),
            inputs,
            outputs,
            data,
            data_quality,
            gas_price,
            gas_limit,
            hash: String::new(),
            signature: Some(signature),
        };

        tx.hash = tx.calculate_hash();
        tx
    }

    /// Create a genesis transaction
    pub fn genesis() -> Self {
        let output = TxOutput {
            amount: 1_000_000_000, // 1 billion initial tokens
            recipient: "genesis".to_string(),
            data_hash: None,
        };

        Transaction::new(
            TransactionType::Genesis,
            "system".to_string(),
            vec![],
            vec![output],
            Some("EdgeAI Genesis Block - The Most Intelligent Data Chain for Edge AI".to_string()),
            0,
            0,
        )
    }

    /// Create a transfer transaction (unsigned - needs to be signed later)
    pub fn transfer(sender: String, recipient: String, amount: u64) -> Self {
        let output = TxOutput {
            amount,
            recipient,
            data_hash: None,
        };

        Transaction::new(
            TransactionType::Transfer,
            sender,
            vec![],
            vec![output],
            None,
            1,
            21000,
        )
    }

    /// Create a signed transfer transaction
    pub fn transfer_signed(
        sender: String,
        sender_public_key: String,
        recipient: String,
        amount: u64,
        signature: String,
    ) -> Self {
        let output = TxOutput {
            amount,
            recipient,
            data_hash: None,
        };

        Transaction::new_signed(
            TransactionType::Transfer,
            sender,
            sender_public_key,
            vec![],
            vec![output],
            None,
            1,
            21000,
            signature,
        )
    }

    /// Create a data contribution transaction
    pub fn data_contribution(sender: String, data: String, reward_recipient: String) -> Self {
        let data_hash = Self::hash_data(&data);
        let output = TxOutput {
            amount: 0, // Reward will be calculated based on data quality
            recipient: reward_recipient,
            data_hash: Some(data_hash),
        };

        Transaction::new(
            TransactionType::DataContribution,
            sender,
            vec![],
            vec![output],
            Some(data),
            1,
            50000,
        )
    }

    /// Create a signed data contribution transaction
    pub fn data_contribution_signed(
        sender: String,
        sender_public_key: String,
        data: String,
        reward_recipient: String,
        signature: String,
    ) -> Self {
        let data_hash = Self::hash_data(&data);
        let output = TxOutput {
            amount: 0,
            recipient: reward_recipient,
            data_hash: Some(data_hash),
        };

        Transaction::new_signed(
            TransactionType::DataContribution,
            sender,
            sender_public_key,
            vec![],
            vec![output],
            Some(data),
            1,
            50000,
            signature,
        )
    }

    /// Create a data purchase transaction
    pub fn data_purchase(buyer: String, seller: String, data_hash: String, price: u64) -> Self {
        let output = TxOutput {
            amount: price,
            recipient: seller,
            data_hash: Some(data_hash),
        };

        Transaction::new(
            TransactionType::DataPurchase,
            buyer,
            vec![],
            vec![output],
            None,
            1,
            30000,
        )
    }

    /// Create a reward transaction
    pub fn reward(recipient: String, amount: u64, reason: String) -> Self {
        let output = TxOutput {
            amount,
            recipient: recipient.clone(),
            data_hash: None,
        };

        Transaction::new(
            TransactionType::Reward,
            "system".to_string(),
            vec![],
            vec![output],
            Some(reason),
            0,
            0,
        )
    }

    /// Calculate transaction hash (for internal use)
    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}{}",
            self.id,
            self.timestamp,
            self.sender,
            serde_json::to_string(&self.outputs).unwrap(),
            self.data.as_deref().unwrap_or(""),
            self.gas_price
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get the message to be signed (deterministic, based on transaction content)
    pub fn signing_message(&self) -> String {
        self.calculate_hash()
    }

    /// Create deterministic transfer message for signing (used by wallet API)
    pub fn create_transfer_signing_message(from: &str, to: &str, amount: u64) -> String {
        let data = format!("TRANSFER:{}:{}:{}", from, to, amount);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create deterministic data contribution message for signing (used by wallet API)
    pub fn create_data_contribution_signing_message(sender: &str, data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let data_hash = hex::encode(hasher.finalize());

        let message = format!("DATA_CONTRIBUTION:{}:{}", sender, data_hash);
        let mut hasher2 = Sha256::new();
        hasher2.update(message.as_bytes());
        hex::encode(hasher2.finalize())
    }

    /// Set signature on transaction
    pub fn set_signature(&mut self, signature: String, public_key: String) {
        self.signature = Some(signature);
        self.sender_public_key = Some(public_key);
    }

    /// Hash data content
    pub fn hash_data(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Calculate data quality for PoIE
    pub fn calculate_data_quality(data: &str) -> DataQuality {
        let bytes = data.as_bytes();

        // Calculate entropy
        let mut frequency = [0u64; 256];
        for &byte in bytes {
            frequency[byte as usize] += 1;
        }

        let len = bytes.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        // Calculate uniqueness (based on unique byte ratio)
        let unique_bytes = frequency.iter().filter(|&&c| c > 0).count();
        let uniqueness = unique_bytes as f64 / 256.0;

        // Freshness is 1.0 for new data
        let freshness = 1.0;

        // Completeness based on data length (assuming minimum useful length is 100 bytes)
        let completeness = (bytes.len() as f64 / 100.0).min(1.0);

        DataQuality::new(entropy, uniqueness, freshness, completeness)
    }

    /// Calculate reward based on data quality (PoIE)
    pub fn calculate_reward(&self, base_reward: u64) -> u64 {
        if let Some(ref quality) = self.data_quality {
            (base_reward as f64 * quality.overall_score) as u64
        } else {
            0
        }
    }

    /// Verify transaction hash
    pub fn verify_hash(&self) -> bool {
        self.hash == self.calculate_hash()
    }

    /// Verify transaction signature using deterministic message
    pub fn verify_signature_deterministic(&self) -> Result<bool, WalletError> {
        // System transactions don't need signatures
        if self.tx_type == TransactionType::Genesis
            || self.tx_type == TransactionType::Reward
            || self.sender == "system"
        {
            return Ok(true);
        }

        // Check if signature and public key are present
        let signature = match &self.signature {
            Some(s) => s,
            None => return Ok(false),
        };

        let public_key = match &self.sender_public_key {
            Some(pk) => pk,
            None => return Ok(false),
        };

        // Create the deterministic message based on transaction type
        let message = match self.tx_type {
            TransactionType::Transfer => {
                if let Some(output) = self.outputs.first() {
                    Self::create_transfer_signing_message(
                        &self.sender,
                        &output.recipient,
                        output.amount,
                    )
                } else {
                    return Ok(false);
                }
            }
            TransactionType::DataContribution => {
                if let Some(ref data) = self.data {
                    Self::create_data_contribution_signing_message(&self.sender, data)
                } else {
                    return Ok(false);
                }
            }
            _ => self.signing_message(),
        };

        // Verify the signature
        verify_signature(public_key, message.as_bytes(), signature)
    }

    /// Verify transaction signature (legacy method)
    pub fn verify_signature(&self) -> Result<bool, WalletError> {
        self.verify_signature_deterministic()
    }

    /// Verify that sender address matches public key
    pub fn verify_sender(&self) -> Result<bool, WalletError> {
        // System transactions don't need verification
        if self.sender == "system" || self.sender == "genesis" {
            return Ok(true);
        }

        let public_key = match &self.sender_public_key {
            Some(pk) => pk,
            None => return Ok(false),
        };

        let derived_address = address_from_public_key(public_key)?;
        Ok(derived_address == self.sender)
    }

    /// Full verification: hash + signature + sender
    pub fn verify(&self) -> bool {
        if !self.verify_hash() {
            return false;
        }

        // For legacy/unsigned transactions, skip signature verification
        if self.signature.is_none() && self.sender_public_key.is_none() {
            return true;
        }

        // Verify signature
        match self.verify_signature() {
            Ok(valid) => {
                if !valid {
                    return false;
                }
            }
            Err(_) => return false,
        }

        // Verify sender matches public key
        match self.verify_sender() {
            Ok(valid) => valid,
            Err(_) => false,
        }
    }

    /// Get total output amount
    pub fn total_output(&self) -> u64 {
        self.outputs.iter().map(|o| o.amount).sum()
    }
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Transaction {{ id: {}, type: {:?}, sender: {}, hash: {} }}",
            &self.id[..8],
            self.tx_type,
            &self.sender[..8.min(self.sender.len())],
            &self.hash[..8]
        )
    }
}
