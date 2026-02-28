//! On-chain Governance (DAO) Module
//!
//! This module implements decentralized governance for the EdgeAI blockchain,
//! allowing token holders to propose and vote on protocol changes.
//!
//! # Features
//! - Proposal creation with deposit requirement
//! - Voting with stake-weighted power
//! - Multiple proposal types (parameter change, upgrade, treasury spend)
//! - Configurable voting periods and thresholds
//! - Automatic proposal execution

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Governance configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    /// Minimum deposit required to create a proposal (in smallest unit)
    pub min_deposit: u128,
    /// Duration of the voting period in seconds
    pub voting_period: u64,
    /// Minimum participation rate required for a valid vote (0-100)
    pub quorum_percentage: u8,
    /// Minimum yes votes percentage to pass (0-100)
    pub pass_threshold: u8,
    /// Minimum veto votes percentage to reject (0-100)
    pub veto_threshold: u8,
    /// Delay before execution after passing (in seconds)
    pub execution_delay: u64,
    /// Maximum number of active proposals
    pub max_active_proposals: usize,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            min_deposit: 10_000_000_000_000_000_000_000, // 10,000 EDGE
            voting_period: 7 * 24 * 60 * 60,             // 7 days
            quorum_percentage: 33,                       // 33% participation
            pass_threshold: 50,                          // 50% yes votes
            veto_threshold: 33,                          // 33% veto to reject
            execution_delay: 2 * 24 * 60 * 60,           // 2 days
            max_active_proposals: 10,
        }
    }
}

/// Types of governance proposals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    /// Change a protocol parameter
    ParameterChange {
        module: String,
        parameter: String,
        old_value: String,
        new_value: String,
    },
    /// Protocol upgrade
    SoftwareUpgrade {
        name: String,
        version: String,
        upgrade_height: u64,
        info: String,
    },
    /// Treasury spending
    TreasurySpend {
        recipient: String,
        amount: u128,
        reason: String,
    },
    /// Add or remove validator from active set
    ValidatorChange {
        validator: String,
        action: ValidatorAction,
    },
    /// Free-form text proposal
    Text { content: String },
    /// Emergency action (requires higher threshold)
    Emergency {
        action: String,
        justification: String,
    },
}

/// Actions that can be taken on validators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidatorAction {
    Add,
    Remove,
    Jail,
    Unjail,
}

/// Current status of a proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    /// Proposal is in deposit period, collecting deposits
    DepositPeriod,
    /// Proposal is in voting period
    VotingPeriod,
    /// Proposal passed and is waiting for execution
    Passed,
    /// Proposal was rejected
    Rejected,
    /// Proposal was vetoed
    Vetoed,
    /// Proposal has been executed
    Executed,
    /// Proposal execution failed
    ExecutionFailed { reason: String },
    /// Proposal expired without reaching quorum
    Expired,
}

/// Vote options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VoteOption {
    Yes,
    No,
    Abstain,
    NoWithVeto,
}

/// A single vote cast by an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: String,
    pub option: VoteOption,
    pub voting_power: u128,
    pub timestamp: u64,
}

/// Tally of votes for a proposal
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoteTally {
    pub yes: u128,
    pub no: u128,
    pub abstain: u128,
    pub no_with_veto: u128,
}

impl VoteTally {
    pub fn total(&self) -> u128 {
        self.yes + self.no + self.abstain + self.no_with_veto
    }

    pub fn total_voting(&self) -> u128 {
        self.yes + self.no + self.no_with_veto
    }

    pub fn yes_percentage(&self) -> f64 {
        let total = self.total_voting();
        if total == 0 {
            return 0.0;
        }
        (self.yes as f64 / total as f64) * 100.0
    }

    pub fn veto_percentage(&self) -> f64 {
        let total = self.total_voting();
        if total == 0 {
            return 0.0;
        }
        (self.no_with_veto as f64 / total as f64) * 100.0
    }
}

/// A governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub deposit: u128,
    pub submit_time: u64,
    pub deposit_end_time: u64,
    pub voting_start_time: Option<u64>,
    pub voting_end_time: Option<u64>,
    pub execution_time: Option<u64>,
    pub tally: VoteTally,
    pub votes: HashMap<String, Vote>,
}

impl Proposal {
    pub fn new(
        id: u64,
        proposer: String,
        title: String,
        description: String,
        proposal_type: ProposalType,
        initial_deposit: u128,
        config: &GovernanceConfig,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let deposit_period = 2 * 24 * 60 * 60; // 2 days for deposit period

        let (status, voting_start, voting_end) = if initial_deposit >= config.min_deposit {
            (
                ProposalStatus::VotingPeriod,
                Some(now),
                Some(now + config.voting_period),
            )
        } else {
            (ProposalStatus::DepositPeriod, None, None)
        };

        Self {
            id,
            proposer,
            title,
            description,
            proposal_type,
            status,
            deposit: initial_deposit,
            submit_time: now,
            deposit_end_time: now + deposit_period,
            voting_start_time: voting_start,
            voting_end_time: voting_end,
            execution_time: None,
            tally: VoteTally::default(),
            votes: HashMap::new(),
        }
    }

