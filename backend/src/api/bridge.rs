//! Bridge API for EdgeAI Blockchain
//!
//! Provides cross-chain bridge functionality between EdgeAI native chain
//! and EVM-compatible chains (BSC, Ethereum, etc.).
//!
//! ## Architecture
//! - EdgeAI EDGE (native) ←→ $EDGE (ERC-20 on target chains)
//! - Semi-automatic: lock/release on-chain, manual EVM transfer by admin
//! - Multi-chain ready: target_chain parameter supports future expansion
//!
//! ## Flow (EDGE → BSC)
//! 1. User calls POST /api/bridge/lock with amount + BSC address
//! 2. Backend locks EDGE in bridge_vault account, creates pending request
//! 3. Admin sees pending request, manually transfers EDGEAI on BSC
//! 4. Admin calls POST /api/bridge/complete with BSC tx hash
//!
//! ## Flow (BSC → EDGE)
//! 1. User transfers EDGEAI to admin wallet on BSC
//! 2. Admin calls POST /api/bridge/release with BSC tx hash + user's EdgeAI address
//! 3. Backend releases EDGE from bridge_vault to user

#![allow(dead_code)]

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn, error};
use chrono::Utc;

use crate::blockchain::Blockchain;

// ============ Constants ============

/// The bridge vault account address on EdgeAI chain
const BRIDGE_VAULT: &str = "edgeai_bridge_vault";

/// Minimum bridge amount (prevent dust attacks)
const MIN_BRIDGE_AMOUNT: u64 = 100;

/// Maximum bridge amount per request
const MAX_BRIDGE_AMOUNT: u64 = 10_000_000;

/// Bridge fee rate (0.1%)
const BRIDGE_FEE_BPS: u64 = 10; // basis points (10 = 0.1%)

/// Admin API key header name
const ADMIN_KEY_HEADER: &str = "X-Bridge-Admin-Key";

// ============ Supported Chains ============

/// Supported target chains for bridging
fn is_supported_chain(chain: &str) -> bool {
    matches!(chain, "bsc" | "ethereum" | "base" | "arbitrum" | "solana")
}

/// Get chain display name
fn chain_display_name(chain: &str) -> &str {
    match chain {
        "bsc" => "BNB Smart Chain",
        "ethereum" => "Ethereum",
        "base" => "Base",
        "arbitrum" => "Arbitrum",
        "solana" => "Solana",
        _ => "Unknown",
    }
}

