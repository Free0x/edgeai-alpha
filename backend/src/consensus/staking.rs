//! Enhanced Staking Module for EdgeAI Blockchain
//!
//! This module implements a comprehensive staking system including:
//! - Validator staking with lock periods
//! - Delegation (stake delegation to validators)
//! - Unbonding periods for stake withdrawal
//! - Slashing for misbehavior (double signing, downtime)
//! - Reward distribution to validators and delegators

use chrono::{DateTime, Duration, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Staking configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingConfig {
    /// Minimum stake required to become a validator
    pub min_validator_stake: u64,
    /// Minimum delegation amount
    pub min_delegation: u64,
    /// Unbonding period in seconds (default: 7 days)
    pub unbonding_period: i64,
    /// Maximum validators allowed
    pub max_validators: usize,
    /// Slash percentage for double signing (e.g., 0.05 = 5%)
    pub slash_double_sign: f64,
    /// Slash percentage for downtime (e.g., 0.01 = 1%)
    pub slash_downtime: f64,
    /// Minimum uptime percentage required (e.g., 0.95 = 95%)
    pub min_uptime: f64,
    /// Blocks to check for downtime detection
    pub downtime_window: u64,
    /// Commission rate range (min, max)
    pub commission_range: (f64, f64),
}

impl Default for StakingConfig {
    fn default() -> Self {
        StakingConfig {
            min_validator_stake: 10_000,
            min_delegation: 100,
            unbonding_period: 7 * 24 * 60 * 60, // 7 days
            max_validators: 100,
            slash_double_sign: 0.05,
            slash_downtime: 0.01,
            min_uptime: 0.95,
            downtime_window: 1000,
            commission_range: (0.0, 0.25), // 0% - 25%
        }
    }
}

/// Validator status in the staking system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidatorStatus {
    /// Active and participating in consensus
    Active,
    /// Inactive but not jailed
    Inactive,
    /// Jailed due to misbehavior
    Jailed,
    /// Unbonding (withdrawing stake)
    Unbonding,
}

/// Staking validator with enhanced features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingValidator {
    /// Validator's address (public key hash)
    pub address: String,
    /// Validator's operator address (for management)
    pub operator_address: String,
    /// Self-bonded stake amount
    pub self_stake: u64,
    /// Total delegated stake from others
    pub delegated_stake: u64,
    /// Commission rate (0.0 - 1.0)
    pub commission_rate: f64,
    /// Validator status
    pub status: ValidatorStatus,
    /// Reputation score (0-100)
    pub reputation: f64,
    /// Total blocks validated
    pub blocks_validated: u64,
    /// Blocks missed in current window
    pub blocks_missed: u64,
    /// Last block signed
    pub last_block_signed: u64,
    /// Jail release time (if jailed)
    pub jail_until: Option<DateTime<Utc>>,
    /// Registration time
    pub created_at: DateTime<Utc>,
    /// Total rewards earned
    pub total_rewards: u64,
    /// Pending rewards to distribute
    pub pending_rewards: u64,
    /// Validator description/metadata
    pub description: ValidatorDescription,
}

/// Validator description metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidatorDescription {
    pub moniker: String,
    pub identity: Option<String>,
    pub website: Option<String>,
    pub security_contact: Option<String>,
    pub details: Option<String>,
}

impl StakingValidator {
    pub fn new(
        address: String,
        operator_address: String,
        stake: u64,
        commission_rate: f64,
        description: ValidatorDescription,
    ) -> Self {
        StakingValidator {
            address,
            operator_address,
            self_stake: stake,
            delegated_stake: 0,
            commission_rate,
            status: ValidatorStatus::Active,
            reputation: 50.0,
            blocks_validated: 0,
            blocks_missed: 0,
            last_block_signed: 0,
            jail_until: None,
            created_at: Utc::now(),
            total_rewards: 0,
            pending_rewards: 0,
            description,
        }
    }

    /// Get total stake (self + delegated)
    pub fn total_stake(&self) -> u64 {
        self.self_stake + self.delegated_stake
    }

    /// Calculate voting power weight
    pub fn voting_power(&self) -> f64 {
        let stake_weight = (self.total_stake() as f64).sqrt();
        let reputation_weight = self.reputation / 100.0;
        stake_weight * (1.0 + reputation_weight)
    }

