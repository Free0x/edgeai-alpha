//! Governance API Endpoints
//!
//! REST API endpoints for on-chain governance operations.

#![allow(dead_code)]

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::auth::{SignedRequest, AuthData, verify_signed_request};
use crate::consensus::governance::{
    GovernanceManager, GovernanceStats, Proposal, ProposalStatus, ProposalType,
    ValidatorAction, VoteOption, VoteTally,
};

/// Shared governance state
pub type GovernanceState = Arc<RwLock<GovernanceManager>>;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProposalRequest {
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalTypeRequest,
    pub initial_deposit: String, // Amount in smallest unit as string
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ProposalTypeRequest {
    #[serde(rename = "parameter_change")]
    ParameterChange {
        module: String,
        parameter: String,
        old_value: String,
        new_value: String,
    },
    #[serde(rename = "software_upgrade")]
    SoftwareUpgrade {
        name: String,
        version: String,
        upgrade_height: u64,
        info: String,
    },
    #[serde(rename = "treasury_spend")]
    TreasurySpend {
        recipient: String,
        amount: String,
        reason: String,
    },
    #[serde(rename = "validator_change")]
    ValidatorChange {
        validator: String,
        action: String, // "add", "remove", "jail", "unjail"
    },
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "emergency")]
    Emergency {
        action: String,
        justification: String,
    },
}