// ============ Request/Response Types ============

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> BridgeResponse<T> {
    pub fn success(data: T) -> HttpResponse {
        HttpResponse::Ok().json(BridgeResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    pub fn error(status: actix_web::http::StatusCode, msg: &str) -> HttpResponse {
        HttpResponse::build(status).json(BridgeResponse::<()> {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        })
    }
}

/// Request to lock EDGE and bridge to EVM chain
#[derive(Debug, Deserialize)]
pub struct BridgeLockRequest {
    /// EdgeAI chain address (sender)
    pub edge_address: String,
    /// EVM wallet address (recipient on target chain)
    pub evm_address: String,
    /// Amount of EDGE to bridge
    pub amount: u64,
    /// Target chain (default: "bsc")
    pub target_chain: Option<String>,
}

/// Request to complete a bridge (admin marks as done after manual EVM transfer)
#[derive(Debug, Deserialize)]
pub struct BridgeCompleteRequest {
    /// The bridge request ID to complete
    pub request_id: String,
    /// BSC/EVM transaction hash as proof
    pub tx_hash_evm: String,
    /// Optional admin note
    pub note: Option<String>,
}

/// Request to release EDGE back from bridge (BSC → EdgeAI direction)
#[derive(Debug, Deserialize)]
pub struct BridgeReleaseRequest {
    /// EdgeAI chain address (recipient)
    pub edge_address: String,
    /// EVM wallet address (sender on BSC)
    pub evm_address: String,
    /// Amount of EDGE to release
    pub amount: u64,
    /// BSC/EVM transaction hash as proof of EDGEAI transfer
    pub tx_hash_evm: String,
    /// Source chain (default: "bsc")
    pub source_chain: Option<String>,
}

/// Request to cancel a pending bridge request
#[derive(Debug, Deserialize)]
pub struct BridgeCancelRequest {
    /// The bridge request ID to cancel
    pub request_id: String,
    /// Reason for cancellation
    pub reason: Option<String>,
}

/// Query parameters for bridge history
#[derive(Debug, Deserialize)]
pub struct BridgeHistoryQuery {
    /// Filter by EdgeAI address
    pub edge_address: Option<String>,
    /// Filter by EVM address
    pub evm_address: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Limit results (default 50)
    pub limit: Option<u32>,
}

/// Bridge request record
#[derive(Debug, Serialize, Clone)]
pub struct BridgeRequest {
    pub request_id: String,
    pub direction: String,
    pub target_chain: String,
    pub edge_address: String,
    pub evm_address: String,
    pub amount: u64,
    pub fee: u64,
    pub status: String,
    pub tx_hash_edge: Option<String>,
    pub tx_hash_evm: Option<String>,
    pub admin_note: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Bridge statistics
#[derive(Debug, Serialize)]
pub struct BridgeStats {
    pub total_requests: u64,
    pub pending_requests: u64,
    pub completed_requests: u64,
    pub total_volume_locked: u64,
    pub total_volume_released: u64,
    pub total_fees_collected: u64,
    pub vault_balance: u64,
    pub supported_chains: Vec<ChainInfo>,
}

/// Chain info for supported chains
#[derive(Debug, Serialize)]
pub struct ChainInfo {
    pub chain_id: String,
    pub name: String,
    pub token_symbol: String,
    pub token_address: Option<String>,
    pub active: bool,
}

/// Bridge state shared across handlers
pub struct BridgeState {
    pub blockchain: Arc<RwLock<Blockchain>>,
    admin_key: String,
}

impl BridgeState {
    pub fn new(blockchain: Arc<RwLock<Blockchain>>) -> Self {
        let admin_key = std::env::var("BRIDGE_ADMIN_KEY")
            .unwrap_or_else(|_| "edgeai-bridge-admin-2026".to_string());
        BridgeState {
            blockchain,
            admin_key,
        }
    }

    fn verify_admin(&self, req: &actix_web::HttpRequest) -> bool {
        req.headers()
            .get(ADMIN_KEY_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|key| key == self.admin_key)
            .unwrap_or(false)
    }
}

// ============ Handlers ============

/// POST /api/bridge/lock - Lock EDGE on native chain to bridge out
pub async fn bridge_lock(
    state: web::Data<BridgeState>,
    body: web::Json<BridgeLockRequest>,
) -> impl Responder {
    let target_chain = body.target_chain.clone().unwrap_or_else(|| "bsc".to_string());

    // Validate target chain
    if !is_supported_chain(&target_chain) {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::BAD_REQUEST,
            &format!("Unsupported target chain: {}. Supported: bsc, ethereum, base, arbitrum, solana", target_chain),
        );
    }

    // Validate EVM address format (0x + 40 hex chars)
    if !body.evm_address.starts_with("0x") || body.evm_address.len() != 42 {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::BAD_REQUEST,
            "Invalid EVM address format. Must be 0x followed by 40 hex characters.",
        );
    }

    // Validate amount
    if body.amount < MIN_BRIDGE_AMOUNT {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::BAD_REQUEST,
            &format!("Amount too small. Minimum bridge amount is {} EDGE.", MIN_BRIDGE_AMOUNT),
        );
    }
    if body.amount > MAX_BRIDGE_AMOUNT {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::BAD_REQUEST,
            &format!("Amount too large. Maximum bridge amount is {} EDGE per request.", MAX_BRIDGE_AMOUNT),
        );
    }

    // Calculate fee
    let fee = body.amount * BRIDGE_FEE_BPS / 10_000;
    let net_amount = body.amount - fee;

    // Lock EDGE: transfer from user to bridge_vault
    let request_id = format!("br_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_string());
    let tx_hash_edge: String;

    {
        let mut chain = state.blockchain.write().await;

        // Check user balance
        let balance = chain.get_balance(&body.edge_address);
        if balance < body.amount {
            return BridgeResponse::<()>::error(
                actix_web::http::StatusCode::BAD_REQUEST,
                &format!("Insufficient balance. Available: {} EDGE, Required: {} EDGE.", balance, body.amount),
            );
        }

        // Execute internal transfer: user → bridge_vault
        match chain.internal_transfer(&body.edge_address, BRIDGE_VAULT, body.amount) {
            Ok(hash) => {
                tx_hash_edge = hash;
                info!("Bridge lock: {} EDGE from {} to vault (fee: {}), request: {}",
                    body.amount, body.edge_address, fee, request_id);
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to lock EDGE: {}", e),
                );
            }
        }

        // Save bridge request to OceanBase
        if let Some(ref ob) = chain.get_ob_storage() {
            let now = Utc::now();
            if let Err(e) = sqlx::query(
                "INSERT INTO bridge_requests (request_id, direction, target_chain, edge_address, evm_address, \
                 amount, fee, status, tx_hash_edge, created_at) \
                 VALUES (?, 'edge_to_bsc', ?, ?, ?, ?, ?, 'pending', ?, ?)"
            )
            .bind(&request_id)
            .bind(&target_chain)
            .bind(&body.edge_address)
            .bind(&body.evm_address)
            .bind(body.amount)
            .bind(fee)
            .bind(&tx_hash_edge)
            .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
            .execute(ob.pool())
            .await
            {
                warn!("Failed to save bridge request to OceanBase: {}", e);
            }
        }
    }

    let response = BridgeRequest {
        request_id: request_id.clone(),
        direction: "edge_to_bsc".to_string(),
        target_chain: target_chain.clone(),
        edge_address: body.edge_address.clone(),
        evm_address: body.evm_address.clone(),
        amount: body.amount,
        fee,
        status: "pending".to_string(),
        tx_hash_edge: Some(tx_hash_edge),
        tx_hash_evm: None,
        admin_note: None,
        created_at: Utc::now().to_rfc3339(),
        completed_at: None,
    };

    BridgeResponse::success(response)
}

