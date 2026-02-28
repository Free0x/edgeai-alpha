//! Staking API endpoints for EdgeAI Blockchain
//!
//! This module provides HTTP endpoints for validator staking, delegation,
//! unbonding, and reward management.

#![allow(dead_code)]

use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::rest::ApiResponse;
use crate::consensus::{SlashReason, StakingConfig, StakingManager, ValidatorDescription};

/// Staking state (shared across handlers)
pub struct StakingState {
    pub manager: Arc<RwLock<StakingManager>>,
}

// ============ Request Types ============

#[derive(Debug, Deserialize)]
pub struct RegisterValidatorRequest {
    pub address: String,
    pub operator_address: String,
    pub stake: u64,
    pub commission_rate: f64,
    pub moniker: String,
    pub website: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DelegateRequest {
    pub delegator: String,
    pub validator: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct UndelegateRequest {
    pub delegator: String,
    pub validator: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct UnjailRequest {
    pub validator: String,
}

// ============ Response Types ============

#[derive(Debug, Serialize)]
pub struct ValidatorResponse {
    pub address: String,
    pub operator_address: String,
    pub moniker: String,
    pub self_stake: u64,
    pub delegated_stake: u64,
    pub total_stake: u64,
    pub commission_rate: f64,
    pub status: String,
    pub reputation: f64,
    pub voting_power: f64,
    pub blocks_validated: u64,
    pub uptime: f64,
    pub total_rewards: u64,
}

#[derive(Debug, Serialize)]
pub struct DelegationResponse {
    pub delegator: String,
    pub validator: String,
    pub amount: u64,
    pub rewards: u64,
}

#[derive(Debug, Serialize)]
pub struct StakingStatsResponse {
    pub total_validators: usize,
    pub active_validators: usize,
    pub jailed_validators: usize,
    pub total_staked: u64,
    pub total_delegated: u64,
    pub total_delegators: usize,
    pub unbonding_count: usize,
    pub slash_events: usize,
}

#[derive(Debug, Serialize)]
pub struct UnbondingResponse {
    pub completion_time: String,
    pub amount: u64,
}

// ============ Handlers ============

/// Get staking statistics
pub async fn get_staking_stats(data: web::Data<StakingState>) -> impl Responder {
    let manager = data.manager.read().await;
    let stats = manager.get_stats();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(StakingStatsResponse {
            total_validators: stats.total_validators,
            active_validators: stats.active_validators,
            jailed_validators: stats.jailed_validators,
            total_staked: stats.total_staked,
            total_delegated: stats.total_delegated,
            total_delegators: stats.total_delegators,
            unbonding_count: stats.unbonding_count,
            slash_events: stats.slash_events,
        }),
        error: None,
    })
}

/// Get all validators
pub async fn get_validators(data: web::Data<StakingState>) -> impl Responder {
    let manager = data.manager.read().await;
    let validators: Vec<ValidatorResponse> = manager
        .validators
        .values()
        .map(|v| ValidatorResponse {
            address: v.address.clone(),
            operator_address: v.operator_address.clone(),
            moniker: v.description.moniker.clone(),
            self_stake: v.self_stake,
            delegated_stake: v.delegated_stake,
            total_stake: v.total_stake(),
            commission_rate: v.commission_rate,
            status: format!("{:?}", v.status),
            reputation: v.reputation,
            voting_power: v.voting_power(),
            blocks_validated: v.blocks_validated,
            uptime: v.uptime(),
            total_rewards: v.total_rewards,
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(validators),
        error: None,
    })
}

/// Get active validators (sorted by voting power)
pub async fn get_active_validators(data: web::Data<StakingState>) -> impl Responder {
    let manager = data.manager.read().await;
    let validators: Vec<ValidatorResponse> = manager
        .get_active_validators()
        .iter()
        .map(|v| ValidatorResponse {
            address: v.address.clone(),
            operator_address: v.operator_address.clone(),
            moniker: v.description.moniker.clone(),
            self_stake: v.self_stake,
            delegated_stake: v.delegated_stake,
            total_stake: v.total_stake(),
            commission_rate: v.commission_rate,
            status: format!("{:?}", v.status),
            reputation: v.reputation,
            voting_power: v.voting_power(),
            blocks_validated: v.blocks_validated,
            uptime: v.uptime(),
            total_rewards: v.total_rewards,
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(validators),
        error: None,
    })
}

/// Get validator by address
pub async fn get_validator(
    data: web::Data<StakingState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();
    let manager = data.manager.read().await;

    match manager.get_validator(&address) {
        Some(v) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(ValidatorResponse {
                address: v.address.clone(),
                operator_address: v.operator_address.clone(),
                moniker: v.description.moniker.clone(),
                self_stake: v.self_stake,
                delegated_stake: v.delegated_stake,
                total_stake: v.total_stake(),
                commission_rate: v.commission_rate,
                status: format!("{:?}", v.status),
                reputation: v.reputation,
                voting_power: v.voting_power(),
                blocks_validated: v.blocks_validated,
                uptime: v.uptime(),
                total_rewards: v.total_rewards,
            }),
            error: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Validator not found".to_string()),
        }),
    }
}

