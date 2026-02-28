//! Proof of Information Entropy (PoIE) Consensus Mechanism
//!
//! PoIE is a novel consensus mechanism designed for EdgeAI that rewards
//! nodes based on the information entropy (value) of their data contributions.

#![allow(dead_code)]

use log::{debug, info};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::blockchain::{Block, Transaction, TransactionType};

/// Proof of Information Entropy (PoIE) Consensus Mechanism
///
/// PoIE is a novel consensus mechanism designed for EdgeAI that:
/// 1. Evaluates the quality and information content of contributed data
/// 2. Rewards nodes based on the entropy (information value) of their data
/// 3. Ensures fair distribution of rewards based on actual data contribution
/// 4. Prevents low-quality or duplicate data from flooding the network

/// Validator node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: String,
    pub stake: u64,
    pub reputation: f64,
    pub data_contributions: u64,
    pub total_entropy_contributed: f64,
    pub is_active: bool,
    pub last_block_validated: u64,
}

impl Validator {
    pub fn new(address: String, stake: u64) -> Self {
        Validator {
            address,
            stake,
            reputation: 50.0, // Start with neutral reputation
            data_contributions: 0,
            total_entropy_contributed: 0.0,
            is_active: true,
            last_block_validated: 0,
        }
    }

    /// Calculate validator's weight for block selection
    pub fn weight(&self) -> f64 {
        let stake_weight = (self.stake as f64).sqrt();
        let reputation_weight = self.reputation / 100.0;
        let entropy_weight = (self.total_entropy_contributed / 1000.0).min(1.0);

        stake_weight * (1.0 + reputation_weight + entropy_weight)
    }
}

/// PoIE Consensus Engine
pub struct PoIEConsensus {
    pub validators: HashMap<String, Validator>,
    pub min_stake: u64,
    pub min_entropy_threshold: f64,
    pub block_time: u64, // Target block time in seconds
    pub entropy_reward_multiplier: f64,
    pub duplicate_penalty: f64,
    pub seen_data_hashes: HashMap<String, u64>, // hash -> block_index
}

impl PoIEConsensus {
    pub fn new() -> Self {
        PoIEConsensus {
            validators: HashMap::new(),
            min_stake: 1000,
            min_entropy_threshold: 2.0, // Minimum entropy for valid data
            block_time: 10,
            entropy_reward_multiplier: 10.0,
            duplicate_penalty: 0.5,
            seen_data_hashes: HashMap::new(),
        }
    }

    /// Register a new validator
    pub fn register_validator(&mut self, address: String, stake: u64) -> Result<(), String> {
        if stake < self.min_stake {
            return Err(format!("Minimum stake required: {}", self.min_stake));
        }

        let validator = Validator::new(address.clone(), stake);
        self.validators.insert(address.clone(), validator);

        info!(
            "Validator {} registered with stake {}",
            &address[..8],
            stake
        );
        Ok(())
    }

    /// Select the next block validator based on PoIE
    pub fn select_validator(&self, _block_entropy: f64, random_seed: &[u8]) -> Option<String> {
        let active_validators: Vec<&Validator> =
            self.validators.values().filter(|v| v.is_active).collect();

        if active_validators.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: f64 = active_validators.iter().map(|v| v.weight()).sum();

        // Use random seed to select validator
        let mut hasher = Sha256::new();
        hasher.update(random_seed);
        let hash = hasher.finalize();
        let random_value =
            u64::from_be_bytes(hash[..8].try_into().unwrap()) as f64 / u64::MAX as f64;

        let target = random_value * total_weight;
        let mut cumulative = 0.0;

        for validator in &active_validators {
            cumulative += validator.weight();
            if cumulative >= target {
                return Some(validator.address.clone());
            }
        }

        active_validators.last().map(|v| v.address.clone())
    }

    /// Validate a block according to PoIE rules
    pub fn validate_block(&mut self, block: &Block) -> Result<ValidationResult, String> {
        let mut result = ValidationResult::new();

        // Check block entropy meets minimum threshold
        if block.header.data_entropy < self.min_entropy_threshold
            && !block
                .transactions
                .iter()
                .all(|tx| tx.tx_type == TransactionType::Reward)
        {
            // Allow low entropy only for reward-only blocks
            debug!(
                "Block entropy {} below threshold {}",
                block.header.data_entropy, self.min_entropy_threshold
            );
        }

        // Validate each data contribution transaction
        for tx in &block.transactions {
            if tx.tx_type == TransactionType::DataContribution {
                let tx_result = self.validate_data_contribution(tx, block.index)?;
                result.merge(tx_result);
            }
        }

        // Update validator stats
        if let Some(validator) = self.validators.get_mut(&block.validator) {
            validator.last_block_validated = block.index;
            validator.reputation += result.reputation_change;
            validator.total_entropy_contributed += block.header.data_entropy;
        }

        result.is_valid = true;
        result.block_entropy = block.header.data_entropy;

        Ok(result)
    }