/// POST /api/bridge/complete - Admin marks a bridge request as completed (after manual EVM transfer)
pub async fn bridge_complete(
    state: web::Data<BridgeState>,
    req: actix_web::HttpRequest,
    body: web::Json<BridgeCompleteRequest>,
) -> impl Responder {
    // Verify admin
    if !state.verify_admin(&req) {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Invalid admin key.",
        );
    }

    let chain = state.blockchain.read().await;
    if let Some(ref ob) = chain.get_ob_storage() {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        match sqlx::query(
            "UPDATE bridge_requests SET status = 'completed', tx_hash_evm = ?, admin_note = ?, \
             completed_at = ? WHERE request_id = ? AND status = 'pending'"
        )
        .bind(&body.tx_hash_evm)
        .bind(&body.note)
        .bind(&now)
        .bind(&body.request_id)
        .execute(ob.pool())
        .await
        {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    return BridgeResponse::<()>::error(
                        actix_web::http::StatusCode::NOT_FOUND,
                        "Bridge request not found or not in pending status.",
                    );
                }
                info!("Bridge request {} completed with EVM tx: {}", body.request_id, body.tx_hash_evm);
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error: {}", e),
                );
            }
        }
    } else {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
            "Database not available.",
        );
    }

    BridgeResponse::success(serde_json::json!({
        "request_id": body.request_id,
        "status": "completed",
        "tx_hash_evm": body.tx_hash_evm,
    }))
}

/// POST /api/bridge/release - Admin releases EDGE back to user (BSC → EdgeAI direction)
pub async fn bridge_release(
    state: web::Data<BridgeState>,
    req: actix_web::HttpRequest,
    body: web::Json<BridgeReleaseRequest>,
) -> impl Responder {
    // Verify admin
    if !state.verify_admin(&req) {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Invalid admin key.",
        );
    }

    let source_chain = body.source_chain.clone().unwrap_or_else(|| "bsc".to_string());

    // Validate amount
    if body.amount < MIN_BRIDGE_AMOUNT {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::BAD_REQUEST,
            &format!("Amount too small. Minimum is {} EDGE.", MIN_BRIDGE_AMOUNT),
        );
    }

    let fee = body.amount * BRIDGE_FEE_BPS / 10_000;
    let net_amount = body.amount - fee;
    let request_id = format!("br_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..16].to_string());
    let tx_hash_edge: String;

    {
        let mut chain = state.blockchain.write().await;

        // Check vault balance
        let vault_balance = chain.get_balance(BRIDGE_VAULT);
        if vault_balance < net_amount {
            return BridgeResponse::<()>::error(
                actix_web::http::StatusCode::BAD_REQUEST,
                &format!("Insufficient vault balance. Available: {} EDGE, Required: {} EDGE.", vault_balance, net_amount),
            );
        }

        // Execute internal transfer: bridge_vault → user
        match chain.internal_transfer(BRIDGE_VAULT, &body.edge_address, net_amount) {
            Ok(hash) => {
                tx_hash_edge = hash;
                info!("Bridge release: {} EDGE to {} from vault (fee: {}), request: {}",
                    net_amount, body.edge_address, fee, request_id);
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to release EDGE: {}", e),
                );
            }
        }

        // Save bridge request to OceanBase
        if let Some(ref ob) = chain.get_ob_storage() {
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            if let Err(e) = sqlx::query(
                "INSERT INTO bridge_requests (request_id, direction, target_chain, edge_address, evm_address, \
                 amount, fee, status, tx_hash_edge, tx_hash_evm, completed_at, created_at) \
                 VALUES (?, 'bsc_to_edge', ?, ?, ?, ?, ?, 'completed', ?, ?, ?, ?)"
            )
            .bind(&request_id)
            .bind(&source_chain)
            .bind(&body.edge_address)
            .bind(&body.evm_address)
            .bind(body.amount)
            .bind(fee)
            .bind(&tx_hash_edge)
            .bind(&body.tx_hash_evm)
            .bind(&now)
            .bind(&now)
            .execute(ob.pool())
            .await
            {
                warn!("Failed to save bridge release to OceanBase: {}", e);
            }
        }
    }

    BridgeResponse::success(serde_json::json!({
        "request_id": request_id,
        "direction": "bsc_to_edge",
        "source_chain": source_chain,
        "edge_address": body.edge_address,
        "amount": body.amount,
        "fee": fee,
        "net_amount": net_amount,
        "status": "completed",
        "tx_hash_edge": tx_hash_edge,
        "tx_hash_evm": body.tx_hash_evm,
    }))
}