    /// Check if validator is eligible for block production
    pub fn is_eligible(&self) -> bool {
        self.status == ValidatorStatus::Active && self.total_stake() > 0
    }

    /// Update uptime statistics
    pub fn record_block_signed(&mut self, block_height: u64) {
        self.blocks_validated += 1;
        self.last_block_signed = block_height;
        self.reputation = (self.reputation + 0.1).min(100.0);
    }

    /// Record missed block
    pub fn record_block_missed(&mut self) {
        self.blocks_missed += 1;
        self.reputation = (self.reputation - 0.5).max(0.0);
    }

    /// Calculate uptime percentage
    pub fn uptime(&self) -> f64 {
        let total = self.blocks_validated + self.blocks_missed;
        if total == 0 {
            return 1.0;
        }
        self.blocks_validated as f64 / total as f64
    }
}

/// Delegation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    /// Delegator's address
    pub delegator: String,
    /// Validator's address
    pub validator: String,
    /// Delegated amount
    pub amount: u64,
    /// Delegation start time
    pub created_at: DateTime<Utc>,
    /// Accumulated rewards
    pub rewards: u64,
}

/// Unbonding entry for stake withdrawal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondingEntry {
    /// Address of the unbonder
    pub address: String,
    /// Validator address (for delegations)
    pub validator: Option<String>,
    /// Amount being unbonded
    pub amount: u64,
    /// Time when unbonding completes
    pub completion_time: DateTime<Utc>,
}

/// Slashing event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashEvent {
    /// Validator that was slashed
    pub validator: String,
    /// Reason for slashing
    pub reason: SlashReason,
    /// Amount slashed
    pub amount: u64,
    /// Block height when slashing occurred
    pub block_height: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Reasons for slashing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashReason {
    /// Signed two different blocks at the same height
    DoubleSigning,
    /// Missed too many blocks
    Downtime,
    /// Submitted invalid data
    InvalidData,
    /// Other misbehavior
    Other(String),
}

/// Main staking manager
pub struct StakingManager {
    /// Configuration
    pub config: StakingConfig,
    /// All validators
    pub validators: HashMap<String, StakingValidator>,
    /// All delegations: delegator -> validator -> Delegation
    pub delegations: HashMap<String, HashMap<String, Delegation>>,
    /// Unbonding queue
    pub unbonding_queue: Vec<UnbondingEntry>,
    /// Slash history
    pub slash_history: Vec<SlashEvent>,
    /// Total staked across all validators
    pub total_staked: u64,
    /// Reward pool for distribution
    pub reward_pool: u64,
}

impl StakingManager {
    pub fn new(config: StakingConfig) -> Self {
        StakingManager {
            config,
            validators: HashMap::new(),
            delegations: HashMap::new(),
            unbonding_queue: Vec::new(),
            slash_history: Vec::new(),
            total_staked: 0,
            reward_pool: 0,
        }
    }

    /// Register a new validator
    pub fn register_validator(
        &mut self,
        address: String,
        operator_address: String,
        stake: u64,
        commission_rate: f64,
        description: ValidatorDescription,
    ) -> Result<(), String> {
        // Check minimum stake
        if stake < self.config.min_validator_stake {
            return Err(format!(
                "Minimum stake required: {} EDGE",
                self.config.min_validator_stake
            ));
        }

        // Check max validators
        if self.validators.len() >= self.config.max_validators {
            return Err("Maximum validators reached".to_string());
        }

        // Check commission rate
        if commission_rate < self.config.commission_range.0
            || commission_rate > self.config.commission_range.1
        {
            return Err(format!(
                "Commission rate must be between {}% and {}%",
                self.config.commission_range.0 * 100.0,
                self.config.commission_range.1 * 100.0
            ));
        }

        // Check if already registered
        if self.validators.contains_key(&address) {
            return Err("Validator already registered".to_string());
        }

        let validator = StakingValidator::new(
            address.clone(),
            operator_address,
            stake,
            commission_rate,
            description,
        );

        self.total_staked += stake;
        self.validators.insert(address.clone(), validator);

        info!(
            "Validator {} registered with stake {} EDGE",
            &address[..8.min(address.len())],
            stake
        );
        Ok(())
    }

