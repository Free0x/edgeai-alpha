//! WASM Runtime for EdgeAI Smart Contracts
//!
//! This module provides a WebAssembly execution environment for smart contracts.
//! It uses Wasmtime as the WASM runtime and implements gas metering for resource control.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Engine, Instance, Linker, Memory, MemoryType, Module, Store, Val};

/// Gas costs for different operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasCosts {
    /// Base cost per WASM instruction
    pub instruction: u64,
    /// Cost per byte of memory allocation
    pub memory_byte: u64,
    /// Cost for storage read
    pub storage_read: u64,
    /// Cost for storage write
    pub storage_write: u64,
    /// Cost for external call
    pub external_call: u64,
    /// Cost for logging
    pub log: u64,
    /// Cost for hashing (per 64 bytes)
    pub hash: u64,
    /// Cost for signature verification
    pub verify_signature: u64,
}

impl Default for GasCosts {
    fn default() -> Self {
        GasCosts {
            instruction: 1,
            memory_byte: 3,
            storage_read: 200,
            storage_write: 5000,
            external_call: 2500,
            log: 375,
            hash: 30,
            verify_signature: 3000,
        }
    }
}

/// Gas meter for tracking and limiting resource usage
#[derive(Debug, Clone)]
pub struct GasMeter {
    /// Gas limit for the execution
    pub limit: u64,
    /// Gas used so far
    pub used: u64,
    /// Gas costs configuration
    pub costs: GasCosts,
}

impl GasMeter {
    pub fn new(limit: u64) -> Self {
        GasMeter {
            limit,
            used: 0,
            costs: GasCosts::default(),
        }
    }

    pub fn with_costs(limit: u64, costs: GasCosts) -> Self {
        GasMeter {
            limit,
            used: 0,
            costs,
        }
    }

    /// Consume gas, returns error if limit exceeded
    pub fn consume(&mut self, amount: u64) -> Result<(), WasmError> {
        self.used = self.used.saturating_add(amount);
        if self.used > self.limit {
            Err(WasmError::OutOfGas {
                used: self.used,
                limit: self.limit,
            })
        } else {
            Ok(())
        }
    }

    /// Get remaining gas
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    /// Consume gas for storage read
    pub fn consume_storage_read(&mut self) -> Result<(), WasmError> {
        self.consume(self.costs.storage_read)
    }

    /// Consume gas for storage write
    pub fn consume_storage_write(&mut self) -> Result<(), WasmError> {
        self.consume(self.costs.storage_write)
    }
}

/// WASM execution errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmError {
    /// Compilation error
    CompilationError(String),
    /// Runtime error
    RuntimeError(String),
    /// Out of gas
    OutOfGas { used: u64, limit: u64 },
    /// Invalid contract
    InvalidContract(String),
    /// Memory access error
    MemoryError(String),
    /// Import error
    ImportError(String),
    /// Function not found
    FunctionNotFound(String),
    /// Invalid argument
    InvalidArgument(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::CompilationError(msg) => write!(f, "Compilation error: {}", msg),
            WasmError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            WasmError::OutOfGas { used, limit } => {
                write!(f, "Out of gas: used {} / limit {}", used, limit)
            }
            WasmError::InvalidContract(msg) => write!(f, "Invalid contract: {}", msg),
            WasmError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            WasmError::ImportError(msg) => write!(f, "Import error: {}", msg),
            WasmError::FunctionNotFound(name) => write!(f, "Function not found: {}", name),
            WasmError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
        }
    }
}

/// Contract execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Contract address
    pub contract_address: String,
    /// Caller address
    pub caller: String,
    /// Transaction value (tokens sent)
    pub value: u64,
    /// Block height
    pub block_height: u64,
    /// Block timestamp
    pub block_timestamp: i64,
    /// Gas limit
    pub gas_limit: u64,
}

