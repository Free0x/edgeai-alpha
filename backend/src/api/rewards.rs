//! Rewards Distribution System
//!
//! Comprehensive reward calculation, distribution, and tracking system
//! for EdgeAI blockchain data contributors.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use log::{info, warn};

use super::rest::ApiResponse;

// ============ Constants ============

/// Base reward per data contribution (EDGE)
const BASE_REWARD_PER_CONTRIBUTION: f64 = 0.1;

/// Base reward per KB of data (EDGE)
const BASE_REWARD_PER_KB: f64 = 0.001;

/// Maximum daily reward per device (EDGE)
const MAX_DAILY_REWARD_PER_DEVICE: f64 = 100.0;

/// Minimum reward threshold for distribution (EDGE)
const MIN_DISTRIBUTION_THRESHOLD: f64 = 1.0;

/// Reward pool replenishment rate (EDGE per block)
const POOL_REPLENISHMENT_RATE: f64 = 10.0;

/// Distribution interval (seconds) - every hour
const DISTRIBUTION_INTERVAL_SECS: u64 = 3600;

// ============ Data Types ============

/// Reward types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RewardType {
    DataContribution,       // Reward for contributing data
    QualityBonus,          // Bonus for high-quality data
    ConsistencyBonus,      // Bonus for consistent contributions
    ScarcityBonus,         // Bonus for rare data types/regions
    EarlyAdopterBonus,     // Bonus for early participants
    ReferralBonus,         // Bonus for referring new devices
    ValidatorReward,       // Reward for block validation
    StakingReward,         // Reward for staking
    GovernanceReward,      // Reward for governance participation
}

/// Reward status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RewardStatus {
    Pending,      // Calculated but not distributed
    Processing,   // Being distributed
    Distributed,  // Successfully distributed
    Claimed,      // Claimed by recipient
    Failed,       // Distribution failed
    Expired,      // Expired (not claimed in time)
}

/// Individual reward record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRecord {
    pub reward_id: String,
    pub recipient: String,           // Device ID or wallet address
    pub reward_type: RewardType,
    pub amount: f64,
    pub status: RewardStatus,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub distributed_at: Option<u64>,
    pub claimed_at: Option<u64>,
    pub tx_hash: Option<String>,
}

/// Reward multipliers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardMultipliers {
    // Quality multipliers (0.5x to 2.0x)
    pub quality_low: f64,      // < 0.3 quality
    pub quality_medium: f64,   // 0.3 - 0.7 quality
    pub quality_high: f64,     // > 0.7 quality
    
    // Device type multipliers
    pub device_type_multipliers: HashMap<String, f64>,
    
    // Region scarcity multipliers
    pub region_multipliers: HashMap<String, f64>,
    
    // Time-based multipliers
    pub peak_hours_multiplier: f64,
    pub off_peak_multiplier: f64,
    
    // Streak multipliers (consecutive days)
    pub streak_3_days: f64,
    pub streak_7_days: f64,
    pub streak_30_days: f64,
}

impl Default for RewardMultipliers {
    fn default() -> Self {
        let mut device_type_multipliers = HashMap::new();
        device_type_multipliers.insert("EdgeAIProcessor".to_string(), 2.0);
        device_type_multipliers.insert("Camera".to_string(), 1.5);
        device_type_multipliers.insert("MedicalDevice".to_string(), 1.8);
        device_type_multipliers.insert("IndustrialSensor".to_string(), 1.4);
        device_type_multipliers.insert("WeatherStation".to_string(), 1.3);
        
        let mut region_multipliers = HashMap::new();
        // Higher multipliers for underrepresented regions
        region_multipliers.insert("AF".to_string(), 2.0); // Africa
        region_multipliers.insert("SA".to_string(), 1.8); // South America
        region_multipliers.insert("OC".to_string(), 1.5); // Oceania
        
        Self {
            quality_low: 0.5,
            quality_medium: 1.0,
            quality_high: 1.5,
            device_type_multipliers,
            region_multipliers,
            peak_hours_multiplier: 0.8,  // Lower during peak (more competition)
            off_peak_multiplier: 1.2,    // Higher during off-peak
            streak_3_days: 1.1,
            streak_7_days: 1.25,
            streak_30_days: 1.5,
        }
    }
}