impl From<ProposalTypeRequest> for ProposalType {
    fn from(req: ProposalTypeRequest) -> Self {
        match req {
            ProposalTypeRequest::ParameterChange {
                module,
                parameter,
                old_value,
                new_value,
            } => ProposalType::ParameterChange {
                module,
                parameter,
                old_value,
                new_value,
            },
            ProposalTypeRequest::SoftwareUpgrade {
                name,
                version,
                upgrade_height,
                info,
            } => ProposalType::SoftwareUpgrade {
                name,
                version,
                upgrade_height,
                info,
            },
            ProposalTypeRequest::TreasurySpend {
                recipient,
                amount,
                reason,
            } => ProposalType::TreasurySpend {
                recipient,
                amount: amount.parse().unwrap_or(0),
                reason,
            },
            ProposalTypeRequest::ValidatorChange { validator, action } => {
                let action = match action.to_lowercase().as_str() {
                    "add" => ValidatorAction::Add,
                    "remove" => ValidatorAction::Remove,
                    "jail" => ValidatorAction::Jail,
                    "unjail" => ValidatorAction::Unjail,
                    _ => ValidatorAction::Add,
                };
                ProposalType::ValidatorChange { validator, action }
            }
            ProposalTypeRequest::Text { content } => ProposalType::Text { content },
            ProposalTypeRequest::Emergency {
                action,
                justification,
            } => ProposalType::Emergency {
                action,
                justification,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateProposalResponse {
    pub success: bool,
    pub proposal_id: u64,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub depositor: String,
    pub amount: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VoteRequest {
    pub voter: String,
    pub option: String, // "yes", "no", "abstain", "no_with_veto"
    pub voting_power: String,
}

#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: u64,
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub proposal_type: String,
    pub status: String,
    pub deposit: String,
    pub submit_time: u64,
    pub voting_start_time: Option<u64>,
    pub voting_end_time: Option<u64>,
    pub execution_time: Option<u64>,
    pub tally: TallyResponse,
    pub vote_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TallyResponse {
    pub yes: String,
    pub no: String,
    pub abstain: String,
    pub no_with_veto: String,
    pub total: String,
    pub yes_percentage: f64,
    pub veto_percentage: f64,
}

impl From<&Proposal> for ProposalResponse {
    fn from(p: &Proposal) -> Self {
        let proposal_type = match &p.proposal_type {
            ProposalType::ParameterChange { .. } => "parameter_change",
            ProposalType::SoftwareUpgrade { .. } => "software_upgrade",
            ProposalType::TreasurySpend { .. } => "treasury_spend",
            ProposalType::ValidatorChange { .. } => "validator_change",
            ProposalType::Text { .. } => "text",
            ProposalType::Emergency { .. } => "emergency",
        };

        let status = match &p.status {
            ProposalStatus::DepositPeriod => "deposit_period",
            ProposalStatus::VotingPeriod => "voting_period",
            ProposalStatus::Passed => "passed",
            ProposalStatus::Rejected => "rejected",
            ProposalStatus::Vetoed => "vetoed",
            ProposalStatus::Executed => "executed",
            ProposalStatus::ExecutionFailed { .. } => "execution_failed",
            ProposalStatus::Expired => "expired",
        };

        ProposalResponse {
            id: p.id,
            proposer: p.proposer.clone(),
            title: p.title.clone(),
            description: p.description.clone(),
            proposal_type: proposal_type.to_string(),
            status: status.to_string(),
            deposit: p.deposit.to_string(),
            submit_time: p.submit_time,
            voting_start_time: p.voting_start_time,
            voting_end_time: p.voting_end_time,
            execution_time: p.execution_time,
            tally: TallyResponse {
                yes: p.tally.yes.to_string(),
                no: p.tally.no.to_string(),
                abstain: p.tally.abstain.to_string(),
                no_with_veto: p.tally.no_with_veto.to_string(),
                total: p.tally.total().to_string(),
                yes_percentage: p.tally.yes_percentage(),
                veto_percentage: p.tally.veto_percentage(),
            },
            vote_count: p.votes.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct GovernanceStatsResponse {
    pub total_proposals: usize,
    pub active_proposals: usize,
    pub passed_proposals: usize,
    pub rejected_proposals: usize,
    pub total_votes: usize,
    pub config: ConfigResponse,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub min_deposit: String,
    pub voting_period_days: f64,
    pub quorum_percentage: u8,
    pub pass_threshold: u8,
    pub veto_threshold: u8,
    pub execution_delay_days: f64,
}

// ============================================================================
// API Handlers
// ============================================================================

/// Get governance statistics
pub async fn get_governance_stats(governance: web::Data<GovernanceState>) -> impl Responder {
    let gov = governance.read().await;
    let stats = gov.get_stats();

    HttpResponse::Ok().json(GovernanceStatsResponse {
        total_proposals: stats.total_proposals,
        active_proposals: stats.active_proposals,
        passed_proposals: stats.passed_proposals,
        rejected_proposals: stats.rejected_proposals,
        total_votes: stats.total_votes,
        config: ConfigResponse {
            min_deposit: stats.config.min_deposit.to_string(),
            voting_period_days: stats.config.voting_period as f64 / 86400.0,
            quorum_percentage: stats.config.quorum_percentage,
            pass_threshold: stats.config.pass_threshold,
            veto_threshold: stats.config.veto_threshold,
            execution_delay_days: stats.config.execution_delay as f64 / 86400.0,
        },
    })
}

/// Get all proposals
pub async fn get_proposals(governance: web::Data<GovernanceState>) -> impl Responder {
    let gov = governance.read().await;
    let proposals: Vec<ProposalResponse> = gov
        .get_all_proposals()
        .iter()
        .map(|p| ProposalResponse::from(*p))
        .collect();

    HttpResponse::Ok().json(ProposalListResponse {
        total: proposals.len(),
        proposals,
    })
}

/// Get active proposals
pub async fn get_active_proposals(governance: web::Data<GovernanceState>) -> impl Responder {
    let gov = governance.read().await;
    let proposals: Vec<ProposalResponse> = gov
        .get_active_proposals()
        .iter()
        .map(|p| ProposalResponse::from(*p))
        .collect();

    HttpResponse::Ok().json(ProposalListResponse {
        total: proposals.len(),
        proposals,
    })
}

/// Get proposal by ID
pub async fn get_proposal(
    governance: web::Data<GovernanceState>,
    path: web::Path<u64>,
) -> impl Responder {
    let proposal_id = path.into_inner();
    let gov = governance.read().await;

    match gov.get_proposal(proposal_id) {
        Some(proposal) => HttpResponse::Ok().json(ProposalResponse::from(proposal)),
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Proposal not found"
        })),
    }
}

/// Create a new proposal (requires signature authentication)
/// 
/// Request body must be wrapped in SignedRequest with auth data
pub async fn create_proposal(
    governance: web::Data<GovernanceState>,
    body: web::Json<SignedRequest<CreateProposalRequest>>,
) -> impl Responder {
    // Verify signature - proposer must sign the request
    let message = serde_json::to_vec(&body.data).unwrap_or_default();
    match verify_signed_request(
        &body.auth,
        &message,
        Some(&body.data.proposer),
        300, // 5 minute expiry
    ) {
        Ok(_) => {},
        Err(response) => return response,
    };

    let mut gov = governance.write().await;
    let body = &body.data;

    let initial_deposit: u128 = body.initial_deposit.parse().unwrap_or(0);
    let proposal_type: ProposalType = body.proposal_type.clone().into();

    match gov.create_proposal(
        body.proposer.clone(),
        body.title.clone(),
        body.description.clone(),
        proposal_type,
        initial_deposit,
    ) {
        Ok(proposal_id) => {
            let proposal = gov.get_proposal(proposal_id).unwrap();
            let status = match proposal.status {
                ProposalStatus::VotingPeriod => "voting_period",
                ProposalStatus::DepositPeriod => "deposit_period",
                _ => "unknown",
            };

            HttpResponse::Ok().json(CreateProposalResponse {
                success: true,
                proposal_id,
                status: status.to_string(),
                message: format!("Proposal {} created successfully", proposal_id),
            })
        }
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Add deposit to a proposal
pub async fn add_deposit(
    governance: web::Data<GovernanceState>,
    path: web::Path<u64>,
    body: web::Json<DepositRequest>,
) -> impl Responder {
    let proposal_id = path.into_inner();
    let mut gov = governance.write().await;

    let amount: u128 = body.amount.parse().unwrap_or(0);

    match gov.add_deposit(body.depositor.clone(), proposal_id, amount) {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Deposit added successfully"
        })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Vote on a proposal (requires signature authentication)
/// 
/// Request body must be wrapped in SignedRequest with auth data
pub async fn vote_on_proposal(
    governance: web::Data<GovernanceState>,
    path: web::Path<u64>,
    body: web::Json<SignedRequest<VoteRequest>>,
) -> impl Responder {
    // Verify signature - voter must sign the request
    let message = serde_json::to_vec(&body.data).unwrap_or_default();
    match verify_signed_request(
        &body.auth,
        &message,
        Some(&body.data.voter),
        300, // 5 minute expiry
    ) {
        Ok(_) => {},
        Err(response) => return response,
    };

    let proposal_id = path.into_inner();
    let mut gov = governance.write().await;
    let body = &body.data;

    let option = match body.option.to_lowercase().as_str() {
        "yes" => VoteOption::Yes,
        "no" => VoteOption::No,
        "abstain" => VoteOption::Abstain,
        "no_with_veto" | "veto" => VoteOption::NoWithVeto,
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid vote option. Use: yes, no, abstain, no_with_veto"
            }));
        }
    };

    let voting_power: u128 = body.voting_power.parse().unwrap_or(0);

    match gov.vote(body.voter.clone(), proposal_id, option, voting_power) {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Vote cast successfully"
        })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Get votes for a proposal
pub async fn get_proposal_votes(
    governance: web::Data<GovernanceState>,
    path: web::Path<u64>,
) -> impl Responder {
    let proposal_id = path.into_inner();
    let gov = governance.read().await;

    match gov.get_proposal(proposal_id) {
        Some(proposal) => {
            let votes: Vec<serde_json::Value> = proposal
                .votes
                .values()
                .map(|v| {
                    serde_json::json!({
                        "voter": v.voter,
                        "option": format!("{:?}", v.option).to_lowercase(),
                        "voting_power": v.voting_power.to_string(),
                        "timestamp": v.timestamp
                    })
                })
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "proposal_id": proposal_id,
                "votes": votes,
                "total": votes.len()
            }))
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Proposal not found"
        })),
    }
}