/// POST /api/bridge/cancel - Cancel a pending bridge request (refund EDGE)
pub async fn bridge_cancel(
    state: web::Data<BridgeState>,
    req: actix_web::HttpRequest,
    body: web::Json<BridgeCancelRequest>,
) -> impl Responder {
    // Verify admin
    if !state.verify_admin(&req) {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Invalid admin key.",
        );
    }

    let mut chain = state.blockchain.write().await;

    // Look up the request from OceanBase
    let request_info: Option<(String, u64)> = if let Some(ref ob) = chain.get_ob_storage() {
        match sqlx::query_as::<_, (String, u64)>(
            "SELECT edge_address, amount FROM bridge_requests WHERE request_id = ? AND status = 'pending'"
        )
        .bind(&body.request_id)
        .fetch_optional(ob.pool())
        .await
        {
            Ok(row) => row,
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error: {}", e),
                );
            }
        }
    } else {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
            "Database not available.",
        );
    };

    let (edge_address, amount) = match request_info {
        Some(info) => info,
        None => {
            return BridgeResponse::<()>::error(
                actix_web::http::StatusCode::NOT_FOUND,
                "Bridge request not found or not in pending status.",
            );
        }
    };

    // Refund: transfer from bridge_vault back to user
    match chain.internal_transfer(BRIDGE_VAULT, &edge_address, amount) {
        Ok(_) => {
            info!("Bridge cancel: refunded {} EDGE to {}, request: {}", amount, edge_address, body.request_id);
        }
        Err(e) => {
            return BridgeResponse::<()>::error(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to refund EDGE: {}", e),
            );
        }
    }

    // Update status in OceanBase
    if let Some(ref ob) = chain.get_ob_storage() {
        let note = body.reason.clone().unwrap_or_else(|| "Cancelled by admin".to_string());
        let _ = sqlx::query(
            "UPDATE bridge_requests SET status = 'cancelled', admin_note = ? WHERE request_id = ?"
        )
        .bind(&note)
        .bind(&body.request_id)
        .execute(ob.pool())
        .await;
    }

    BridgeResponse::success(serde_json::json!({
        "request_id": body.request_id,
        "status": "cancelled",
        "refunded_amount": amount,
        "edge_address": edge_address,
    }))
}

/// GET /api/bridge/pending - Get all pending bridge requests (admin)
pub async fn bridge_pending(
    state: web::Data<BridgeState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    // Verify admin
    if !state.verify_admin(&req) {
        return BridgeResponse::<()>::error(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "Invalid admin key.",
        );
    }

    let chain = state.blockchain.read().await;
    if let Some(ref ob) = chain.get_ob_storage() {
        match sqlx::query_as::<_, (String, String, String, String, String, u64, u64, String, Option<String>, String)>(
            "SELECT request_id, direction, target_chain, edge_address, evm_address, amount, fee, status, tx_hash_edge, created_at \
             FROM bridge_requests WHERE status = 'pending' ORDER BY created_at ASC"
        )
        .fetch_all(ob.pool())
        .await
        {
            Ok(rows) => {
                let requests: Vec<serde_json::Value> = rows.iter().map(|r| {
                    serde_json::json!({
                        "request_id": r.0,
                        "direction": r.1,
                        "target_chain": r.2,
                        "edge_address": r.3,
                        "evm_address": r.4,
                        "amount": r.5,
                        "fee": r.6,
                        "status": r.7,
                        "tx_hash_edge": r.8,
                        "created_at": r.9,
                    })
                }).collect();

                return BridgeResponse::success(serde_json::json!({
                    "pending_count": requests.len(),
                    "requests": requests,
                }));
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error: {}", e),
                );
            }
        }
    }

    BridgeResponse::<()>::error(
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        "Database not available.",
    )
}