    /// Delegate stake to a validator
    pub fn delegate(
        &mut self,
        delegator: String,
        validator_address: String,
        amount: u64,
    ) -> Result<(), String> {
        // Check minimum delegation
        if amount < self.config.min_delegation {
            return Err(format!(
                "Minimum delegation: {} EDGE",
                self.config.min_delegation
            ));
        }

        // Check validator exists and is active
        let validator = self
            .validators
            .get_mut(&validator_address)
            .ok_or("Validator not found")?;

        if validator.status == ValidatorStatus::Jailed {
            return Err("Cannot delegate to jailed validator".to_string());
        }

        // Update validator's delegated stake
        validator.delegated_stake += amount;
        self.total_staked += amount;

        // Create or update delegation record
        let delegator_delegations = self.delegations.entry(delegator.clone()).or_default();

        if let Some(existing) = delegator_delegations.get_mut(&validator_address) {
            existing.amount += amount;
        } else {
            delegator_delegations.insert(
                validator_address.clone(),
                Delegation {
                    delegator: delegator.clone(),
                    validator: validator_address.clone(),
                    amount,
                    created_at: Utc::now(),
                    rewards: 0,
                },
            );
        }

        info!(
            "Delegator {} delegated {} EDGE to validator {}",
            &delegator[..8.min(delegator.len())],
            amount,
            &validator_address[..8.min(validator_address.len())]
        );
        Ok(())
    }

    /// Undelegate stake from a validator (starts unbonding)
    pub fn undelegate(
        &mut self,
        delegator: String,
        validator_address: String,
        amount: u64,
    ) -> Result<DateTime<Utc>, String> {
        // Check delegation exists
        let delegator_delegations = self
            .delegations
            .get_mut(&delegator)
            .ok_or("No delegations found")?;

        let delegation = delegator_delegations
            .get_mut(&validator_address)
            .ok_or("Delegation not found")?;

        if delegation.amount < amount {
            return Err("Insufficient delegation amount".to_string());
        }

        // Update delegation
        delegation.amount -= amount;
        if delegation.amount == 0 {
            delegator_delegations.remove(&validator_address);
        }

        // Update validator
        if let Some(validator) = self.validators.get_mut(&validator_address) {
            validator.delegated_stake -= amount;
        }

        self.total_staked -= amount;

        // Create unbonding entry
        let completion_time = Utc::now() + Duration::seconds(self.config.unbonding_period);
        self.unbonding_queue.push(UnbondingEntry {
            address: delegator.clone(),
            validator: Some(validator_address.clone()),
            amount,
            completion_time,
        });

        info!(
            "Delegator {} started unbonding {} EDGE from validator {}",
            &delegator[..8.min(delegator.len())],
            amount,
            &validator_address[..8.min(validator_address.len())]
        );

        Ok(completion_time)
    }

    /// Process completed unbonding entries
    pub fn process_unbonding(&mut self) -> Vec<UnbondingEntry> {
        let now = Utc::now();
        let (completed, remaining): (Vec<_>, Vec<_>) = self
            .unbonding_queue
            .drain(..)
            .partition(|entry| entry.completion_time <= now);

        self.unbonding_queue = remaining;

        for entry in &completed {
            info!(
                "Unbonding completed: {} EDGE returned to {}",
                entry.amount,
                &entry.address[..8.min(entry.address.len())]
            );
        }

        completed
    }

