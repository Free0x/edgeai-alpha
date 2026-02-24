//! REST API endpoints for EdgeAI Blockchain
//!
//! This module provides HTTP endpoints for blockchain operations, transactions,
//! accounts, mining, consensus, marketplace, and network management.

#![allow(dead_code)]

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use log::info;

use crate::blockchain::{Blockchain, Transaction, Block};
use crate::consensus::{PoIEConsensus};
use crate::data_market::{DataMarketplace, DataListing, DataCategory, SortBy};
use crate::network::NetworkManager;

// Re-export Validator for use in handlers
use crate::consensus::poie::Validator;

/// Application state shared across handlers
pub struct AppState {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub consensus: Arc<RwLock<PoIEConsensus>>,
    pub marketplace: Arc<RwLock<DataMarketplace>>,
    pub network: Arc<NetworkManager>,
}

// ============ Request/Response Types ============

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(msg: &str) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct DataContributionRequest {
    pub sender: String,
    pub data: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListDataRequest {
    pub owner: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: u64,
    pub data: String,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseDataRequest {
    pub buyer: String,
    pub data_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct FaucetRequest {
    pub address: String,
    #[serde(default)]
    pub amount: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MineBlockRequest {
    pub validator: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterValidatorRequest {
    pub address: String,
    pub stake: u64,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    pub min_price: Option<u64>,
    pub max_price: Option<u64>,
    pub min_quality: Option<f64>,
    pub sort_by: Option<String>,
    pub limit: Option<usize>,
}

// ============ Blockchain Endpoints ============

/// Get blockchain info
pub async fn get_chain_info(data: web::Data<AppState>) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    let stats = blockchain.get_stats();
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Get all blocks
pub async fn get_blocks(
    data: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    let start = query.offset.unwrap_or(0) as usize;
    let limit = query.limit.unwrap_or(10) as usize;
    
    let blocks: Vec<&Block> = blockchain.chain.iter()
        .skip(start)
        .take(limit)
        .collect();
    
    HttpResponse::Ok().json(ApiResponse::success(blocks))
}

/// Get block by index
pub async fn get_block(
    data: web::Data<AppState>,
    path: web::Path<u64>,
) -> impl Responder {
    let index = path.into_inner();
    let blockchain = data.blockchain.read().await;
    
    match blockchain.get_block(index) {
        Some(block) => HttpResponse::Ok().json(ApiResponse::success(block)),
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Block not found")),
    }
}

/// Get block by hash
pub async fn get_block_by_hash(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let hash = path.into_inner();
    let blockchain = data.blockchain.read().await;
    
    match blockchain.get_block_by_hash(&hash) {
        Some(block) => HttpResponse::Ok().json(ApiResponse::success(block)),
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Block not found")),
    }
}

/// Get latest block
pub async fn get_latest_block(data: web::Data<AppState>) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    let block = blockchain.latest_block();
    HttpResponse::Ok().json(ApiResponse::success(block))
}

// ============ Transaction Endpoints ============

/// Get transaction by hash
pub async fn get_transaction(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let hash = path.into_inner();
    let blockchain = data.blockchain.read().await;
    
    match blockchain.get_transaction(&hash) {
        Some(tx) => HttpResponse::Ok().json(ApiResponse::success(tx)),
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Transaction not found")),
    }
}

/// Get recent transactions (from in-memory blocks)
pub async fn get_recent_transactions(
    data: web::Data<AppState>,
    query: web::Query<PaginationQuery>,
) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    let offset = query.offset.unwrap_or(0) as usize;
    let limit = query.limit.unwrap_or(20) as usize;
    
    // Collect transactions from in-memory blocks (most recent first)
    let mut all_txs: Vec<&Transaction> = Vec::new();
    for block in blockchain.chain.iter().rev() {
        for tx in block.transactions.iter().rev() {
            all_txs.push(tx);
        }
    }
    
    let total = all_txs.len();
    let txs: Vec<&Transaction> = all_txs.into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    #[derive(Serialize)]
    struct TransactionsResponse<'a> {
        transactions: Vec<&'a Transaction>,
        total: usize,
        offset: usize,
        limit: usize,
    }
    
    HttpResponse::Ok().json(TransactionsResponse {
        transactions: txs,
        total,
        offset,
        limit,
    })
}

/// Get pending transactions
pub async fn get_pending_transactions(data: web::Data<AppState>) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    HttpResponse::Ok().json(ApiResponse::success(&blockchain.pending_transactions))
}

/// Create transfer transaction
pub async fn create_transfer(
    data: web::Data<AppState>,
    body: web::Json<TransferRequest>,
) -> impl Responder {
    let tx = Transaction::transfer(
        body.from.clone(),
        body.to.clone(),
        body.amount,
    );
    
    let mut blockchain = data.blockchain.write().await;
    match blockchain.add_transaction(tx) {
        Ok(hash) => {
            info!("Transfer created: {} -> {} ({} tokens)", 
                &body.from[..8.min(body.from.len())], 
                &body.to[..8.min(body.to.len())], 
                body.amount);
            HttpResponse::Ok().json(ApiResponse::success(hash))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Create data contribution transaction
pub async fn create_data_contribution(
    data: web::Data<AppState>,
    body: web::Json<DataContributionRequest>,
) -> impl Responder {
    let tx = Transaction::data_contribution(
        body.sender.clone(),
        body.data.clone(),
        body.sender.clone(),
    );
    
    let quality_score = tx.data_quality.as_ref()
        .map(|q| q.overall_score)
        .unwrap_or(0.0);
    
    let mut blockchain = data.blockchain.write().await;
    match blockchain.add_transaction(tx) {
        Ok(hash) => {
            info!("Data contribution: {} (quality: {:.2})", 
                &body.sender[..8.min(body.sender.len())], quality_score);
            
            #[derive(Serialize)]
            struct ContributionResponse {
                tx_hash: String,
                quality_score: f64,
            }
            
            HttpResponse::Ok().json(ApiResponse::success(ContributionResponse {
                tx_hash: hash,
                quality_score,
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

// ============ Account Endpoints ============

/// Get account info
pub async fn get_account(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();
    let blockchain = data.blockchain.read().await;
    
    match blockchain.get_account(&address) {
        Some(account) => HttpResponse::Ok().json(ApiResponse::success(account)),
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Account not found")),
    }
}

/// Faucet - Give test tokens to an address (testnet only)
pub async fn faucet(
    data: web::Data<AppState>,
    body: web::Json<FaucetRequest>,
) -> impl Responder {
    let mut blockchain = data.blockchain.write().await;
    let amount = body.amount.unwrap_or(1000);
    
    // For testnet: directly credit the account balance without requiring sender balance
    // Get or create the account and add tokens directly
    {
        use crate::blockchain::chain::Account;
        let account = blockchain.state.accounts
            .entry(body.address.clone())
            .or_insert_with(|| Account::new(body.address.clone()));
        account.balance += amount;
    }
    
    // Create a record transaction hash for the faucet distribution
    let tx_hash = format!("faucet_{}_{}", body.address, chrono::Utc::now().timestamp());
    
    info!("Faucet: credited {} tokens to {}", amount, &body.address);
    
    #[derive(Serialize)]
    struct FaucetResponse {
        address: String,
        amount: u64,
        transaction_hash: String,
    }
    
    HttpResponse::Ok().json(ApiResponse::success(FaucetResponse {
        address: body.address.clone(),
        amount,
        transaction_hash: tx_hash,
    }))
}

/// Get account balance
pub async fn get_balance(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();
    let blockchain = data.blockchain.read().await;
    let balance = blockchain.get_balance(&address);
    
    #[derive(Serialize)]
    struct BalanceResponse {
        address: String,
        balance: u64,
    }
    
    HttpResponse::Ok().json(ApiResponse::success(BalanceResponse { address, balance }))
}

/// Get account transactions
pub async fn get_account_transactions(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();
    let blockchain = data.blockchain.read().await;
    let txs = blockchain.get_transactions_for_address(&address);
    HttpResponse::Ok().json(ApiResponse::success(txs))
}

// ============ Mining Endpoints ============

/// Mine a new block
pub async fn mine_block(
    data: web::Data<AppState>,
    body: web::Json<MineBlockRequest>,
) -> impl Responder {
    let mut blockchain = data.blockchain.write().await;
    
    match blockchain.mine_block(body.validator.clone()) {
        Ok(block) => {
            info!("Block #{} mined by {}", block.index, &body.validator[..8.min(body.validator.len())]);
            HttpResponse::Ok().json(ApiResponse::success(block))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

// ============ Consensus Endpoints ============

/// Get validators
pub async fn get_validators(data: web::Data<AppState>) -> impl Responder {
    let consensus = data.consensus.read().await;
    let validators: Vec<&Validator> = consensus.get_active_validators();
    HttpResponse::Ok().json(ApiResponse::success(validators))
}

/// Register validator
pub async fn register_validator(
    data: web::Data<AppState>,
    body: web::Json<RegisterValidatorRequest>,
) -> impl Responder {
    let mut consensus = data.consensus.write().await;
    
    match consensus.register_validator(body.address.clone(), body.stake) {
        Ok(_) => {
            info!("Validator registered: {} (stake: {})", 
                &body.address[..8.min(body.address.len())], body.stake);
            HttpResponse::Ok().json(ApiResponse::success("Validator registered"))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

// ============ Data Marketplace Endpoints ============

/// List data for sale
pub async fn list_data(
    data: web::Data<AppState>,
    body: web::Json<ListDataRequest>,
) -> impl Responder {
    let data_hash = Transaction::hash_data(&body.data);
    let quality = Transaction::calculate_data_quality(&body.data);
    
    let listing = DataListing::new(
        data_hash.clone(),
        body.owner.clone(),
        body.title.clone(),
        body.description.clone(),
        DataCategory::from_string(&body.category),
        body.price,
        quality.overall_score,
        quality.entropy_score,
        body.data.len() as u64,
    );
    
    let mut marketplace = data.marketplace.write().await;
    match marketplace.list_data(listing) {
        Ok(id) => {
            info!("Data listed: {} by {}", &data_hash[..8], &body.owner[..8.min(body.owner.len())]);
            
            #[derive(Serialize)]
            struct ListingResponse {
                listing_id: String,
                data_hash: String,
                quality_score: f64,
            }
            
            HttpResponse::Ok().json(ApiResponse::success(ListingResponse {
                listing_id: id,
                data_hash,
                quality_score: quality.overall_score,
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Get marketplace listings
pub async fn get_marketplace_listings(
    data: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    let marketplace = data.marketplace.read().await;
    
    let category = query.category.as_ref()
        .map(|c| DataCategory::from_string(c));
    
    let sort_by = match query.sort_by.as_deref() {
        Some("price_asc") => SortBy::PriceAsc,
        Some("price_desc") => SortBy::PriceDesc,
        Some("quality") => SortBy::QualityDesc,
        Some("popularity") => SortBy::PopularityDesc,
        Some("rating") => SortBy::RatingDesc,
        _ => SortBy::Newest,
    };
    
    let listings = marketplace.search(
        query.query.as_deref(),
        category.as_ref(),
        query.min_price,
        query.max_price,
        query.min_quality,
        sort_by,
        query.limit.unwrap_or(50),
    );
    
    HttpResponse::Ok().json(ApiResponse::success(listings))
}

/// Get marketplace stats
pub async fn get_marketplace_stats(data: web::Data<AppState>) -> impl Responder {
    let marketplace = data.marketplace.read().await;
    let stats = marketplace.get_stats();
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Purchase data
pub async fn purchase_data(
    data: web::Data<AppState>,
    body: web::Json<PurchaseDataRequest>,
) -> impl Responder {
    let mut marketplace = data.marketplace.write().await;
    
    match marketplace.purchase_data(&body.data_hash, &body.buyer) {
        Ok(purchase) => {
            info!("Data purchased: {} by {}", 
                &body.data_hash[..8], &body.buyer[..8.min(body.buyer.len())]);
            HttpResponse::Ok().json(ApiResponse::success(purchase))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Get listing by hash
pub async fn get_listing(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let hash = path.into_inner();
    let marketplace = data.marketplace.read().await;
    
    match marketplace.get_listing(&hash) {
        Some(listing) => HttpResponse::Ok().json(ApiResponse::success(listing)),
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Listing not found")),
    }
}

// ============ Network Endpoints ============

/// Get network stats
pub async fn get_network_stats(data: web::Data<AppState>) -> impl Responder {
    let stats = data.network.get_stats().await;
    HttpResponse::Ok().json(ApiResponse::success(stats))
}

/// Get peers
pub async fn get_peers(data: web::Data<AppState>) -> impl Responder {
    let peers = data.network.get_active_peers().await;
    HttpResponse::Ok().json(ApiResponse::success(peers))
}

// ============ Utility Types ============

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

// ============ Router Configuration ============

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Blockchain routes
        .route("/api/chain", web::get().to(get_chain_info))
        .route("/api/blocks", web::get().to(get_blocks))
        .route("/api/blocks/latest", web::get().to(get_latest_block))
        .route("/api/blocks/{index}", web::get().to(get_block))
        .route("/api/blocks/hash/{hash}", web::get().to(get_block_by_hash))
        
        // Transaction routes
        .route("/api/transactions", web::get().to(get_recent_transactions))
        .route("/api/transactions/pending", web::get().to(get_pending_transactions))
        .route("/api/transactions/{hash}", web::get().to(get_transaction))
        .route("/api/transactions/transfer", web::post().to(create_transfer))
        .route("/api/transactions/contribute", web::post().to(create_data_contribution))
        
        // Account routes
        .route("/api/accounts/{address}", web::get().to(get_account))
        .route("/api/accounts/{address}/balance", web::get().to(get_balance))
        .route("/api/accounts/{address}/transactions", web::get().to(get_account_transactions))
        
        // Faucet route (for testnet)
        .route("/api/faucet", web::post().to(faucet))
        
        // Mining routes
        .route("/api/mine", web::post().to(mine_block))
        
        // Consensus routes
        .route("/api/validators", web::get().to(get_validators))
        .route("/api/validators/register", web::post().to(register_validator))
        
        // Marketplace routes
        .route("/api/marketplace", web::get().to(get_marketplace_listings))
        .route("/api/marketplace/stats", web::get().to(get_marketplace_stats))
        .route("/api/marketplace/list", web::post().to(list_data))
        .route("/api/marketplace/purchase", web::post().to(purchase_data))
        .route("/api/marketplace/{hash}", web::get().to(get_listing))
        
        // Network routes
        .route("/api/network", web::get().to(get_network_stats))
        .route("/api/network/peers", web::get().to(get_peers))
        
        // Health & Status routes
        .route("/api/status", web::get().to(get_status))
        .route("/api/health", web::get().to(health_check));
    
    // Configure IoT routes
    super::iot::configure_iot_routes(cfg);
    
    // Configure Rewards routes
    super::rewards::configure_rewards_routes(cfg);
}

/// Health check endpoint (for load balancers)
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

/// Status endpoint with basic info
pub async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let blockchain = data.blockchain.read().await;
    let height = blockchain.chain.len() as u64 + blockchain.total_blocks.saturating_sub(blockchain.chain.len() as u64);
    
    HttpResponse::Ok().json(serde_json::json!({
        "status": "running",
        "version": "0.6.0",
        "chain_height": height,
        "pending_tx": blockchain.pending_transactions.len(),
        "timestamp": chrono::Utc::now().timestamp()
    }))
}