/// GET /api/bridge/history - Get bridge request history
pub async fn bridge_history(
    state: web::Data<BridgeState>,
    query: web::Query<BridgeHistoryQuery>,
) -> impl Responder {
    let chain = state.blockchain.read().await;
    if let Some(ref ob) = chain.get_ob_storage() {
        let limit = query.limit.unwrap_or(50).min(200) as i64;

        // Build dynamic query based on filters
        let (where_clause, bind_values) = build_history_filter(&query);
        let sql = format!(
            "SELECT request_id, direction, target_chain, edge_address, evm_address, \
             amount, fee, status, tx_hash_edge, tx_hash_evm, created_at, completed_at \
             FROM bridge_requests {} ORDER BY created_at DESC LIMIT {}",
            where_clause, limit
        );

        // Use a simpler approach - fetch with dynamic binds
        let mut q = sqlx::query(&sql);
        for val in &bind_values {
            q = q.bind(val);
        }

        match q.fetch_all(ob.pool()).await {
            Ok(rows) => {
                use sqlx::Row;
                let requests: Vec<serde_json::Value> = rows.iter().map(|row| {
                    serde_json::json!({
                        "request_id": row.get::<String, _>("request_id"),
                        "direction": row.get::<String, _>("direction"),
                        "target_chain": row.get::<String, _>("target_chain"),
                        "edge_address": row.get::<String, _>("edge_address"),
                        "evm_address": row.get::<String, _>("evm_address"),
                        "amount": row.get::<u64, _>("amount"),
                        "fee": row.get::<u64, _>("fee"),
                        "status": row.get::<String, _>("status"),
                        "tx_hash_edge": row.get::<Option<String>, _>("tx_hash_edge"),
                        "tx_hash_evm": row.get::<Option<String>, _>("tx_hash_evm"),
                        "created_at": row.get::<String, _>("created_at"),
                        "completed_at": row.get::<Option<String>, _>("completed_at"),
                    })
                }).collect();

                return BridgeResponse::success(serde_json::json!({
                    "total": requests.len(),
                    "requests": requests,
                }));
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error: {}", e),
                );
            }
        }
    }

    BridgeResponse::<()>::error(
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        "Database not available.",
    )
}

/// GET /api/bridge/stats - Get bridge statistics
pub async fn bridge_stats(
    state: web::Data<BridgeState>,
) -> impl Responder {
    let chain = state.blockchain.read().await;
    let vault_balance = chain.get_balance(BRIDGE_VAULT);

    let mut stats = BridgeStats {
        total_requests: 0,
        pending_requests: 0,
        completed_requests: 0,
        total_volume_locked: 0,
        total_volume_released: 0,
        total_fees_collected: 0,
        vault_balance,
        supported_chains: vec![
            ChainInfo {
                chain_id: "bsc".to_string(),
                name: "BNB Smart Chain".to_string(),
                token_symbol: "EDGEAI".to_string(),
                token_address: Some("0x276b792D11B9e3712FE6A78A460a0DEb416baB0A".to_string()),
                active: true,
            },
            ChainInfo {
                chain_id: "ethereum".to_string(),
                name: "Ethereum".to_string(),
                token_symbol: "EDGE".to_string(),
                token_address: None,
                active: false,
            },
            ChainInfo {
                chain_id: "solana".to_string(),
                name: "Solana".to_string(),
                token_symbol: "EDGE".to_string(),
                token_address: None,
                active: false,
            },
        ],
    };

    if let Some(ref ob) = chain.get_ob_storage() {
        // Get aggregate stats from OceanBase
        use sqlx::Row;
        if let Ok(row) = sqlx::query(
            "SELECT \
                COUNT(*) as total, \
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending, \
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed, \
                COALESCE(SUM(CASE WHEN direction = 'edge_to_bsc' AND status = 'completed' THEN amount ELSE 0 END), 0) as vol_locked, \
                COALESCE(SUM(CASE WHEN direction = 'bsc_to_edge' AND status = 'completed' THEN amount ELSE 0 END), 0) as vol_released, \
                COALESCE(SUM(CASE WHEN status = 'completed' THEN fee ELSE 0 END), 0) as fees \
             FROM bridge_requests"
        )
        .fetch_one(ob.pool())
        .await
        {
            stats.total_requests = row.try_get::<i64, _>("total").unwrap_or(0) as u64;
            stats.pending_requests = row.try_get::<i64, _>("pending").unwrap_or(0) as u64;
            stats.completed_requests = row.try_get::<i64, _>("completed").unwrap_or(0) as u64;
            stats.total_volume_locked = row.try_get::<i64, _>("vol_locked").unwrap_or(0) as u64;
            stats.total_volume_released = row.try_get::<i64, _>("vol_released").unwrap_or(0) as u64;
            stats.total_fees_collected = row.try_get::<i64, _>("fees").unwrap_or(0) as u64;
        }
    }

    BridgeResponse::success(stats)
}