    pub fn add_deposit(&mut self, amount: u128, config: &GovernanceConfig) -> bool {
        if self.status != ProposalStatus::DepositPeriod {
            return false;
        }

        self.deposit += amount;

        // Check if deposit threshold is met
        if self.deposit >= config.min_deposit {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.status = ProposalStatus::VotingPeriod;
            self.voting_start_time = Some(now);
            self.voting_end_time = Some(now + config.voting_period);
        }

        true
    }

    pub fn cast_vote(
        &mut self,
        voter: String,
        option: VoteOption,
        voting_power: u128,
    ) -> Result<(), &'static str> {
        if self.status != ProposalStatus::VotingPeriod {
            return Err("Proposal is not in voting period");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(end_time) = self.voting_end_time {
            if now > end_time {
                return Err("Voting period has ended");
            }
        }

        // Remove previous vote if exists
        if let Some(prev_vote) = self.votes.get(&voter) {
            match prev_vote.option {
                VoteOption::Yes => self.tally.yes -= prev_vote.voting_power,
                VoteOption::No => self.tally.no -= prev_vote.voting_power,
                VoteOption::Abstain => self.tally.abstain -= prev_vote.voting_power,
                VoteOption::NoWithVeto => self.tally.no_with_veto -= prev_vote.voting_power,
            }
        }

        // Add new vote
        match option {
            VoteOption::Yes => self.tally.yes += voting_power,
            VoteOption::No => self.tally.no += voting_power,
            VoteOption::Abstain => self.tally.abstain += voting_power,
            VoteOption::NoWithVeto => self.tally.no_with_veto += voting_power,
        }

        self.votes.insert(
            voter.clone(),
            Vote {
                voter,
                option,
                voting_power,
                timestamp: now,
            },
        );

        Ok(())
    }

    pub fn finalize(&mut self, total_voting_power: u128, config: &GovernanceConfig) {
        if self.status != ProposalStatus::VotingPeriod {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if voting period has ended
        if let Some(end_time) = self.voting_end_time {
            if now < end_time {
                return; // Voting still in progress
            }
        }

        // Calculate participation rate
        let participation = if total_voting_power > 0 {
            (self.tally.total() as f64 / total_voting_power as f64) * 100.0
        } else {
            0.0
        };

        // Check quorum
        if participation < config.quorum_percentage as f64 {
            self.status = ProposalStatus::Expired;
            return;
        }

        // Check veto threshold
        if self.tally.veto_percentage() >= config.veto_threshold as f64 {
            self.status = ProposalStatus::Vetoed;
            return;
        }

        // Check pass threshold
        if self.tally.yes_percentage() >= config.pass_threshold as f64 {
            self.status = ProposalStatus::Passed;
            self.execution_time = Some(now + config.execution_delay);
        } else {
            self.status = ProposalStatus::Rejected;
        }
    }
}

/// Governance manager handling all proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceManager {
    pub config: GovernanceConfig,
    pub proposals: HashMap<u64, Proposal>,
    pub next_proposal_id: u64,
    /// Deposits by account -> proposal_id -> amount
    pub deposits: HashMap<String, HashMap<u64, u128>>,
}

impl GovernanceManager {
    pub fn new(config: GovernanceConfig) -> Self {
        Self {
            config,
            proposals: HashMap::new(),
            next_proposal_id: 1,
            deposits: HashMap::new(),
        }
    }