    /// Validate a data contribution transaction
    fn validate_data_contribution(
        &mut self,
        tx: &Transaction,
        block_index: u64,
    ) -> Result<ValidationResult, String> {
        let mut result = ValidationResult::new();

        let quality = tx.data_quality.as_ref().ok_or("Data quality not found")?;

        // Check entropy threshold
        if quality.entropy_score < self.min_entropy_threshold {
            result.reputation_change -= 5.0;
            result.warnings.push(format!(
                "Low entropy data: {:.2} < {:.2}",
                quality.entropy_score, self.min_entropy_threshold
            ));
        }

        // Check for duplicate data
        if let Some(ref data) = tx.data {
            let data_hash = Transaction::hash_data(data);

            if let Some(prev_block) = self.seen_data_hashes.get(&data_hash) {
                result.is_duplicate = true;
                result.reputation_change -= 10.0;
                result.reward_multiplier *= self.duplicate_penalty;
                result.warnings.push(format!(
                    "Duplicate data detected (first seen in block {})",
                    prev_block
                ));
            } else {
                self.seen_data_hashes.insert(data_hash, block_index);
            }
        }

        // Calculate reward based on quality
        result.entropy_reward = (quality.overall_score * self.entropy_reward_multiplier) as u64;
        result.reputation_change += quality.overall_score * 5.0;

        Ok(result)
    }

    /// Calculate block reward based on PoIE
    pub fn calculate_block_reward(&self, block: &Block, base_reward: u64) -> u64 {
        // Base reward + entropy bonus
        let entropy_bonus = (block.header.data_entropy * self.entropy_reward_multiplier) as u64;

        // Count high-quality data contributions
        let quality_bonus: u64 = block
            .transactions
            .iter()
            .filter(|tx| tx.tx_type == TransactionType::DataContribution)
            .filter_map(|tx| tx.data_quality.as_ref())
            .map(|q| (q.overall_score * 10.0) as u64)
            .sum();

        base_reward + entropy_bonus + quality_bonus
    }

    /// Get validator by address
    pub fn get_validator(&self, address: &str) -> Option<&Validator> {
        self.validators.get(address)
    }

    /// Get all active validators
    pub fn get_active_validators(&self) -> Vec<&Validator> {
        self.validators.values().filter(|v| v.is_active).collect()
    }

    /// Update validator stake
    pub fn update_stake(&mut self, address: &str, new_stake: u64) -> Result<(), String> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or("Validator not found")?;

        if new_stake < self.min_stake {
            validator.is_active = false;
        } else {
            validator.is_active = true;
        }

        validator.stake = new_stake;
        Ok(())
    }

    /// Slash validator for misbehavior
    pub fn slash_validator(&mut self, address: &str, percentage: f64) -> Result<u64, String> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or("Validator not found")?;

        let slash_amount = (validator.stake as f64 * percentage) as u64;
        validator.stake -= slash_amount;
        validator.reputation -= 20.0;

        if validator.stake < self.min_stake {
            validator.is_active = false;
        }

        info!(
            "Validator {} slashed {} tokens",
            &address[..8],
            slash_amount
        );
        Ok(slash_amount)
    }
}

/// Result of block/transaction validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub is_duplicate: bool,
    pub block_entropy: f64,
    pub entropy_reward: u64,
    pub reputation_change: f64,
    pub reward_multiplier: f64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            is_valid: false,
            is_duplicate: false,
            block_entropy: 0.0,
            entropy_reward: 0,
            reputation_change: 0.0,
            reward_multiplier: 1.0,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.entropy_reward += other.entropy_reward;
        self.reputation_change += other.reputation_change;
        self.reward_multiplier *= other.reward_multiplier;
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);

        if other.is_duplicate {
            self.is_duplicate = true;
        }
    }
}

/// Entropy calculator for various data types
pub struct EntropyCalculator;

impl EntropyCalculator {
    /// Calculate Shannon entropy of byte data
    pub fn shannon_entropy(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut frequency = [0u64; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Calculate normalized entropy (0-1 scale)
    pub fn normalized_entropy(data: &[u8]) -> f64 {
        let entropy = Self::shannon_entropy(data);
        entropy / 8.0 // Max entropy for bytes is 8 bits
    }

    /// Calculate entropy for JSON data (considers structure)
    pub fn json_entropy(json_str: &str) -> f64 {
        let base_entropy = Self::shannon_entropy(json_str.as_bytes());

        // Bonus for structured data
        let structure_bonus = if json_str.contains('{') && json_str.contains('}') {
            0.5
        } else {
            0.0
        };

        base_entropy + structure_bonus
    }

    /// Calculate entropy for sensor data
    pub fn sensor_data_entropy(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        // Calculate variance-based entropy
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        // Higher variance = higher entropy
        (1.0 + variance).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        // Random data should have high entropy
        let random_data: Vec<u8> = (0..256).collect();
        let entropy = EntropyCalculator::shannon_entropy(&random_data);
        assert!(entropy > 7.0);

        // Repetitive data should have low entropy
        let repetitive_data = vec![0u8; 256];
        let low_entropy = EntropyCalculator::shannon_entropy(&repetitive_data);
        assert!(low_entropy < 1.0);
    }

    #[test]
    fn test_validator_registration() {
        let mut consensus = PoIEConsensus::new();

        // Should fail with low stake
        assert!(consensus
            .register_validator("addr1".to_string(), 100)
            .is_err());

        // Should succeed with sufficient stake
        assert!(consensus
            .register_validator("addr2".to_string(), 1000)
            .is_ok());
        assert!(consensus.validators.contains_key("addr2"));
    }

    #[test]
    fn test_validator_selection() {
        let mut consensus = PoIEConsensus::new();
        consensus
            .register_validator("validator1".to_string(), 5000)
            .unwrap();
        consensus
            .register_validator("validator2".to_string(), 3000)
            .unwrap();

        let seed = b"random_seed_123";
        let selected = consensus.select_validator(5.0, seed);
        assert!(selected.is_some());
    }
}