/// GET /api/bridge/request/{request_id} - Get a specific bridge request
pub async fn bridge_get_request(
    state: web::Data<BridgeState>,
    path: web::Path<String>,
) -> impl Responder {
    let request_id = path.into_inner();
    let chain = state.blockchain.read().await;

    if let Some(ref ob) = chain.get_ob_storage() {
        use sqlx::Row;
        match sqlx::query(
            "SELECT request_id, direction, target_chain, edge_address, evm_address, \
             amount, fee, status, tx_hash_edge, tx_hash_evm, admin_note, created_at, completed_at \
             FROM bridge_requests WHERE request_id = ?"
        )
        .bind(&request_id)
        .fetch_optional(ob.pool())
        .await
        {
            Ok(Some(row)) => {
                let request = serde_json::json!({
                    "request_id": row.get::<String, _>("request_id"),
                    "direction": row.get::<String, _>("direction"),
                    "target_chain": row.get::<String, _>("target_chain"),
                    "edge_address": row.get::<String, _>("edge_address"),
                    "evm_address": row.get::<String, _>("evm_address"),
                    "amount": row.get::<u64, _>("amount"),
                    "fee": row.get::<u64, _>("fee"),
                    "status": row.get::<String, _>("status"),
                    "tx_hash_edge": row.get::<Option<String>, _>("tx_hash_edge"),
                    "tx_hash_evm": row.get::<Option<String>, _>("tx_hash_evm"),
                    "admin_note": row.get::<Option<String>, _>("admin_note"),
                    "created_at": row.get::<String, _>("created_at"),
                    "completed_at": row.get::<Option<String>, _>("completed_at"),
                });
                return BridgeResponse::success(request);
            }
            Ok(None) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::NOT_FOUND,
                    "Bridge request not found.",
                );
            }
            Err(e) => {
                return BridgeResponse::<()>::error(
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Database error: {}", e),
                );
            }
        }
    }

    BridgeResponse::<()>::error(
        actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
        "Database not available.",
    )
}

// ============ Helper Functions ============

fn build_history_filter(query: &BridgeHistoryQuery) -> (String, Vec<String>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref addr) = query.edge_address {
        conditions.push("edge_address = ?".to_string());
        values.push(addr.clone());
    }
    if let Some(ref addr) = query.evm_address {
        conditions.push("evm_address = ?".to_string());
        values.push(addr.clone());
    }
    if let Some(ref status) = query.status {
        conditions.push("status = ?".to_string());
        values.push(status.clone());
    }

    if conditions.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", conditions.join(" AND ")), values)
    }
}

// ============ Route Configuration ============

/// Configure bridge API routes
pub fn configure_bridge_routes(cfg: &mut web::ServiceConfig, bridge_state: web::Data<BridgeState>) {
    cfg.app_data(bridge_state)
        .route("/api/bridge/lock", web::post().to(bridge_lock))
        .route("/api/bridge/complete", web::post().to(bridge_complete))
        .route("/api/bridge/release", web::post().to(bridge_release))
        .route("/api/bridge/cancel", web::post().to(bridge_cancel))
        .route("/api/bridge/pending", web::get().to(bridge_pending))
        .route("/api/bridge/history", web::get().to(bridge_history))
        .route("/api/bridge/stats", web::get().to(bridge_stats))
        .route("/api/bridge/request/{request_id}", web::get().to(bridge_get_request));
}