/// Demo proposal creation request (no signature required)
#[derive(Debug, Deserialize)]
pub struct DemoCreateProposalRequest {
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalTypeRequest,
    pub initial_deposit: String,
}

/// Create a new proposal in demo mode (no signature required)
/// This endpoint is for demonstration purposes only
pub async fn create_proposal_demo(
    governance: web::Data<GovernanceState>,
    body: web::Json<DemoCreateProposalRequest>,
) -> impl Responder {
    let mut gov = governance.write().await;

    let initial_deposit: u128 = body.initial_deposit.parse().unwrap_or(0);
    let proposal_type: ProposalType = body.proposal_type.clone().into();

    match gov.create_proposal(
        body.proposer.clone(),
        body.title.clone(),
        body.description.clone(),
        proposal_type,
        initial_deposit,
    ) {
        Ok(proposal_id) => {
            let proposal = gov.get_proposal(proposal_id).unwrap();
            let status = match proposal.status {
                ProposalStatus::VotingPeriod => "voting_period",
                ProposalStatus::DepositPeriod => "deposit_period",
                _ => "unknown",
            };

            HttpResponse::Ok().json(CreateProposalResponse {
                success: true,
                proposal_id,
                status: status.to_string(),
                message: format!("Proposal {} created successfully (demo mode)", proposal_id),
            })
        }
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Demo vote request (no signature required)
#[derive(Debug, Deserialize)]
pub struct DemoVoteRequest {
    pub voter: String,
    pub option: String,
    pub voting_power: String,
}

/// Vote on a proposal in demo mode (no signature required)
pub async fn vote_on_proposal_demo(
    governance: web::Data<GovernanceState>,
    path: web::Path<u64>,
    body: web::Json<DemoVoteRequest>,
) -> impl Responder {
    let proposal_id = path.into_inner();
    let mut gov = governance.write().await;

    let option = match body.option.to_lowercase().as_str() {
        "yes" => VoteOption::Yes,
        "no" => VoteOption::No,
        "abstain" => VoteOption::Abstain,
        "no_with_veto" | "veto" => VoteOption::NoWithVeto,
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid vote option. Use: yes, no, abstain, or no_with_veto"
            }))
        }
    };

    let voting_power: u128 = body.voting_power.parse().unwrap_or(1000000000000000000); // Default 1 EDGE

    match gov.vote(body.voter.clone(), proposal_id, option, voting_power) {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Vote recorded successfully (demo mode)"
        })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Configure governance routes
pub fn configure_governance_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/governance")
            .route("/stats", web::get().to(get_governance_stats))
            .route("/proposals", web::get().to(get_proposals))
            .route("/proposals/active", web::get().to(get_active_proposals))
            .route("/proposals", web::post().to(create_proposal))
            .route("/proposals/demo", web::post().to(create_proposal_demo))
            .route("/proposals/{id}", web::get().to(get_proposal))
            .route("/proposals/{id}/deposit", web::post().to(add_deposit))
            .route("/proposals/{id}/vote", web::post().to(vote_on_proposal))
            .route("/proposals/{id}/vote/demo", web::post().to(vote_on_proposal_demo))
            .route("/proposals/{id}/votes", web::get().to(get_proposal_votes)),
    );
}