/// Contract execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Return data
    pub return_data: Vec<u8>,
    /// Gas used
    pub gas_used: u64,
    /// Logs emitted
    pub logs: Vec<ContractLog>,
    /// State changes
    pub state_changes: HashMap<String, Vec<u8>>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Contract log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractLog {
    /// Contract address
    pub contract: String,
    /// Log topics (indexed)
    pub topics: Vec<String>,
    /// Log data
    pub data: Vec<u8>,
}

/// Host environment for WASM contracts
#[derive(Clone)]
pub struct HostEnv {
    /// Contract storage
    storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    /// Execution context
    context: ExecutionContext,
    /// Gas meter
    gas_meter: Arc<Mutex<GasMeter>>,
    /// Logs
    logs: Arc<Mutex<Vec<ContractLog>>>,
}

/// Contract ABI (Application Binary Interface)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractAbi {
    /// Contract name
    pub name: String,
    /// Version
    pub version: String,
    /// Functions
    pub functions: Vec<AbiFunction>,
    /// Events
    pub events: Vec<AbiEvent>,
}

/// ABI function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiFunction {
    /// Function name
    pub name: String,
    /// Input parameters
    pub inputs: Vec<AbiParam>,
    /// Output parameters
    pub outputs: Vec<AbiParam>,
    /// Whether function modifies state
    pub mutates: bool,
}

/// ABI parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
}

/// ABI event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiEvent {
    /// Event name
    pub name: String,
    /// Event parameters
    pub params: Vec<AbiParam>,
}

/// Contract info for external queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    /// Contract address
    pub address: String,
    /// Contract ABI
    pub abi: ContractAbi,
    /// Code hash
    pub code_hash: String,
    /// Deployment timestamp
    pub deployed_at: DateTime<Utc>,
}

/// Compiled WASM contract
pub struct CompiledContract {
    /// Contract address
    pub address: String,
    /// Compiled module
    module: Module,
    /// Contract ABI (function signatures)
    pub abi: ContractAbi,
    /// Code hash
    pub code_hash: String,
    /// Compilation timestamp
    pub compiled_at: DateTime<Utc>,
}

/// WASM Runtime for executing smart contracts
pub struct WasmRuntime {
    /// Wasmtime engine
    engine: Engine,
    /// Compiled contracts cache
    contracts: HashMap<String, CompiledContract>,
    /// Contract storage
    storage: HashMap<String, HashMap<Vec<u8>, Vec<u8>>>,
    /// Gas costs configuration
    gas_costs: GasCosts,
}

impl WasmRuntime {
    /// Create a new WASM runtime
    pub fn new() -> Self {
        let engine = Engine::default();
        WasmRuntime {
            engine,
            contracts: HashMap::new(),
            storage: HashMap::new(),
            gas_costs: GasCosts::default(),
        }
    }

    /// Compile and deploy a contract
    pub fn deploy_contract(
        &mut self,
        wasm_code: &[u8],
        owner: &str,
        abi: ContractAbi,
    ) -> Result<String, WasmError> {
        // Compile the WASM module
        let module = Module::new(&self.engine, wasm_code)
            .map_err(|e| WasmError::CompilationError(e.to_string()))?;

        // Generate contract address
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(wasm_code);
        hasher.update(owner.as_bytes());
        hasher.update(Utc::now().timestamp().to_le_bytes());
        let address = format!("0x{}", hex::encode(&hasher.finalize()[..20]));

        // Calculate code hash
        let mut code_hasher = Sha256::new();
        code_hasher.update(wasm_code);
        let code_hash = hex::encode(code_hasher.finalize());

        let compiled = CompiledContract {
            address: address.clone(),
            module,
            abi,
            code_hash,
            compiled_at: Utc::now(),
        };

        // Initialize storage for this contract
        self.storage.insert(address.clone(), HashMap::new());
        self.contracts.insert(address.clone(), compiled);

        info!("Contract deployed at {}", &address);
        Ok(address)
    }

