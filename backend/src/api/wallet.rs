// EdgeAI Blockchain - Wallet API Endpoints
// Provides wallet creation, signing, and signed transaction submission

use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::rest::{ApiResponse, AppState};
use crate::blockchain::Transaction;
use crate::crypto::{address_from_public_key, verify_signature, Wallet};

// ============ Request/Response Types ============

#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub address: String,
    pub public_key: String,
    pub secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportWalletRequest {
    pub secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SignMessageRequest {
    pub secret_key: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SignatureResponse {
    pub message: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifySignatureRequest {
    pub public_key: String,
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct SignedTransferRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct SignedDataContributionRequest {
    pub sender: String,
    pub data: String,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct PrepareTransferRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct PreparedTransaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub message_to_sign: String,
}

#[derive(Debug, Deserialize)]
pub struct PrepareDataContributionRequest {
    pub sender: String,
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct PreparedDataContribution {
    pub sender: String,
    pub data_hash: String,
    pub message_to_sign: String,
}

// ============ Helper Functions ============

/// Create a deterministic message to sign for transfers
fn create_transfer_message(from: &str, to: &str, amount: u64) -> String {
    let data = format!("TRANSFER:{}:{}:{}", from, to, amount);
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a deterministic message to sign for data contributions
fn create_data_contribution_message(sender: &str, data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let data_hash = hex::encode(hasher.finalize());

    let message = format!("DATA_CONTRIBUTION:{}:{}", sender, data_hash);
    let mut hasher2 = Sha256::new();
    hasher2.update(message.as_bytes());
    hex::encode(hasher2.finalize())
}

// ============ Wallet Endpoints ============

/// Generate a new wallet (key pair)
pub async fn generate_wallet() -> impl Responder {
    let wallet = Wallet::new();

    info!("New wallet generated: {}", wallet.address());

    HttpResponse::Ok().json(ApiResponse::success(WalletResponse {
        address: wallet.address().to_string(),
        public_key: wallet.public_key_hex(),
        secret_key: wallet.secret_key_hex(),
    }))
}

/// Import wallet from secret key
pub async fn import_wallet(body: web::Json<ImportWalletRequest>) -> impl Responder {
    match Wallet::from_secret_key(&body.secret_key) {
        Ok(wallet) => {
            info!("Wallet imported: {}", wallet.address());
            HttpResponse::Ok().json(ApiResponse::success(WalletResponse {
                address: wallet.address().to_string(),
                public_key: wallet.public_key_hex(),
                secret_key: wallet.secret_key_hex(),
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Invalid secret key: {}",
            e
        ))),
    }
}

/// Get address from public key
pub async fn get_address_from_public_key(path: web::Path<String>) -> impl Responder {
    let public_key = path.into_inner();

    match address_from_public_key(&public_key) {
        Ok(address) => {
            #[derive(Serialize)]
            struct AddressResponse {
                public_key: String,
                address: String,
            }
            HttpResponse::Ok().json(ApiResponse::success(AddressResponse {
                public_key,
                address,
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Invalid public key: {}",
            e
        ))),
    }
}

/// Sign a message with secret key
pub async fn sign_message(body: web::Json<SignMessageRequest>) -> impl Responder {
    match Wallet::from_secret_key(&body.secret_key) {
        Ok(wallet) => {
            let signature = wallet.sign(body.message.as_bytes());

            HttpResponse::Ok().json(ApiResponse::success(SignatureResponse {
                message: body.message.clone(),
                signature,
                public_key: wallet.public_key_hex(),
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Invalid secret key: {}",
            e
        ))),
    }
}

/// Verify a signature
pub async fn verify_signature_endpoint(body: web::Json<VerifySignatureRequest>) -> impl Responder {
    match verify_signature(&body.public_key, body.message.as_bytes(), &body.signature) {
        Ok(valid) => {
            let address =
                address_from_public_key(&body.public_key).unwrap_or_else(|_| "invalid".to_string());

            HttpResponse::Ok().json(ApiResponse::success(VerifyResponse { valid, address }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Verification error: {}",
            e
        ))),
    }
}

/// Prepare a transfer transaction for signing (returns the message to sign)
pub async fn prepare_transfer(body: web::Json<PrepareTransferRequest>) -> impl Responder {
    let message_to_sign = create_transfer_message(&body.from, &body.to, body.amount);

    HttpResponse::Ok().json(ApiResponse::success(PreparedTransaction {
        from: body.from.clone(),
        to: body.to.clone(),
        amount: body.amount,
        message_to_sign,
    }))
}

/// Prepare a data contribution for signing
pub async fn prepare_data_contribution(
    body: web::Json<PrepareDataContributionRequest>,
) -> impl Responder {
    let message_to_sign = create_data_contribution_message(&body.sender, &body.data);

    let mut hasher = Sha256::new();
    hasher.update(body.data.as_bytes());
    let data_hash = hex::encode(hasher.finalize());

    HttpResponse::Ok().json(ApiResponse::success(PreparedDataContribution {
        sender: body.sender.clone(),
        data_hash,
        message_to_sign,
    }))
}

/// Submit a signed transfer transaction
pub async fn submit_signed_transfer(
    data: web::Data<AppState>,
    body: web::Json<SignedTransferRequest>,
) -> impl Responder {
    // Verify the address matches the public key
    let derived_address = match address_from_public_key(&body.public_key) {
        Ok(addr) => addr,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Invalid public key: {}",
                e
            )));
        }
    };

    if derived_address != body.from {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Sender address does not match public key",
        ));
    }

    // Recreate the message that should have been signed
    let expected_message = create_transfer_message(&body.from, &body.to, body.amount);

    // Verify the signature against the expected message
    match verify_signature(
        &body.public_key,
        expected_message.as_bytes(),
        &body.signature,
    ) {
        Ok(valid) => {
            if !valid {
                return HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("Invalid signature"));
            }
        }
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Signature verification error: {}",
                e
            )));
        }
    }

    // Create the signed transaction
    let tx = Transaction::transfer_signed(
        body.from.clone(),
        body.public_key.clone(),
        body.to.clone(),
        body.amount,
        body.signature.clone(),
    );

    // Add to blockchain
    let mut blockchain = data.blockchain.write().await;
    match blockchain.add_transaction(tx) {
        Ok(hash) => {
            info!(
                "Signed transfer: {} -> {} ({} tokens)",
                &body.from[..12.min(body.from.len())],
                &body.to[..12.min(body.to.len())],
                body.amount
            );
            HttpResponse::Ok().json(ApiResponse::success(hash))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Submit a signed data contribution transaction
pub async fn submit_signed_data_contribution(
    data: web::Data<AppState>,
    body: web::Json<SignedDataContributionRequest>,
) -> impl Responder {
    // Verify the address matches the public key
    let derived_address = match address_from_public_key(&body.public_key) {
        Ok(addr) => addr,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Invalid public key: {}",
                e
            )));
        }
    };

    if derived_address != body.sender {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Sender address does not match public key",
        ));
    }

    // Recreate the message that should have been signed
    let expected_message = create_data_contribution_message(&body.sender, &body.data);

    // Verify the signature against the expected message
    match verify_signature(
        &body.public_key,
        expected_message.as_bytes(),
        &body.signature,
    ) {
        Ok(valid) => {
            if !valid {
                return HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("Invalid signature"));
            }
        }
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
                "Signature verification error: {}",
                e
            )));
        }
    }

    // Create the signed transaction
    let tx = Transaction::data_contribution_signed(
        body.sender.clone(),
        body.public_key.clone(),
        body.data.clone(),
        body.sender.clone(),
        body.signature.clone(),
    );

    let quality_score = tx
        .data_quality
        .as_ref()
        .map(|q| q.overall_score)
        .unwrap_or(0.0);

    // Add to blockchain
    let mut blockchain = data.blockchain.write().await;
    match blockchain.add_transaction(tx) {
        Ok(hash) => {
            info!(
                "Signed data contribution: {} (quality: {:.2})",
                &body.sender[..12.min(body.sender.len())],
                quality_score
            );

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

// ============ External IoT Device API ============

/// Request structure for external IoT device data submission
/// This API allows real IoT devices to submit telemetry data to the blockchain
#[derive(Debug, Clone, Deserialize)]
pub struct ExternalIoTDataRequest {
    /// Device identifier (will be used as sender address)
    pub device_id: String,
    /// Device API key for authentication
    pub api_key: String,
    /// Raw telemetry data in JSON format
    pub telemetry: serde_json::Value,
    /// Data category (SmartCity, Manufacturing, Agriculture, Energy, Healthcare, Logistics, EdgeAI)
    pub category: String,
    /// Optional geographic location [latitude, longitude]
    pub location: Option<[f64; 2]>,
}

/// Request structure for batch IoT data submission
/// Allows submitting multiple telemetry records in a single request
#[derive(Debug, Deserialize)]
pub struct BatchIoTDataRequest {
    /// List of IoT data submissions (max 100 per batch)
    pub transactions: Vec<ExternalIoTDataRequest>,
}

/// Response for batch IoT data submission
#[derive(Debug, Serialize)]
pub struct BatchIoTSubmissionResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BatchItemResult>,
}

#[derive(Debug, Serialize)]
pub struct BatchItemResult {
    pub device_id: String,
    pub success: bool,
    pub tx_hash: Option<String>,
    pub reward: Option<u64>,
    pub error: Option<String>,
}

/// Response for IoT data submission
#[derive(Debug, Serialize)]
pub struct IoTSubmissionResponse {
    pub tx_hash: String,
    pub device_id: String,
    pub reward: u64,
    pub quality_score: f64,
    pub block_pending: bool,
}

/// Submit IoT telemetry data from external devices
///
/// # Endpoint
/// POST /api/iot/submit
///
/// # Request Body
/// ```json
/// {
///   "device_id": "my_sensor_001",
///   "api_key": "your_api_key",
///   "telemetry": {"temperature": 25.5, "humidity": 60},
///   "category": "SmartCity",
///   "location": [1.3521, 103.8198]
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "data": {
///     "tx_hash": "0x...",
///     "device_id": "my_sensor_001",
///     "reward": 50,
///     "quality_score": 0.85,
///     "block_pending": true
///   }
/// }
/// ```
pub async fn submit_iot_data(
    data: web::Data<AppState>,
    body: web::Json<ExternalIoTDataRequest>,
) -> impl Responder {
    // Validate category
    let valid_categories = [
        "SmartCity",
        "Manufacturing",
        "Agriculture",
        "Energy",
        "Healthcare",
        "Logistics",
        "EdgeAI",
        "General",
    ];
    if !valid_categories.contains(&body.category.as_str()) {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Invalid category. Must be one of: {:?}",
            valid_categories
        )));
    }

    // TODO: Validate API key against registered devices
    // For now, accept any non-empty API key for testing
    if body.api_key.is_empty() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("API key required"));
    }

    // Build telemetry JSON string
    let telemetry_str = body.telemetry.to_string();

    // Build full data payload
    let (lat, lng) = body.location.map(|l| (l[0], l[1])).unwrap_or((0.0, 0.0));
    let timestamp = chrono::Utc::now().timestamp();

    let full_data = format!(
        r#"{{"device":"{}","category":"{}","telemetry":{},"lat":{},"lng":{},"ts":{},"source":"external"}}"#,
        body.device_id, body.category, telemetry_str, lat, lng, timestamp
    );

    // Calculate reward based on data size and category
    let data_size = full_data.len() as u64;
    let base_reward = 30 + (data_size / 20);
    let category_bonus: u64 = match body.category.as_str() {
        "Healthcare" => 20, // Higher value for medical data
        "Manufacturing" => 15,
        "Energy" => 15,
        "Agriculture" => 10,
        _ => 5,
    };
    let reward = base_reward + category_bonus;

    // Create transaction
    use crate::blockchain::transaction::{TransactionType, TxOutput};

    let output = TxOutput {
        amount: reward,
        recipient: body.device_id.clone(),
        data_hash: Some(format!("ext_{:x}", timestamp)),
    };

    let tx = Transaction::new(
        TransactionType::DataContribution,
        body.device_id.clone(),
        vec![],
        vec![output],
        Some(full_data),
        1,
        21000,
    );

    let quality_score = tx
        .data_quality
        .as_ref()
        .map(|q| q.overall_score)
        .unwrap_or(0.5);

    // Add to blockchain
    let mut blockchain = data.blockchain.write().await;
    match blockchain.add_transaction(tx) {
        Ok(hash) => {
            info!(
                "External IoT data submitted: {} from {} (reward: {} EDGE)",
                &hash[..12.min(hash.len())],
                body.device_id,
                reward
            );

            HttpResponse::Ok().json(ApiResponse::success(IoTSubmissionResponse {
                tx_hash: hash,
                device_id: body.device_id.clone(),
                reward,
                quality_score,
                block_pending: true,
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
    }
}

/// Batch submit IoT telemetry data from multiple devices
///
/// # Endpoint
/// POST /api/iot/batch_submit
///
/// # Request Body
/// ```json
/// {
///   "transactions": [
///     {"device_id": "sensor_001", "api_key": "key", "telemetry": {...}, "category": "SmartCity"},
///     {"device_id": "sensor_002", "api_key": "key", "telemetry": {...}, "category": "Manufacturing"}
///   ]
/// }
/// ```
///
/// # Limits
/// - Maximum 100 transactions per batch
pub async fn batch_submit_iot_data(
    data: web::Data<AppState>,
    body: web::Json<BatchIoTDataRequest>,
) -> impl Responder {
    const MAX_BATCH_SIZE: usize = 100;

    // Validate batch size
    if body.transactions.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "Empty batch: at least one transaction required",
        ));
    }

    if body.transactions.len() > MAX_BATCH_SIZE {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!(
            "Batch too large: maximum {} transactions allowed",
            MAX_BATCH_SIZE
        )));
    }

    let valid_categories = [
        "SmartCity",
        "Manufacturing",
        "Agriculture",
        "Energy",
        "Healthcare",
        "Logistics",
        "EdgeAI",
        "General",
    ];

    let mut results = Vec::with_capacity(body.transactions.len());
    let mut successful = 0;
    let mut failed = 0;

    // Phase 1: Pre-validate and build transactions (can be done without blockchain lock)
    use crate::blockchain::transaction::{TransactionType, TxOutput};
    let timestamp = chrono::Utc::now().timestamp();

    let mut valid_transactions: Vec<(ExternalIoTDataRequest, Transaction, u64)> = Vec::new();

    for item in &body.transactions {
        // Validate category
        if !valid_categories.contains(&item.category.as_str()) {
            results.push(BatchItemResult {
                device_id: item.device_id.clone(),
                success: false,
                tx_hash: None,
                reward: None,
                error: Some(format!("Invalid category: {}", item.category)),
            });
            failed += 1;
            continue;
        }

        // Validate API key
        if item.api_key.is_empty() {
            results.push(BatchItemResult {
                device_id: item.device_id.clone(),
                success: false,
                tx_hash: None,
                reward: None,
                error: Some("API key required".to_string()),
            });
            failed += 1;
            continue;
        }

        // Build telemetry JSON string
        let telemetry_str = item.telemetry.to_string();

        // Build full data payload
        let (lat, lng) = item.location.map(|l| (l[0], l[1])).unwrap_or((0.0, 0.0));

        let full_data = format!(
            r#"{{"device":"{}","category":"{}","telemetry":{},"lat":{},"lng":{},"ts":{},"source":"batch"}}"#,
            item.device_id, item.category, telemetry_str, lat, lng, timestamp
        );

        // Calculate reward
        let data_size = full_data.len() as u64;
        let base_reward = 30 + (data_size / 20);
        let category_bonus: u64 = match item.category.as_str() {
            "Healthcare" => 20,
            "Manufacturing" => 15,
            "Energy" => 15,
            "Agriculture" => 10,
            _ => 5,
        };
        let reward = base_reward + category_bonus;

        let output = TxOutput {
            amount: reward,
            recipient: item.device_id.clone(),
            data_hash: Some(format!("batch_{:x}", timestamp)),
        };

        let tx = Transaction::new(
            TransactionType::DataContribution,
            item.device_id.clone(),
            vec![],
            vec![output],
            Some(full_data),
            1,
            21000,
        );

        valid_transactions.push((item.clone(), tx, reward));
    }

    // Phase 2: Use parallel batch validation if we have valid transactions
    if !valid_transactions.is_empty() {
        let txs: Vec<Transaction> = valid_transactions
            .iter()
            .map(|(_, tx, _)| tx.clone())
            .collect();
        // Use parallel batch processing
        let mut blockchain = data.blockchain.write().await;
        let (_batch_success, _batch_failed, successful_hashes) =
            blockchain.add_transactions_batch(txs);

        // Build results from batch processing
        let hash_set: std::collections::HashSet<String> = successful_hashes.into_iter().collect();

        for (item, tx, reward) in valid_transactions {
            if hash_set.contains(&tx.hash) {
                results.push(BatchItemResult {
                    device_id: item.device_id.clone(),
                    success: true,
                    tx_hash: Some(tx.hash),
                    reward: Some(reward),
                    error: None,
                });
                successful += 1;
            } else {
                results.push(BatchItemResult {
                    device_id: item.device_id.clone(),
                    success: false,
                    tx_hash: None,
                    reward: None,
                    error: Some("Transaction validation failed".to_string()),
                });
                failed += 1;
            }
        }
    }

    info!(
        "Batch IoT submission: {} successful, {} failed out of {} total",
        successful,
        failed,
        body.transactions.len()
    );

    HttpResponse::Ok().json(ApiResponse::success(BatchIoTSubmissionResponse {
        total: body.transactions.len(),
        successful,
        failed,
        results,
    }))
}