    /// Create a new proposal
    pub fn create_proposal(
        &mut self,
        proposer: String,
        title: String,
        description: String,
        proposal_type: ProposalType,
        initial_deposit: u128,
    ) -> Result<u64, &'static str> {
        // Check active proposals limit
        let active_count = self
            .proposals
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProposalStatus::DepositPeriod | ProposalStatus::VotingPeriod
                )
            })
            .count();

        if active_count >= self.config.max_active_proposals {
            return Err("Maximum active proposals reached");
        }

        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal::new(
            proposal_id,
            proposer.clone(),
            title,
            description,
            proposal_type,
            initial_deposit,
            &self.config,
        );

        self.proposals.insert(proposal_id, proposal);

        // Track deposit
        self.deposits
            .entry(proposer)
            .or_default()
            .insert(proposal_id, initial_deposit);

        Ok(proposal_id)
    }

    /// Add deposit to a proposal
    pub fn add_deposit(
        &mut self,
        depositor: String,
        proposal_id: u64,
        amount: u128,
    ) -> Result<(), &'static str> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if !proposal.add_deposit(amount, &self.config) {
            return Err("Cannot add deposit to this proposal");
        }

        // Track deposit
        *self
            .deposits
            .entry(depositor)
            .or_default()
            .entry(proposal_id)
            .or_insert(0) += amount;

        Ok(())
    }

    /// Cast a vote on a proposal
    pub fn vote(
        &mut self,
        voter: String,
        proposal_id: u64,
        option: VoteOption,
        voting_power: u128,
    ) -> Result<(), &'static str> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        proposal.cast_vote(voter, option, voting_power)
    }

    /// Finalize a proposal after voting period ends
    pub fn finalize_proposal(
        &mut self,
        proposal_id: u64,
        total_voting_power: u128,
    ) -> Result<ProposalStatus, &'static str> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        proposal.finalize(total_voting_power, &self.config);

        Ok(proposal.status.clone())
    }

    /// Execute a passed proposal
    pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<(), &'static str> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found")?;

        if proposal.status != ProposalStatus::Passed {
            return Err("Proposal has not passed");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(exec_time) = proposal.execution_time {
            if now < exec_time {
                return Err("Execution delay not yet passed");
            }
        }

        // Execute based on proposal type
        match &proposal.proposal_type {
            ProposalType::ParameterChange {
                module,
                parameter,
                new_value,
                ..
            } => {
                // In a real implementation, this would update the parameter
                log::info!(
                    "Executing parameter change: {}.{} = {}",
                    module,
                    parameter,
                    new_value
                );
            }
            ProposalType::SoftwareUpgrade {
                name,
                version,
                upgrade_height,
                ..
            } => {
                log::info!(
                    "Scheduling upgrade {} v{} at height {}",
                    name,
                    version,
                    upgrade_height
                );
            }
            ProposalType::TreasurySpend {
                recipient,
                amount,
                reason,
            } => {
                log::info!("Treasury spend: {} to {} for {}", amount, recipient, reason);
            }
            ProposalType::ValidatorChange { validator, action } => {
                log::info!("Validator change: {:?} for {}", action, validator);
            }
            ProposalType::Text { content } => {
                log::info!("Text proposal executed: {}", content);
            }
            ProposalType::Emergency {
                action,
                justification,
            } => {
                log::info!("Emergency action: {} - {}", action, justification);
            }
        }

        proposal.status = ProposalStatus::Executed;
        Ok(())
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProposalStatus::DepositPeriod
                        | ProposalStatus::VotingPeriod
                        | ProposalStatus::Passed
                )
            })
            .collect()
    }

    /// Get proposal by ID
    pub fn get_proposal(&self, proposal_id: u64) -> Option<&Proposal> {
        self.proposals.get(&proposal_id)
    }

    /// Get all proposals
    pub fn get_all_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values().collect()
    }

    /// Get deposits by account
    pub fn get_account_deposits(&self, account: &str) -> HashMap<u64, u128> {
        self.deposits.get(account).cloned().unwrap_or_default()
    }

    /// Process expired deposit periods
    pub fn process_expired_deposits(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for proposal in self.proposals.values_mut() {
            if proposal.status == ProposalStatus::DepositPeriod && now > proposal.deposit_end_time {
                proposal.status = ProposalStatus::Expired;
            }
        }
    }

    /// Get governance statistics
    pub fn get_stats(&self) -> GovernanceStats {
        let total_proposals = self.proposals.len();
        let active_proposals = self
            .proposals
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProposalStatus::DepositPeriod | ProposalStatus::VotingPeriod
                )
            })
            .count();
        let passed_proposals = self
            .proposals
            .values()
            .filter(|p| matches!(p.status, ProposalStatus::Passed | ProposalStatus::Executed))
            .count();
        let rejected_proposals = self
            .proposals
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProposalStatus::Rejected | ProposalStatus::Vetoed | ProposalStatus::Expired
                )
            })
            .count();

        let total_votes: usize = self.proposals.values().map(|p| p.votes.len()).sum();

        GovernanceStats {
            total_proposals,
            active_proposals,
            passed_proposals,
            rejected_proposals,
            total_votes,
            config: self.config.clone(),
        }
    }
}

/// Governance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceStats {
    pub total_proposals: usize,
    pub active_proposals: usize,
    pub passed_proposals: usize,
    pub rejected_proposals: usize,
    pub total_votes: usize,
    pub config: GovernanceConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proposal() {
        let mut gov = GovernanceManager::new(GovernanceConfig::default());

        let result = gov.create_proposal(
            "0x1234".to_string(),
            "Test Proposal".to_string(),
            "This is a test proposal".to_string(),
            ProposalType::Text {
                content: "Test content".to_string(),
            },
            10_000_000_000_000_000_000_000, // 10,000 EDGE
        );

        assert!(result.is_ok());
        let proposal_id = result.unwrap();
        let proposal = gov.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::VotingPeriod);
    }

    #[test]
    fn test_vote_tally() {
        let mut tally = VoteTally::default();
        tally.yes = 100;
        tally.no = 50;
        tally.abstain = 25;
        tally.no_with_veto = 25;

        assert_eq!(tally.total(), 200);
        assert_eq!(tally.total_voting(), 175);
        assert!((tally.yes_percentage() - 57.14).abs() < 0.1);
        assert!((tally.veto_percentage() - 14.28).abs() < 0.1);
    }
}