    /// Execute a contract function
    pub fn execute(
        &mut self,
        contract_address: &str,
        function_name: &str,
        args: &[Val],
        context: ExecutionContext,
    ) -> Result<ExecutionResult, WasmError> {
        let contract = self
            .contracts
            .get(contract_address)
            .ok_or_else(|| WasmError::InvalidContract("Contract not found".to_string()))?;

        // Get contract storage
        let storage = self
            .storage
            .get(contract_address)
            .cloned()
            .unwrap_or_default();

        // Create gas meter
        let gas_meter = GasMeter::with_costs(context.gas_limit, self.gas_costs.clone());

        // Create host environment
        let host_env = HostEnv {
            storage: Arc::new(Mutex::new(storage)),
            context: context.clone(),
            gas_meter: Arc::new(Mutex::new(gas_meter)),
            logs: Arc::new(Mutex::new(Vec::new())),
        };

        // Create store with host environment
        let mut store = Store::new(&self.engine, host_env.clone());

        // Create linker and add host functions
        let mut linker = Linker::new(&self.engine);

        // Add host functions
        Self::add_host_functions(&mut linker)?;

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &contract.module)
            .map_err(|e| WasmError::RuntimeError(e.to_string()))?;

        // Get the function
        let func = instance
            .get_func(&mut store, function_name)
            .ok_or_else(|| WasmError::FunctionNotFound(function_name.to_string()))?;

        // Call the function
        let mut results = vec![Val::I64(0)];
        func.call(&mut store, args, &mut results)
            .map_err(|e| WasmError::RuntimeError(e.to_string()))?;

        // Get final state
        let final_storage = host_env.storage.lock().unwrap().clone();
        let final_logs = host_env.logs.lock().unwrap().clone();
        let final_gas = host_env.gas_meter.lock().unwrap().used;

        // Update contract storage
        self.storage
            .insert(contract_address.to_string(), final_storage.clone());

        // Convert storage to state changes
        let state_changes: HashMap<String, Vec<u8>> = final_storage
            .iter()
            .map(|(k, v)| (hex::encode(k), v.clone()))
            .collect();

