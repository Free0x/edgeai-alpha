use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::rest::ApiResponse;

/// Trading pair information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPair {
    pub id: String,
    pub base_token: String,   // e.g., "EDGE"
    pub quote_token: String,  // e.g., "USDT"
    pub base_reserve: u64,    // Amount of base token in pool
    pub quote_reserve: u64,   // Amount of quote token in pool
    pub total_liquidity: u64, // Total LP tokens issued
    pub fee_rate: f64,        // Trading fee (e.g., 0.003 = 0.3%)
    pub volume_24h: u64,
    pub created_at: i64,
}

impl TradingPair {
    /// Calculate price of base token in terms of quote token
    pub fn get_price(&self) -> f64 {
        if self.base_reserve == 0 {
            return 0.0;
        }
        self.quote_reserve as f64 / self.base_reserve as f64
    }

    /// Calculate output amount for a swap (constant product formula: x * y = k)
    pub fn calculate_swap_output(&self, amount_in: u64, is_base_to_quote: bool) -> (u64, u64) {
        let fee = (amount_in as f64 * self.fee_rate) as u64;
        let amount_in_after_fee = amount_in - fee;

        let (reserve_in, reserve_out) = if is_base_to_quote {
            (self.base_reserve, self.quote_reserve)
        } else {
            (self.quote_reserve, self.base_reserve)
        };

        // x * y = k
        // (x + dx) * (y - dy) = k
        // dy = y - k / (x + dx)
        // dy = y * dx / (x + dx)
        let amount_out = (reserve_out as u128 * amount_in_after_fee as u128
            / (reserve_in as u128 + amount_in_after_fee as u128)) as u64;

        (amount_out, fee)
    }

    /// Calculate liquidity tokens to mint for adding liquidity
    pub fn calculate_liquidity_mint(&self, base_amount: u64, quote_amount: u64) -> u64 {
        if self.total_liquidity == 0 {
            // First liquidity provider gets sqrt(base * quote) LP tokens
            ((base_amount as f64 * quote_amount as f64).sqrt()) as u64
        } else {
            // Proportional to existing liquidity
            let base_share =
                base_amount as u128 * self.total_liquidity as u128 / self.base_reserve as u128;
            let quote_share =
                quote_amount as u128 * self.total_liquidity as u128 / self.quote_reserve as u128;
            std::cmp::min(base_share, quote_share) as u64
        }
    }
}

/// Liquidity position for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPosition {
    pub pair_id: String,
    pub owner: String,
    pub lp_tokens: u64,
    pub base_deposited: u64,
    pub quote_deposited: u64,
    pub created_at: i64,
}

/// Order in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub pair_id: String,
    pub owner: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: f64,
    pub amount: u64,
    pub filled: u64,
    pub status: OrderStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

/// Trade history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub pair_id: String,
    pub buyer: String,
    pub seller: String,
    pub price: f64,
    pub amount: u64,
    pub total: u64,
    pub fee: u64,
    pub timestamp: i64,
}

/// DEX Manager
pub struct DexManager {
    pub pairs: HashMap<String, TradingPair>,
    pub positions: HashMap<String, Vec<LiquidityPosition>>,
    pub orders: HashMap<String, Vec<Order>>,
    pub trades: Vec<Trade>,
}

impl DexManager {
    pub fn new() -> Self {
        let mut manager = DexManager {
            pairs: HashMap::new(),
            positions: HashMap::new(),
            orders: HashMap::new(),
            trades: Vec::new(),
        };

        // Initialize with default trading pairs
        manager.initialize_default_pairs();
        manager
    }