/// Reward pool state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPool {
    pub total_balance: f64,
    pub reserved_balance: f64,       // Reserved for pending distributions
    pub distributed_total: f64,
    pub last_replenishment: u64,
    pub replenishment_rate: f64,
}

impl Default for RewardPool {
    fn default() -> Self {
        Self {
            total_balance: 1_000_000.0,  // Initial pool: 1M EDGE
            reserved_balance: 0.0,
            distributed_total: 0.0,
            last_replenishment: 0,
            replenishment_rate: POOL_REPLENISHMENT_RATE,
        }
    }
}

/// Daily reward summary for a recipient
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DailyRewardSummary {
    pub date: String,                // YYYY-MM-DD
    pub recipient: String,
    pub total_rewards: f64,
    pub contribution_count: u64,
    pub data_bytes: u64,
    pub avg_quality: f64,
    pub rewards_by_type: HashMap<String, f64>,
}

/// Rewards system state
pub struct RewardsSystem {
    pub pool: RewardPool,
    pub multipliers: RewardMultipliers,
    pub pending_rewards: Vec<RewardRecord>,
    pub distributed_rewards: Vec<RewardRecord>,
    pub daily_summaries: HashMap<String, Vec<DailyRewardSummary>>, // date -> summaries
    pub recipient_totals: HashMap<String, f64>,  // recipient -> total earned
    pub last_distribution: u64,
}

impl RewardsSystem {
    pub fn new() -> Self {
        Self {
            pool: RewardPool::default(),
            multipliers: RewardMultipliers::default(),
            pending_rewards: Vec::new(),
            distributed_rewards: Vec::new(),
            daily_summaries: HashMap::new(),
            recipient_totals: HashMap::new(),
            last_distribution: 0,
        }
    }

    /// Calculate reward for a data contribution
    pub fn calculate_contribution_reward(
        &self,
        data_bytes: u64,
        quality_score: f64,
        device_type: &str,
        region: &str,
        streak_days: u32,
    ) -> RewardCalculation {
        // Base reward
        let base_reward = BASE_REWARD_PER_CONTRIBUTION + 
            (data_bytes as f64 / 1024.0) * BASE_REWARD_PER_KB;
        
        // Quality multiplier
        let quality_mult = if quality_score < 0.3 {
            self.multipliers.quality_low
        } else if quality_score < 0.7 {
            self.multipliers.quality_medium
        } else {
            self.multipliers.quality_high
        };
        
        // Device type multiplier
        let type_mult = self.multipliers.device_type_multipliers
            .get(device_type)
            .copied()
            .unwrap_or(1.0);
        
        // Region multiplier
        let region_mult = self.multipliers.region_multipliers
            .get(region)
            .copied()
            .unwrap_or(1.0);
        
        // Streak multiplier
        let streak_mult = if streak_days >= 30 {
            self.multipliers.streak_30_days
        } else if streak_days >= 7 {
            self.multipliers.streak_7_days
        } else if streak_days >= 3 {
            self.multipliers.streak_3_days
        } else {
            1.0
        };
        
        // Time-based multiplier (simplified)
        let hour = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() / 3600) % 24;
        let time_mult = if hour >= 9 && hour <= 17 {
            self.multipliers.peak_hours_multiplier
        } else {
            self.multipliers.off_peak_multiplier
        };
        
        // Calculate final reward
        let total_multiplier = quality_mult * type_mult * region_mult * streak_mult * time_mult;
        let final_reward = base_reward * total_multiplier;
        
