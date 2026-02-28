//! Smart Contract API endpoints for EdgeAI Blockchain
//!
//! This module provides HTTP endpoints for smart contract deployment,
//! execution, and management.

#![allow(dead_code)]

use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::Val;

use super::rest::ApiResponse;
use crate::contracts::{AbiFunction, AbiParam, ContractAbi, ExecutionContext, WasmRuntime};

/// Contract state (shared across handlers)
pub struct ContractState {
    pub runtime: Arc<RwLock<WasmRuntime>>,
}

// ============ Request Types ============

#[derive(Debug, Deserialize)]
pub struct DeployContractRequest {
    /// Hex encoded WASM bytecode
    pub wasm_code: String,
    /// Contract owner address
    pub owner: String,
    /// Contract ABI
    pub abi: ContractAbiRequest,
}

#[derive(Debug, Deserialize)]
pub struct ContractAbiRequest {
    pub name: String,
    pub version: String,
    pub functions: Vec<AbiFunctionRequest>,
}

#[derive(Debug, Deserialize)]
pub struct AbiFunctionRequest {
    pub name: String,
    pub inputs: Vec<AbiParamRequest>,
    pub outputs: Vec<AbiParamRequest>,
    pub mutates: bool,
}

#[derive(Debug, Deserialize)]
pub struct AbiParamRequest {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CallContractRequest {
    /// Contract address
    pub contract: String,
    /// Function name to call
    pub function: String,
    /// Function arguments (as JSON values)
    pub args: Vec<serde_json::Value>,
    /// Caller address
    pub caller: String,
    /// Value to send (in tokens)
    pub value: Option<u64>,
    /// Gas limit
    pub gas_limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GetStorageRequest {
    /// Contract address
    pub contract: String,
    /// Storage key (hex encoded)
    pub key: String,
}

// ============ Response Types ============

#[derive(Debug, Serialize)]
pub struct DeployContractResponse {
    pub address: String,
    pub code_hash: String,
}

#[derive(Debug, Serialize)]
pub struct CallContractResponse {
    pub success: bool,
    pub return_data: String,
    pub gas_used: u64,
    pub logs: Vec<LogResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub contract: String,
    pub topics: Vec<String>,
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct ContractInfoResponse {
    pub address: String,
    pub code_hash: String,
    pub name: String,
    pub version: String,
    pub functions: Vec<String>,
    pub compiled_at: String,
}

#[derive(Debug, Serialize)]
pub struct StorageResponse {
    pub key: String,
    pub value: Option<String>,
}

// ============ Handlers ============

/// Deploy a new smart contract
pub async fn deploy_contract(
    data: web::Data<ContractState>,
    req: web::Json<DeployContractRequest>,
) -> impl Responder {
    // Decode WASM bytecode (hex encoded)
    let wasm_code = match hex::decode(&req.wasm_code) {
        Ok(code) => code,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid hex encoding: {}", e)),
            });
        }
    };

    // Convert ABI
    let abi = ContractAbi {
        name: req.abi.name.clone(),
        version: req.abi.version.clone(),
        functions: req
            .abi
            .functions
            .iter()
            .map(|f| AbiFunction {
                name: f.name.clone(),
                inputs: f
                    .inputs
                    .iter()
                    .map(|p| AbiParam {
                        name: p.name.clone(),
                        param_type: p.param_type.clone(),
                    })
                    .collect(),
                outputs: f
                    .outputs
                    .iter()
                    .map(|p| AbiParam {
                        name: p.name.clone(),
                        param_type: p.param_type.clone(),
                    })
                    .collect(),
                mutates: f.mutates,
            })
            .collect(),
        events: vec![],
    };

    let mut runtime = data.runtime.write().await;

    match runtime.deploy_contract(&wasm_code, &req.owner, abi) {
        Ok(address) => {
            let contract = runtime.get_contract(&address).unwrap();
            info!("Contract deployed at {} by {}", &address, &req.owner);
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                data: Some(DeployContractResponse {
                    address,
                    code_hash: contract.code_hash,
                }),
                error: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Call a smart contract function
pub async fn call_contract(
    data: web::Data<ContractState>,
    req: web::Json<CallContractRequest>,
) -> impl Responder {
    // Convert JSON args to WASM values
    let args: Vec<Val> = req
        .args
        .iter()
        .map(|v| match v {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Val::I64(i)
                } else if let Some(f) = n.as_f64() {
                    Val::F64(f.to_bits())
                } else {
                    Val::I32(0)
                }
            }
            serde_json::Value::Bool(b) => Val::I32(if *b { 1 } else { 0 }),
            _ => Val::I32(0),
        })
        .collect();

    let context = ExecutionContext {
        contract_address: req.contract.clone(),
        caller: req.caller.clone(),
        value: req.value.unwrap_or(0),
        block_height: 0, // Would be set from actual blockchain state
        block_timestamp: chrono::Utc::now().timestamp(),
        gas_limit: req.gas_limit.unwrap_or(1_000_000),
    };

    let mut runtime = data.runtime.write().await;

    match runtime.execute(&req.contract, &req.function, &args, context) {
        Ok(result) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(CallContractResponse {
                success: result.success,
                return_data: hex::encode(&result.return_data),
                gas_used: result.gas_used,
                logs: result
                    .logs
                    .iter()
                    .map(|l| LogResponse {
                        contract: l.contract.clone(),
                        topics: l.topics.clone(),
                        data: hex::encode(&l.data),
                    })
                    .collect(),
                error: result.error,
            }),
            error: None,
        }),
        Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Get contract information
pub async fn get_contract(
    data: web::Data<ContractState>,
    path: web::Path<String>,
) -> impl Responder {
    let address = path.into_inner();
    let runtime = data.runtime.read().await;

    match runtime.get_contract(&address) {
        Some(contract) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(ContractInfoResponse {
                address: contract.address,
                code_hash: contract.code_hash,
                name: contract.abi.name,
                version: contract.abi.version,
                functions: contract
                    .abi
                    .functions
                    .iter()
                    .map(|f| f.name.clone())
                    .collect(),
                compiled_at: contract.deployed_at.to_rfc3339(),
            }),
            error: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some("Contract not found".to_string()),
        }),
    }
}

/// List all deployed contracts
pub async fn list_contracts(data: web::Data<ContractState>) -> impl Responder {
    let runtime = data.runtime.read().await;
    let contracts = runtime.list_contracts();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(contracts),
        error: None,
    })
}

/// Get contract storage value
pub async fn get_storage(
    data: web::Data<ContractState>,
    req: web::Json<GetStorageRequest>,
) -> impl Responder {
    let key = match hex::decode(&req.key) {
        Ok(k) => k,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid hex key: {}", e)),
            });
        }
    };

    let runtime = data.runtime.read().await;

    let value = runtime.get_storage(&req.contract, &key);

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(StorageResponse {
            key: req.key.clone(),
            value: value.map(|v| hex::encode(&v)),
        }),
        error: None,
    })
}

/// Configure contract routes
pub fn configure_contract_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/contracts")
            .route("/deploy", web::post().to(deploy_contract))
            .route("/call", web::post().to(call_contract))
            .route("/list", web::get().to(list_contracts))
            .route("/storage", web::post().to(get_storage))
            .route("/{address}", web::get().to(get_contract)),
    );
}