    fn initialize_default_pairs(&mut self) {
        let now = chrono::Utc::now().timestamp();

        // EDGE/USDT pair
        let edge_usdt = TradingPair {
            id: "EDGE-USDT".to_string(),
            base_token: "EDGE".to_string(),
            quote_token: "USDT".to_string(),
            base_reserve: 10_000_000,   // 10M EDGE
            quote_reserve: 5_000_000,   // 5M USDT (price = 0.5 USDT)
            total_liquidity: 7_071_067, // sqrt(10M * 5M)
            fee_rate: 0.003,            // 0.3%
            volume_24h: 1_250_000,
            created_at: now,
        };

        // EDGE/BTC pair
        let edge_btc = TradingPair {
            id: "EDGE-BTC".to_string(),
            base_token: "EDGE".to_string(),
            quote_token: "BTC".to_string(),
            base_reserve: 5_000_000, // 5M EDGE
            quote_reserve: 50,       // 50 BTC (price = 0.00001 BTC)
            total_liquidity: 15_811,
            fee_rate: 0.003,
            volume_24h: 500_000,
            created_at: now,
        };

        // EDGE/ETH pair
        let edge_eth = TradingPair {
            id: "EDGE-ETH".to_string(),
            base_token: "EDGE".to_string(),
            quote_token: "ETH".to_string(),
            base_reserve: 8_000_000, // 8M EDGE
            quote_reserve: 1_600,    // 1600 ETH (price = 0.0002 ETH)
            total_liquidity: 113_137,
            fee_rate: 0.003,
            volume_24h: 800_000,
            created_at: now,
        };

        // DATA/EDGE pair (IoT data token)
        let data_edge = TradingPair {
            id: "DATA-EDGE".to_string(),
            base_token: "DATA".to_string(),
            quote_token: "EDGE".to_string(),
            base_reserve: 50_000_000, // 50M DATA
            quote_reserve: 2_500_000, // 2.5M EDGE (price = 0.05 EDGE)
            total_liquidity: 11_180_339,
            fee_rate: 0.002, // Lower fee for ecosystem token
            volume_24h: 300_000,
            created_at: now,
        };

        self.pairs.insert(edge_usdt.id.clone(), edge_usdt);
        self.pairs.insert(edge_btc.id.clone(), edge_btc);
        self.pairs.insert(edge_eth.id.clone(), edge_eth);
        self.pairs.insert(data_edge.id.clone(), data_edge);

        // Generate some initial trades
        self.generate_initial_trades();
    }

    fn generate_initial_trades(&mut self) {
        let now = chrono::Utc::now().timestamp();

        for pair in self.pairs.values() {
            let base_price = pair.get_price();

            // Generate 50 trades for each pair
            for i in 0..50 {
                let price_variation = (rand::random::<f64>() - 0.5) * 0.02; // ±1%
                let price = base_price * (1.0 + price_variation);
                let amount = (rand::random::<f64>() * 10000.0 + 100.0) as u64;

                let trade = Trade {
                    id: format!("trade_{}_{}", pair.id, i),
                    pair_id: pair.id.clone(),
                    buyer: format!("edge{:x}", rand::random::<u64>()),
                    seller: format!("edge{:x}", rand::random::<u64>()),
                    price,
                    amount,
                    total: (price * amount as f64) as u64,
                    fee: (amount as f64 * pair.fee_rate) as u64,
                    timestamp: now - (50 - i) * 60, // One trade per minute
                };

                self.trades.push(trade);
            }
        }
    }

    /// Execute a swap
    pub fn swap(
        &mut self,
        pair_id: &str,
        amount_in: u64,
        is_base_to_quote: bool,
        user: &str,
    ) -> Result<Trade, String> {
        let pair = self
            .pairs
            .get_mut(pair_id)
            .ok_or_else(|| "Trading pair not found".to_string())?;

        let (amount_out, fee) = pair.calculate_swap_output(amount_in, is_base_to_quote);

        if amount_out == 0 {
            return Err("Insufficient liquidity".to_string());
        }

        // Update reserves
        if is_base_to_quote {
            pair.base_reserve += amount_in;
            pair.quote_reserve -= amount_out;
        } else {
            pair.quote_reserve += amount_in;
            pair.base_reserve -= amount_out;
        }

        pair.volume_24h += amount_in;

        let trade = Trade {
            id: format!("trade_{}", chrono::Utc::now().timestamp_millis()),
            pair_id: pair_id.to_string(),
            buyer: if is_base_to_quote {
                "pool".to_string()
            } else {
                user.to_string()
            },
            seller: if is_base_to_quote {
                user.to_string()
            } else {
                "pool".to_string()
            },
            price: pair.get_price(),
            amount: amount_in,
            total: amount_out,
            fee,
            timestamp: chrono::Utc::now().timestamp(),
        };

        self.trades.push(trade.clone());

        Ok(trade)
    }