        RewardCalculation {
            base_reward,
            quality_multiplier: quality_mult,
            device_type_multiplier: type_mult,
            region_multiplier: region_mult,
            streak_multiplier: streak_mult,
            time_multiplier: time_mult,
            total_multiplier,
            final_reward,
        }
    }

    /// Add a pending reward
    pub fn add_pending_reward(&mut self, reward: RewardRecord) -> Result<(), String> {
        // Check pool balance
        if self.pool.total_balance - self.pool.reserved_balance < reward.amount {
            return Err("Insufficient pool balance".to_string());
        }
        
        // Reserve the amount
        self.pool.reserved_balance += reward.amount;
        
        self.pending_rewards.push(reward);
        Ok(())
    }

    /// Process pending rewards distribution
    pub fn distribute_pending_rewards(&mut self) -> Vec<RewardRecord> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut distributed = Vec::new();
        let mut remaining = Vec::new();
        
        for mut reward in self.pending_rewards.drain(..) {
            if reward.amount >= MIN_DISTRIBUTION_THRESHOLD {
                // Process distribution
                reward.status = RewardStatus::Distributed;
                reward.distributed_at = Some(now);
                reward.tx_hash = Some(format!("0x{:064x}", rand::random::<u128>()));
                
                // Update pool
                self.pool.total_balance -= reward.amount;
                self.pool.reserved_balance -= reward.amount;
                self.pool.distributed_total += reward.amount;
                
                // Update recipient total
                *self.recipient_totals.entry(reward.recipient.clone()).or_insert(0.0) += reward.amount;
                
                distributed.push(reward.clone());
                self.distributed_rewards.push(reward);
            } else {
                remaining.push(reward);
            }
        }
        
        self.pending_rewards = remaining;
        self.last_distribution = now;
        
        // Keep only last 10000 distributed rewards
        if self.distributed_rewards.len() > 10000 {
            self.distributed_rewards = self.distributed_rewards.split_off(self.distributed_rewards.len() - 10000);
        }
        
        distributed
    }

    /// Replenish reward pool (called per block)
    pub fn replenish_pool(&mut self, block_height: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.pool.total_balance += self.pool.replenishment_rate;
        self.pool.last_replenishment = now;
    }

    /// Get rewards for a recipient
    pub fn get_rewards_for_recipient(&self, recipient: &str) -> Vec<&RewardRecord> {
        self.distributed_rewards
            .iter()
            .filter(|r| r.recipient == recipient)
            .collect()
    }

    /// Get pending rewards for a recipient
    pub fn get_pending_for_recipient(&self, recipient: &str) -> Vec<&RewardRecord> {
        self.pending_rewards
            .iter()
            .filter(|r| r.recipient == recipient)
            .collect()
    }

    /// Get system statistics
    pub fn get_stats(&self) -> RewardsStats {
        let total_pending: f64 = self.pending_rewards.iter().map(|r| r.amount).sum();
        let unique_recipients = self.recipient_totals.len();
        
        let rewards_by_type: HashMap<String, f64> = self.distributed_rewards
            .iter()
            .fold(HashMap::new(), |mut acc, r| {
                *acc.entry(format!("{:?}", r.reward_type)).or_insert(0.0) += r.amount;
                acc
            });
        
        RewardsStats {
            pool_balance: self.pool.total_balance,
            pool_reserved: self.pool.reserved_balance,
            total_distributed: self.pool.distributed_total,
            pending_count: self.pending_rewards.len() as u64,
            pending_amount: total_pending,
            distributed_count: self.distributed_rewards.len() as u64,
            unique_recipients: unique_recipients as u64,
            rewards_by_type,
            last_distribution: self.last_distribution,
        }
    }
}

/// Reward calculation breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculation {
    pub base_reward: f64,
    pub quality_multiplier: f64,
    pub device_type_multiplier: f64,
    pub region_multiplier: f64,
    pub streak_multiplier: f64,
    pub time_multiplier: f64,
    pub total_multiplier: f64,
    pub final_reward: f64,
}

/// Rewards system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardsStats {
    pub pool_balance: f64,
    pub pool_reserved: f64,
    pub total_distributed: f64,
    pub pending_count: u64,
    pub pending_amount: f64,
    pub distributed_count: u64,
    pub unique_recipients: u64,
    pub rewards_by_type: HashMap<String, f64>,
    pub last_distribution: u64,
}

// ============ API State ============

pub struct RewardsState {
    pub system: Arc<RwLock<RewardsSystem>>,
}

// ============ Request/Response Types ============