/// Register a new validator
pub async fn register_validator(
    data: web::Data<StakingState>,
    req: web::Json<RegisterValidatorRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    let description = ValidatorDescription {
        moniker: req.moniker.clone(),
        identity: None,
        website: req.website.clone(),
        security_contact: None,
        details: req.details.clone(),
    };

    match manager.register_validator(
        req.address.clone(),
        req.operator_address.clone(),
        req.stake,
        req.commission_rate,
        description,
    ) {
        Ok(()) => {
            info!(
                "Validator {} registered via API",
                &req.address[..8.min(req.address.len())]
            );
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some("Validator registered successfully"),
                error: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

/// Delegate stake to a validator
pub async fn delegate(
    data: web::Data<StakingState>,
    req: web::Json<DelegateRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    match manager.delegate(req.delegator.clone(), req.validator.clone(), req.amount) {
        Ok(()) => {
            info!(
                "Delegation: {} -> {} ({} EDGE)",
                &req.delegator[..8.min(req.delegator.len())],
                &req.validator[..8.min(req.validator.len())],
                req.amount
            );
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some("Delegation successful"),
                error: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

/// Undelegate stake from a validator
pub async fn undelegate(
    data: web::Data<StakingState>,
    req: web::Json<UndelegateRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    match manager.undelegate(req.delegator.clone(), req.validator.clone(), req.amount) {
        Ok(completion_time) => {
            info!(
                "Undelegation started: {} <- {} ({} EDGE)",
                &req.delegator[..8.min(req.delegator.len())],
                &req.validator[..8.min(req.validator.len())],
                req.amount
            );
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some(UnbondingResponse {
                    completion_time: completion_time.to_rfc3339(),
                    amount: req.amount,
                }),
                error: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

/// Get delegations for a delegator
pub async fn get_delegations(
    data: web::Data<StakingState>,
    path: web::Path<String>,
) -> impl Responder {
    let delegator = path.into_inner();
    let manager = data.manager.read().await;

    let delegations: Vec<DelegationResponse> = manager
        .get_delegations(&delegator)
        .iter()
        .map(|d| DelegationResponse {
            delegator: d.delegator.clone(),
            validator: d.validator.clone(),
            amount: d.amount,
            rewards: d.rewards,
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(delegations),
        error: None,
    })
}

/// Unjail a validator
pub async fn unjail(
    data: web::Data<StakingState>,
    req: web::Json<UnjailRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    match manager.unjail(&req.validator) {
        Ok(()) => {
            info!(
                "Validator {} unjailed via API",
                &req.validator[..8.min(req.validator.len())]
            );
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some("Validator unjailed successfully"),
                error: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

/// Get staking configuration
pub async fn get_config(data: web::Data<StakingState>) -> impl Responder {
    let manager = data.manager.read().await;
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(&manager.config),
        error: None,
    })
}

/// Configure staking routes
pub fn configure_staking_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/staking")
            .route("/stats", web::get().to(get_staking_stats))
            .route("/config", web::get().to(get_config))
            .route("/validators", web::get().to(get_validators))
            .route("/validators/active", web::get().to(get_active_validators))
            .route("/validators/{address}", web::get().to(get_validator))
            .route("/validators/register", web::post().to(register_validator))
            .route("/delegate", web::post().to(delegate))
            .route("/undelegate", web::post().to(undelegate))
            .route("/delegations/{delegator}", web::get().to(get_delegations))
            .route("/unjail", web::post().to(unjail)),
    );
}