        Ok(ExecutionResult {
            success: true,
            return_data: results
                .first()
                .map(|v| match v {
                    Val::I64(n) => n.to_le_bytes().to_vec(),
                    Val::I32(n) => n.to_le_bytes().to_vec(),
                    _ => vec![],
                })
                .unwrap_or_default(),
            gas_used: final_gas,
            logs: final_logs,
            state_changes,
            error: None,
        })
    }

    /// Add host functions to the linker
    fn add_host_functions(linker: &mut Linker<HostEnv>) -> Result<(), WasmError> {
        // storage_read(key_ptr, key_len, value_ptr) -> value_len
        linker
            .func_wrap(
                "env",
                "storage_read",
                |mut caller: Caller<'_, HostEnv>,
                 key_ptr: i32,
                 key_len: i32,
                 value_ptr: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return -1,
                    };

                    // Read key from memory
                    let mut key = vec![0u8; key_len as usize];
                    if memory.read(&caller, key_ptr as usize, &mut key).is_err() {
                        return -1;
                    }

                    // Consume gas
                    {
                        let data = caller.data();
                        let mut gas = data.gas_meter.lock().unwrap();
                        if gas.consume_storage_read().is_err() {
                            return -2; // Out of gas
                        }
                    }

                    // Read from storage
                    let value = {
                        let data = caller.data();
                        let storage = data.storage.lock().unwrap();
                        storage.get(&key).cloned()
                    };

                    match value {
                        Some(v) => {
                            // Write value to memory
                            if memory.write(&mut caller, value_ptr as usize, &v).is_err() {
                                return -1;
                            }
                            v.len() as i32
                        }
                        None => 0,
                    }
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // storage_write(key_ptr, key_len, value_ptr, value_len) -> success
        linker
            .func_wrap(
                "env",
                "storage_write",
                |mut caller: Caller<'_, HostEnv>,
                 key_ptr: i32,
                 key_len: i32,
                 value_ptr: i32,
                 value_len: i32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return -1,
                    };

                    // Read key and value from memory
                    let mut key = vec![0u8; key_len as usize];
                    let mut value = vec![0u8; value_len as usize];

                    if memory.read(&caller, key_ptr as usize, &mut key).is_err() {
                        return -1;
                    }
                    if memory
                        .read(&caller, value_ptr as usize, &mut value)
                        .is_err()
                    {
                        return -1;
                    }

                    // Consume gas
                    {
                        let data = caller.data();
                        let mut gas = data.gas_meter.lock().unwrap();
                        if gas.consume_storage_write().is_err() {
                            return -2; // Out of gas
                        }
                    }

                    // Write to storage
                    {
                        let data = caller.data();
                        let mut storage = data.storage.lock().unwrap();
                        storage.insert(key, value);
                    }

                    1 // Success
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // log(msg_ptr, msg_len)
        linker
            .func_wrap(
                "env",
                "log",
                |mut caller: Caller<'_, HostEnv>, msg_ptr: i32, msg_len: i32| {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return,
                    };

                    let mut msg = vec![0u8; msg_len as usize];
                    if memory.read(&caller, msg_ptr as usize, &mut msg).is_err() {
                        return;
                    }

                    let data = caller.data();

                    // Consume gas
                    {
                        let mut gas = data.gas_meter.lock().unwrap();
                        let log_cost = gas.costs.log;
                        let _ = gas.consume(log_cost);
                    }

                    // Add log
                    {
                        let mut logs = data.logs.lock().unwrap();
                        logs.push(ContractLog {
                            contract: data.context.contract_address.clone(),
                            topics: vec!["log".to_string()],
                            data: msg,
                        });
                    }
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // get_caller(ptr) -> len
        linker
            .func_wrap(
                "env",
                "get_caller",
                |mut caller: Caller<'_, HostEnv>, ptr: i32| -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return -1,
                    };

                    let caller_addr = {
                        let data = caller.data();
                        data.context.caller.clone()
                    };
                    let caller_bytes = caller_addr.as_bytes();

                    if memory
                        .write(&mut caller, ptr as usize, caller_bytes)
                        .is_err()
                    {
                        return -1;
                    }

                    caller_bytes.len() as i32
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // get_block_height() -> u64
        linker
            .func_wrap(
                "env",
                "get_block_height",
                |caller: Caller<'_, HostEnv>| -> i64 {
                    let data = caller.data();
                    data.context.block_height as i64
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // get_block_timestamp() -> i64
        linker
            .func_wrap(
                "env",
                "get_block_timestamp",
                |caller: Caller<'_, HostEnv>| -> i64 {
                    let data = caller.data();
                    data.context.block_timestamp
                },
            )
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        // get_value() -> u64
        linker
            .func_wrap("env", "get_value", |caller: Caller<'_, HostEnv>| -> i64 {
                let data = caller.data();
                data.context.value as i64
            })
            .map_err(|e| WasmError::ImportError(e.to_string()))?;

        Ok(())
    }

    /// Get contract info
    pub fn get_contract(&self, address: &str) -> Option<ContractInfo> {
        self.contracts.get(address).map(|c| ContractInfo {
            address: c.address.clone(),
            abi: c.abi.clone(),
            code_hash: c.code_hash.clone(),
            deployed_at: c.compiled_at,
        })
    }

    /// List all contracts
    pub fn list_contracts(&self) -> Vec<ContractInfo> {
        self.contracts
            .values()
            .map(|c| ContractInfo {
                address: c.address.clone(),
                abi: c.abi.clone(),
                code_hash: c.code_hash.clone(),
                deployed_at: c.compiled_at,
            })
            .collect()
    }

    /// Get contract storage
    pub fn get_storage(&self, address: &str, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.get(address)?.get(key).cloned()
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}