#[derive(Debug, Deserialize)]
pub struct CalculateRewardRequest {
    pub data_bytes: u64,
    pub quality_score: f64,
    pub device_type: String,
    pub region: String,
    pub streak_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRewardRequest {
    pub recipient: String,
    pub reward_type: String,
    pub amount: f64,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct RewardResponse {
    pub reward_id: String,
    pub recipient: String,
    pub reward_type: String,
    pub amount: f64,
    pub status: String,
    pub created_at: u64,
    pub distributed_at: Option<u64>,
    pub tx_hash: Option<String>,
}

impl From<&RewardRecord> for RewardResponse {
    fn from(r: &RewardRecord) -> Self {
        Self {
            reward_id: r.reward_id.clone(),
            recipient: r.recipient.clone(),
            reward_type: format!("{:?}", r.reward_type),
            amount: r.amount,
            status: format!("{:?}", r.status),
            created_at: r.created_at,
            distributed_at: r.distributed_at,
            tx_hash: r.tx_hash.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RecipientRewardsResponse {
    pub recipient: String,
    pub total_earned: f64,
    pub pending_amount: f64,
    pub pending_count: u64,
    pub distributed_count: u64,
    pub recent_rewards: Vec<RewardResponse>,
}

// ============ Helper Functions ============

fn parse_reward_type(type_str: &str) -> RewardType {
    match type_str.to_lowercase().as_str() {
        "data_contribution" | "contribution" => RewardType::DataContribution,
        "quality_bonus" | "quality" => RewardType::QualityBonus,
        "consistency_bonus" | "consistency" => RewardType::ConsistencyBonus,
        "scarcity_bonus" | "scarcity" => RewardType::ScarcityBonus,
        "early_adopter" | "early" => RewardType::EarlyAdopterBonus,
        "referral" | "referral_bonus" => RewardType::ReferralBonus,
        "validator" | "validator_reward" => RewardType::ValidatorReward,
        "staking" | "staking_reward" => RewardType::StakingReward,
        "governance" | "governance_reward" => RewardType::GovernanceReward,
        _ => RewardType::DataContribution,
    }
}

// ============ API Endpoints ============

/// Calculate potential reward (preview)
pub async fn calculate_reward(
    data: web::Data<RewardsState>,
    body: web::Json<CalculateRewardRequest>,
) -> impl Responder {
    let system = data.system.read().await;
    
    let calculation = system.calculate_contribution_reward(
        body.data_bytes,
        body.quality_score,
        &body.device_type,
        &body.region,
        body.streak_days.unwrap_or(0),
    );
    
    HttpResponse::Ok().json(ApiResponse::success(calculation))
}

/// Create a new reward (internal/admin)
pub async fn create_reward(
    data: web::Data<RewardsState>,
    body: web::Json<CreateRewardRequest>,
) -> impl Responder {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let reward_id = format!("reward_{:016x}", rand::random::<u64>());
    
    let reward = RewardRecord {
        reward_id: reward_id.clone(),
        recipient: body.recipient.clone(),
        reward_type: parse_reward_type(&body.reward_type),
        amount: body.amount,
        status: RewardStatus::Pending,
        metadata: body.metadata.clone().unwrap_or_default(),
        created_at: now,
        distributed_at: None,
        claimed_at: None,
        tx_hash: None,
    };
    
    let mut system = data.system.write().await;
    
    match system.add_pending_reward(reward.clone()) {
        Ok(_) => {
            info!("Reward created: {} for {} ({} EDGE)", 
                &reward_id, &body.recipient, body.amount);
            
            let response = RewardResponse::from(&reward);
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Trigger reward distribution (admin/scheduled)
pub async fn distribute_rewards(
    data: web::Data<RewardsState>,
) -> impl Responder {
    let mut system = data.system.write().await;
    
    let distributed = system.distribute_pending_rewards();
    let count = distributed.len();
    let total: f64 = distributed.iter().map(|r| r.amount).sum();
    
    info!("Distributed {} rewards totaling {} EDGE", count, total);
    
    #[derive(Serialize)]
    struct DistributionResult {
        distributed_count: usize,
        total_amount: f64,
        rewards: Vec<RewardResponse>,
    }
    
    HttpResponse::Ok().json(ApiResponse::success(DistributionResult {
        distributed_count: count,
        total_amount: total,
        rewards: distributed.iter().map(RewardResponse::from).collect(),
    }))
}

/// Get rewards for a recipient
pub async fn get_recipient_rewards(
    data: web::Data<RewardsState>,
    path: web::Path<String>,
) -> impl Responder {
    let recipient = path.into_inner();
    let system = data.system.read().await;
    
    let distributed = system.get_rewards_for_recipient(&recipient);
    let pending = system.get_pending_for_recipient(&recipient);
    
    let total_earned = system.recipient_totals.get(&recipient).copied().unwrap_or(0.0);
    let pending_amount: f64 = pending.iter().map(|r| r.amount).sum();
    
    let mut recent: Vec<&RewardRecord> = distributed.into_iter().chain(pending.into_iter()).collect();
    recent.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    recent.truncate(50);
    
    let response = RecipientRewardsResponse {
        recipient: recipient.clone(),
        total_earned,
        pending_amount,
        pending_count: system.get_pending_for_recipient(&recipient).len() as u64,
        distributed_count: system.get_rewards_for_recipient(&recipient).len() as u64,
        recent_rewards: recent.iter().map(|r| RewardResponse::from(*r)).collect(),
    };
    
    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// Get rewards system statistics
pub async fn get_rewards_stats(
    data: web::Data<RewardsState>,
) -> impl Responder {
    let system = data.system.read().await;
    let stats = system.get_stats();
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Get reward multipliers configuration
pub async fn get_multipliers(
    data: web::Data<RewardsState>,
) -> impl Responder {
    let system = data.system.read().await;
    HttpResponse::Ok().json(ApiResponse::success(system.multipliers.clone()))
}

/// Get reward pool status
pub async fn get_pool_status(
    data: web::Data<RewardsState>,
) -> impl Responder {
    let system = data.system.read().await;
    HttpResponse::Ok().json(ApiResponse::success(system.pool.clone()))
}

/// Get leaderboard by total rewards
pub async fn get_rewards_leaderboard(
    data: web::Data<RewardsState>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let limit = query.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20usize)
        .min(100);
    
    let system = data.system.read().await;
    
    let mut leaderboard: Vec<(&String, &f64)> = system.recipient_totals.iter().collect();
    leaderboard.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    leaderboard.truncate(limit);
    
    #[derive(Serialize)]
    struct LeaderboardEntry {
        rank: usize,
        recipient: String,
        total_rewards: f64,
    }
    
    let entries: Vec<LeaderboardEntry> = leaderboard
        .into_iter()
        .enumerate()
        .map(|(i, (recipient, total))| LeaderboardEntry {
            rank: i + 1,
            recipient: recipient.clone(),
            total_rewards: *total,
        })
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(entries))
}

/// Get recent distributed rewards
pub async fn get_recent_rewards(
    data: web::Data<RewardsState>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let limit = query.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50usize)
        .min(100);
    
    let system = data.system.read().await;
    
    let recent: Vec<RewardResponse> = system.distributed_rewards
        .iter()
        .rev()
        .take(limit)
        .map(RewardResponse::from)
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(recent))
}

// ============ Router Configuration ============

pub fn configure_rewards_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Reward calculation
        .route("/api/rewards/calculate", web::post().to(calculate_reward))
        
        // Reward management
        .route("/api/rewards/create", web::post().to(create_reward))
        .route("/api/rewards/distribute", web::post().to(distribute_rewards))
        
        // Query endpoints
        .route("/api/rewards/stats", web::get().to(get_rewards_stats))
        .route("/api/rewards/pool", web::get().to(get_pool_status))
        .route("/api/rewards/multipliers", web::get().to(get_multipliers))
        .route("/api/rewards/leaderboard", web::get().to(get_rewards_leaderboard))
        .route("/api/rewards/recent", web::get().to(get_recent_rewards))
        .route("/api/rewards/recipient/{recipient}", web::get().to(get_recipient_rewards));
}