    /// Add liquidity to a pool
    pub fn add_liquidity(
        &mut self,
        pair_id: &str,
        base_amount: u64,
        quote_amount: u64,
        user: &str,
    ) -> Result<LiquidityPosition, String> {
        let pair = self
            .pairs
            .get_mut(pair_id)
            .ok_or_else(|| "Trading pair not found".to_string())?;

        let lp_tokens = pair.calculate_liquidity_mint(base_amount, quote_amount);

        if lp_tokens == 0 {
            return Err("Insufficient amounts".to_string());
        }

        // Update pair reserves
        pair.base_reserve += base_amount;
        pair.quote_reserve += quote_amount;
        pair.total_liquidity += lp_tokens;

        let position = LiquidityPosition {
            pair_id: pair_id.to_string(),
            owner: user.to_string(),
            lp_tokens,
            base_deposited: base_amount,
            quote_deposited: quote_amount,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.positions
            .entry(user.to_string())
            .or_insert_with(Vec::new)
            .push(position.clone());

        Ok(position)
    }
}

/// DEX state shared across handlers
pub struct DexState {
    pub manager: Arc<RwLock<DexManager>>,
}

impl DexState {
    pub fn new() -> Self {
        DexState {
            manager: Arc::new(RwLock::new(DexManager::new())),
        }
    }
}

// ============== API Request/Response Types ==============

#[derive(Debug, Deserialize)]
pub struct SwapRequest {
    pub pair_id: String,
    pub amount_in: u64,
    pub is_base_to_quote: bool,
    pub user: String,
    pub min_amount_out: Option<u64>, // Slippage protection
}

#[derive(Debug, Deserialize)]
pub struct AddLiquidityRequest {
    pub pair_id: String,
    pub base_amount: u64,
    pub quote_amount: u64,
    pub user: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePairRequest {
    pub base_token: String,
    pub quote_token: String,
    pub initial_base_amount: u64,
    pub initial_quote_amount: u64,
    pub fee_rate: Option<f64>,
    pub creator: String,
}

#[derive(Debug, Serialize)]
pub struct PairStats {
    pub pair: TradingPair,
    pub price: f64,
    pub price_change_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
}

#[derive(Debug, Serialize)]
pub struct SwapQuote {
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price_impact: f64,
    pub exchange_rate: f64,
}

// ============== API Handlers ==============

/// Get all trading pairs
pub async fn get_pairs(data: web::Data<DexState>) -> impl Responder {
    let manager = data.manager.read().await;
    let pairs: Vec<PairStats> = manager
        .pairs
        .values()
        .map(|pair| {
            PairStats {
                price: pair.get_price(),
                price_change_24h: (rand::random::<f64>() - 0.5) * 10.0, // Simulated for now
                high_24h: pair.get_price() * 1.05,
                low_24h: pair.get_price() * 0.95,
                pair: pair.clone(),
            }
        })
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(pairs))
}

/// Get specific trading pair
pub async fn get_pair(data: web::Data<DexState>, path: web::Path<String>) -> impl Responder {
    let pair_id = path.into_inner();
    let manager = data.manager.read().await;

    match manager.pairs.get(&pair_id) {
        Some(pair) => {
            let stats = PairStats {
                price: pair.get_price(),
                price_change_24h: (rand::random::<f64>() - 0.5) * 10.0,
                high_24h: pair.get_price() * 1.05,
                low_24h: pair.get_price() * 0.95,
                pair: pair.clone(),
            };
            HttpResponse::Ok().json(ApiResponse::success(stats))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Trading pair not found")),
    }
}

/// Get swap quote (preview)
pub async fn get_swap_quote(
    data: web::Data<DexState>,
    query: web::Query<SwapRequest>,
) -> impl Responder {
    let manager = data.manager.read().await;

    match manager.pairs.get(&query.pair_id) {
        Some(pair) => {
            let (amount_out, fee) =
                pair.calculate_swap_output(query.amount_in, query.is_base_to_quote);

            // Calculate price impact
            let current_price = pair.get_price();
            let new_base = if query.is_base_to_quote {
                pair.base_reserve + query.amount_in
            } else {
                pair.base_reserve - amount_out
            };
            let new_quote = if query.is_base_to_quote {
                pair.quote_reserve - amount_out
            } else {
                pair.quote_reserve + query.amount_in
            };
            let new_price = new_quote as f64 / new_base as f64;
            let price_impact = ((new_price - current_price) / current_price * 100.0).abs();

            let quote = SwapQuote {
                amount_in: query.amount_in,
                amount_out,
                fee,
                price_impact,
                exchange_rate: amount_out as f64 / query.amount_in as f64,
            };

            HttpResponse::Ok().json(ApiResponse::success(quote))
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Trading pair not found")),
    }
}

/// Execute swap
pub async fn execute_swap(
    data: web::Data<DexState>,
    body: web::Json<SwapRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    match manager.swap(
        &body.pair_id,
        body.amount_in,
        body.is_base_to_quote,
        &body.user,
    ) {
        Ok(trade) => {
            info!(
                "Swap executed: {} {} for {} in pair {}",
                body.amount_in,
                if body.is_base_to_quote {
                    "base"
                } else {
                    "quote"
                },
                trade.total,
                body.pair_id
            );
            HttpResponse::Ok().json(ApiResponse::success(trade))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Add liquidity
pub async fn add_liquidity(
    data: web::Data<DexState>,
    body: web::Json<AddLiquidityRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    match manager.add_liquidity(
        &body.pair_id,
        body.base_amount,
        body.quote_amount,
        &body.user,
    ) {
        Ok(position) => {
            info!(
                "Liquidity added: {} base + {} quote to {} by {}",
                body.base_amount, body.quote_amount, body.pair_id, body.user
            );
            HttpResponse::Ok().json(ApiResponse::success(position))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Create new trading pair
pub async fn create_pair(
    data: web::Data<DexState>,
    body: web::Json<CreatePairRequest>,
) -> impl Responder {
    let mut manager = data.manager.write().await;

    let pair_id = format!("{}-{}", body.base_token, body.quote_token);

    if manager.pairs.contains_key(&pair_id) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Trading pair already exists"));
    }

    let lp_tokens =
        ((body.initial_base_amount as f64 * body.initial_quote_amount as f64).sqrt()) as u64;

    let pair = TradingPair {
        id: pair_id.clone(),
        base_token: body.base_token.clone(),
        quote_token: body.quote_token.clone(),
        base_reserve: body.initial_base_amount,
        quote_reserve: body.initial_quote_amount,
        total_liquidity: lp_tokens,
        fee_rate: body.fee_rate.unwrap_or(0.003),
        volume_24h: 0,
        created_at: chrono::Utc::now().timestamp(),
    };

    manager.pairs.insert(pair_id.clone(), pair.clone());

    // Create initial liquidity position for creator
    let position = LiquidityPosition {
        pair_id: pair_id.clone(),
        owner: body.creator.clone(),
        lp_tokens,
        base_deposited: body.initial_base_amount,
        quote_deposited: body.initial_quote_amount,
        created_at: chrono::Utc::now().timestamp(),
    };

    manager
        .positions
        .entry(body.creator.clone())
        .or_insert_with(Vec::new)
        .push(position);

    info!("New trading pair created: {} by {}", pair_id, body.creator);

    HttpResponse::Ok().json(ApiResponse::success(pair))
}

/// Get recent trades for a pair
pub async fn get_trades(data: web::Data<DexState>, path: web::Path<String>) -> impl Responder {
    let pair_id = path.into_inner();
    let manager = data.manager.read().await;

    let trades: Vec<&Trade> = manager
        .trades
        .iter()
        .filter(|t| t.pair_id == pair_id)
        .rev()
        .take(100)
        .collect();

    HttpResponse::Ok().json(ApiResponse::success(trades))
}

/// Get user's liquidity positions
pub async fn get_user_positions(
    data: web::Data<DexState>,
    path: web::Path<String>,
) -> impl Responder {
    let user = path.into_inner();
    let manager = data.manager.read().await;

    let positions = manager.positions.get(&user).cloned().unwrap_or_default();

    HttpResponse::Ok().json(ApiResponse::success(positions))
}

/// Configure DEX routes
pub fn configure_dex_routes(cfg: &mut web::ServiceConfig, dex_state: web::Data<DexState>) {
    cfg.app_data(dex_state)
        .route("/api/dex/pairs", web::get().to(get_pairs))
        .route("/api/dex/pairs/{pair_id}", web::get().to(get_pair))
        .route("/api/dex/quote", web::get().to(get_swap_quote))
        .route("/api/dex/swap", web::post().to(execute_swap))
        .route("/api/dex/liquidity", web::post().to(add_liquidity))
        .route("/api/dex/pairs/create", web::post().to(create_pair))
        .route("/api/dex/trades/{pair_id}", web::get().to(get_trades))
        .route(
            "/api/dex/positions/{user}",
            web::get().to(get_user_positions),
        );
}