/// Get device registration info and API documentation
pub async fn get_iot_api_info() -> impl Responder {
    #[derive(Serialize)]
    struct IoTApiInfo {
        version: &'static str,
        endpoints: Vec<EndpointInfo>,
        categories: Vec<&'static str>,
        example_request: serde_json::Value,
    }

    #[derive(Serialize)]
    struct EndpointInfo {
        method: &'static str,
        path: &'static str,
        description: &'static str,
    }

    let info = IoTApiInfo {
        version: "1.1.0",
        endpoints: vec![
            EndpointInfo {
                method: "POST",
                path: "/api/iot/submit",
                description: "Submit single IoT telemetry data to the blockchain",
            },
            EndpointInfo {
                method: "POST",
                path: "/api/iot/batch_submit",
                description:
                    "Submit multiple IoT telemetry data in a single request (max 100 per batch)",
            },
            EndpointInfo {
                method: "GET",
                path: "/api/iot/info",
                description: "Get API documentation and supported categories",
            },
        ],
        categories: vec![
            "SmartCity",
            "Manufacturing",
            "Agriculture",
            "Energy",
            "Healthcare",
            "Logistics",
            "EdgeAI",
            "General",
        ],
        example_request: serde_json::json!({
            "single_submit": {
                "device_id": "my_sensor_001",
                "api_key": "your_api_key_here",
                "telemetry": {
                    "temperature": 25.5,
                    "humidity": 60,
                    "pressure": 1013.25
                },
                "category": "SmartCity",
                "location": [1.3521, 103.8198]
            },
            "batch_submit": {
                "transactions": [
                    {
                        "device_id": "sensor_001",
                        "api_key": "your_api_key",
                        "telemetry": {"temperature": 25.5},
                        "category": "SmartCity"
                    },
                    {
                        "device_id": "sensor_002",
                        "api_key": "your_api_key",
                        "telemetry": {"humidity": 60},
                        "category": "Manufacturing"
                    }
                ]
            }
        }),
    };

    HttpResponse::Ok().json(ApiResponse::success(info))
}

// ============ Router Configuration ============

pub fn configure_wallet_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Wallet management
        .route("/api/wallet/generate", web::post().to(generate_wallet))
        .route("/api/wallet/import", web::post().to(import_wallet))
        .route(
            "/api/wallet/address/{public_key}",
            web::get().to(get_address_from_public_key),
        )
        // Signing
        .route("/api/wallet/sign", web::post().to(sign_message))
        .route(
            "/api/wallet/verify",
            web::post().to(verify_signature_endpoint),
        )
        // Signed transactions
        .route(
            "/api/wallet/prepare-transfer",
            web::post().to(prepare_transfer),
        )
        .route(
            "/api/wallet/prepare-contribute",
            web::post().to(prepare_data_contribution),
        )
        .route(
            "/api/wallet/transfer",
            web::post().to(submit_signed_transfer),
        )
        .route(
            "/api/wallet/contribute",
            web::post().to(submit_signed_data_contribution),
        )
        // External IoT device API
        .route("/api/iot/submit", web::post().to(submit_iot_data))
        .route(
            "/api/iot/batch_submit",
            web::post().to(batch_submit_iot_data),
        )
        .route("/api/iot/info", web::get().to(get_iot_api_info));
}