    /// Slash a validator for misbehavior
    pub fn slash(
        &mut self,
        validator_address: &str,
        reason: SlashReason,
        block_height: u64,
    ) -> Result<u64, String> {
        // First, get validator info without mutable borrow
        let (total_stake, delegated_stake, self_stake) = {
            let validator = self
                .validators
                .get(validator_address)
                .ok_or("Validator not found")?;
            (
                validator.total_stake(),
                validator.delegated_stake,
                validator.self_stake,
            )
        };

        // Determine slash percentage based on reason
        let slash_percentage = match &reason {
            SlashReason::DoubleSigning => self.config.slash_double_sign,
            SlashReason::Downtime => self.config.slash_downtime,
            SlashReason::InvalidData => self.config.slash_downtime,
            SlashReason::Other(_) => self.config.slash_downtime,
        };

        // Calculate slash amount (affects both self-stake and delegated)
        let slash_amount = (total_stake as f64 * slash_percentage) as u64;

        // Slash self-stake first
        let self_slash = slash_amount.min(self_stake);

        // Slash delegated stake proportionally if needed
        let remaining_slash = slash_amount - self_slash;
        let delegated_slash = if remaining_slash > 0 && delegated_stake > 0 {
            remaining_slash.min(delegated_stake)
        } else {
            0
        };

        // Collect delegation updates to apply later
        let delegation_updates: Vec<(String, u64)> = if delegated_slash > 0 {
            self.delegations
                .iter()
                .filter_map(|(delegator, del_map)| {
                    del_map.get(validator_address).map(|d| {
                        let proportion = d.amount as f64 / delegated_stake as f64;
                        let individual_slash = (delegated_slash as f64 * proportion) as u64;
                        (delegator.clone(), individual_slash)
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        // Apply delegation updates
        for (delegator, slash) in delegation_updates {
            if let Some(del_map) = self.delegations.get_mut(&delegator) {
                if let Some(del) = del_map.get_mut(validator_address) {
                    del.amount = del.amount.saturating_sub(slash);
                }
            }
        }

        // Now update validator with mutable borrow
        let validator = self.validators.get_mut(validator_address).unwrap();
        validator.self_stake -= self_slash;
        validator.delegated_stake -= delegated_slash;
        validator.reputation = (validator.reputation - 20.0).max(0.0);

        // Jail validator for double signing
        if reason == SlashReason::DoubleSigning {
            validator.status = ValidatorStatus::Jailed;
            validator.jail_until = Some(Utc::now() + Duration::days(7));
        }

        // Check if validator should be deactivated
        if validator.self_stake < self.config.min_validator_stake {
            validator.status = ValidatorStatus::Inactive;
        }

        self.total_staked -= slash_amount;

        // Record slash event
        self.slash_history.push(SlashEvent {
            validator: validator_address.to_string(),
            reason,
            amount: slash_amount,
            block_height,
            timestamp: Utc::now(),
        });

        warn!(
            "Validator {} slashed {} EDGE",
            &validator_address[..8.min(validator_address.len())],
            slash_amount
        );

        Ok(slash_amount)
    }

    /// Unjail a validator (after jail period)
    pub fn unjail(&mut self, validator_address: &str) -> Result<(), String> {
        let validator = self
            .validators
            .get_mut(validator_address)
            .ok_or("Validator not found")?;

        if validator.status != ValidatorStatus::Jailed {
            return Err("Validator is not jailed".to_string());
        }

        if let Some(jail_until) = validator.jail_until {
            if Utc::now() < jail_until {
                return Err(format!("Jail period not over. Release at: {}", jail_until));
            }
        }

        // Check minimum stake
        if validator.self_stake < self.config.min_validator_stake {
            return Err("Insufficient stake to unjail".to_string());
        }

        validator.status = ValidatorStatus::Active;
        validator.jail_until = None;
        validator.blocks_missed = 0;

        info!(
            "Validator {} unjailed",
            &validator_address[..8.min(validator_address.len())]
        );
        Ok(())
    }

    /// Distribute rewards to validators and delegators
    pub fn distribute_rewards(&mut self, block_reward: u64) {
        if self.validators.is_empty() {
            return;
        }

        // Calculate total voting power
        let total_power: f64 = self
            .validators
            .values()
            .filter(|v| v.is_eligible())
            .map(|v| v.voting_power())
            .sum();

        if total_power == 0.0 {
            return;
        }

        // Distribute to each validator proportionally
        for validator in self.validators.values_mut() {
            if !validator.is_eligible() {
                continue;
            }

            let share = validator.voting_power() / total_power;
            let validator_reward = (block_reward as f64 * share) as u64;

            // Commission goes to validator
            let commission = (validator_reward as f64 * validator.commission_rate) as u64;
            validator.pending_rewards += commission;
            validator.total_rewards += commission;

            // Remaining goes to delegators (proportionally)
            let delegator_pool = validator_reward - commission;
            if validator.delegated_stake > 0 && delegator_pool > 0 {
                // Store for later distribution to delegators
                validator.pending_rewards += delegator_pool;
            } else {
                // If no delegators, all goes to validator
                validator.pending_rewards += delegator_pool;
            }
        }
    }

    /// Get all delegations to a specific validator
    fn get_delegations_to_validator(&self, validator_address: &str) -> Option<Vec<&Delegation>> {
        let delegations: Vec<&Delegation> = self
            .delegations
            .values()
            .filter_map(|del_map| del_map.get(validator_address))
            .collect();

        if delegations.is_empty() {
            None
        } else {
            Some(delegations)
        }
    }

    /// Get validator by address
    pub fn get_validator(&self, address: &str) -> Option<&StakingValidator> {
        self.validators.get(address)
    }

    /// Get all active validators sorted by voting power
    pub fn get_active_validators(&self) -> Vec<&StakingValidator> {
        let mut validators: Vec<_> = self
            .validators
            .values()
            .filter(|v| v.is_eligible())
            .collect();
        validators.sort_by(|a, b| b.voting_power().partial_cmp(&a.voting_power()).unwrap());
        validators
    }

    /// Get delegations for a delegator
    pub fn get_delegations(&self, delegator: &str) -> Vec<&Delegation> {
        self.delegations
            .get(delegator)
            .map(|m| m.values().collect())
            .unwrap_or_default()
    }

    /// Get staking statistics
    pub fn get_stats(&self) -> StakingStats {
        let active_validators = self.validators.values().filter(|v| v.is_eligible()).count();
        let jailed_validators = self
            .validators
            .values()
            .filter(|v| v.status == ValidatorStatus::Jailed)
            .count();
        let total_delegators = self.delegations.len();
        let total_delegated: u64 = self.validators.values().map(|v| v.delegated_stake).sum();

        StakingStats {
            total_validators: self.validators.len(),
            active_validators,
            jailed_validators,
            total_staked: self.total_staked,
            total_delegated,
            total_delegators,
            unbonding_count: self.unbonding_queue.len(),
            slash_events: self.slash_history.len(),
        }
    }
}

/// Staking statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingStats {
    pub total_validators: usize,
    pub active_validators: usize,
    pub jailed_validators: usize,
    pub total_staked: u64,
    pub total_delegated: u64,
    pub total_delegators: usize,
    pub unbonding_count: usize,
    pub slash_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_registration() {
        let mut manager = StakingManager::new(StakingConfig::default());

        // Should fail with low stake
        assert!(manager
            .register_validator(
                "addr1".to_string(),
                "op1".to_string(),
                100,
                0.1,
                ValidatorDescription::default()
            )
            .is_err());

        // Should succeed with sufficient stake
        assert!(manager
            .register_validator(
                "addr2".to_string(),
                "op2".to_string(),
                10_000,
                0.1,
                ValidatorDescription::default()
            )
            .is_ok());
    }

    #[test]
    fn test_delegation() {
        let mut manager = StakingManager::new(StakingConfig::default());

        manager
            .register_validator(
                "validator1".to_string(),
                "op1".to_string(),
                10_000,
                0.1,
                ValidatorDescription::default(),
            )
            .unwrap();

        // Delegate
        assert!(manager
            .delegate("delegator1".to_string(), "validator1".to_string(), 1000)
            .is_ok());

        let validator = manager.get_validator("validator1").unwrap();
        assert_eq!(validator.delegated_stake, 1000);
        assert_eq!(validator.total_stake(), 11_000);
    }

    #[test]
    fn test_slashing() {
        let mut manager = StakingManager::new(StakingConfig::default());

        manager
            .register_validator(
                "validator1".to_string(),
                "op1".to_string(),
                10_000,
                0.1,
                ValidatorDescription::default(),
            )
            .unwrap();

        // Slash for double signing (5%)
        let slashed = manager
            .slash("validator1", SlashReason::DoubleSigning, 100)
            .unwrap();

        assert_eq!(slashed, 500); // 5% of 10,000
        let validator = manager.get_validator("validator1").unwrap();
        assert_eq!(validator.self_stake, 9_500);
        assert_eq!(validator.status, ValidatorStatus::Jailed);
    }
}
