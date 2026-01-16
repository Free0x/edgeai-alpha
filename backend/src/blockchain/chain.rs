//! Blockchain chain module for EdgeAI
//!
//! This module contains the core blockchain logic including chain state,
//! block management, and transaction processing.

#![allow(dead_code)]

use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use log::{info, error, warn};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use rayon::prelude::*;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::{Transaction, TransactionType};
use crate::blockchain::storage::Storage;

const DATA_DIR: &str = "/data";
const BLOCKS_FILE: &str = "blocks.jsonl";  // JSON Lines format for append-only
const STATE_FILE: &str = "state.json";     // Separate state file
const MAX_BLOCKS_IN_MEMORY: usize = 100;   // Only keep recent blocks in RAM

/// Account state in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub nonce: u64,
    pub data_contributions: u64,
    pub reputation_score: f64,
    pub staked_amount: u64,
}

impl Account {
    pub fn new(address: String) -> Self {
        Account {
            address,
            balance: 0,
            nonce: 0,
            data_contributions: 0,
            reputation_score: 0.0,
            staked_amount: 0,
        }
    }
}

/// Blockchain state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub accounts: HashMap<String, Account>,
    pub data_registry: HashMap<String, DataEntry>,  // data_hash -> DataEntry
    pub total_supply: u64,
    pub total_staked: u64,
}

/// Data entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
    pub hash: String,
    pub owner: String,
    pub price: u64,
    pub quality_score: f64,
    pub timestamp: i64,
    pub purchases: u64,
    pub category: String,
}

/// Metadata for blockchain persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    pub total_blocks: u64,
    pub difficulty: u64,
    pub block_reward: u64,
    pub data_reward_base: u64,
    pub last_block_time: i64,
}

/// The main blockchain structure - optimized for memory efficiency
/// Now uses RocksDB for persistent storage with file-based fallback
#[derive(Serialize, Deserialize)]
pub struct Blockchain {
    /// Only keep recent blocks in memory for API queries
    #[serde(skip)]
    pub chain: Vec<Block>,
    #[serde(skip)]
    pub pending_transactions: Vec<Transaction>,
    /// RocksDB storage backend (primary)
    #[serde(skip)]
    storage: Option<Storage>,
    pub state: ChainState,
    pub difficulty: u64,
    pub block_reward: u64,
    pub data_reward_base: u64,
    pub last_block_time: i64,
    /// Total number of blocks (including those on disk)
    #[serde(default)]
    pub total_blocks: u64,
}

impl Blockchain {
    /// Create a new blockchain with genesis block or load from disk
    pub fn new() -> Self {
        // Try to load from disk first
        if let Some(chain) = Self::load_from_disk() {
            info!("Blockchain loaded from disk with {} total blocks ({} in memory)", 
                  chain.total_blocks, chain.chain.len());
            return chain;
        }

        info!("No existing blockchain found, creating new genesis chain");
        let genesis = Block::genesis();
        
        let mut accounts = HashMap::new();
        // Initialize genesis account
        accounts.insert("genesis".to_string(), Account {
            address: "genesis".to_string(),
            balance: 1_000_000_000,
            nonce: 0,
            data_contributions: 0,
            reputation_score: 100.0,
            staked_amount: 0,
        });
        
        // Initialize simulated IoT device accounts with 100 EDGE each
        let simulated_devices = [
            "edge_node_001", "edge_node_002", "edge_node_003",
            "edge_node_004", "edge_node_005", "edge_node_006",
            "edge_node_007", "edge_node_008", "edge_node_009",
            "edge_node_010", "factory_hub_a", "factory_hub_b",
            "city_gateway", "agri_node_1", "med_device_1",
            "power_grid_01", "transit_hub", "warehouse_sys",
        ];
        
        for device in simulated_devices.iter() {
            accounts.insert(device.to_string(), Account {
                address: device.to_string(),
                balance: 100,
                nonce: 0,
                data_contributions: 0,
                reputation_score: 50.0,
                staked_amount: 0,
            });
        }
        info!("Initialized {} simulated device accounts with 100 EDGE each", simulated_devices.len());
        
        let state = ChainState {
            accounts,
            data_registry: HashMap::new(),
            total_supply: 1_000_000_000,
            total_staked: 0,
        };
        
        info!("Blockchain initialized with genesis block");
        
        // Initialize RocksDB storage
        let storage = match Storage::open(DATA_DIR) {
            Ok(s) => {
                info!("RocksDB storage initialized");
                Some(s)
            }
            Err(e) => {
                warn!("Failed to initialize RocksDB: {}, falling back to file storage", e);
                None
            }
        };
        
        let chain = Blockchain {
            chain: vec![genesis.clone()],
            pending_transactions: Vec::new(),
            storage,
            state,
            difficulty: 2,
            block_reward: 100,
            data_reward_base: 50,
            last_block_time: Utc::now().timestamp(),
            total_blocks: 1,
        };

        // Save initial state to both RocksDB and file (for compatibility)
        chain.persist_block(&genesis);
        chain.persist_state();
        chain
    }

    /// Load blockchain from disk - memory efficient version
    /// Priority: RocksDB > New file format > Legacy format
    fn load_from_disk() -> Option<Self> {
        // Try RocksDB first (primary storage)
        if let Some(chain) = Self::load_from_rocksdb() {
            info!("Loaded blockchain from RocksDB");
            return Some(chain);
        }
        
        let state_path = Path::new(DATA_DIR).join(STATE_FILE);
        let blocks_path = Path::new(DATA_DIR).join(BLOCKS_FILE);
        
        // Try file format and migrate to RocksDB
        if state_path.exists() && blocks_path.exists() {
            info!("Loading from file format and migrating to RocksDB...");
            return Self::load_new_format_and_migrate();
        }
        
        // Fall back to legacy format
        let legacy_path = Path::new(DATA_DIR).join("chain.json");
        if legacy_path.exists() {
            info!("Migrating from legacy chain.json format...");
            return Self::load_and_migrate_legacy();
        }
        
        None
    }
    
    /// Load from RocksDB storage
    fn load_from_rocksdb() -> Option<Self> {
        let storage = Storage::open(DATA_DIR).ok()?;
        
        // Check if RocksDB has data
        let metadata = storage.get_metadata()?;
        if metadata.total_blocks == 0 {
            return None;
        }
        
        // Load recent blocks into memory
        let recent_blocks = storage.get_recent_blocks(MAX_BLOCKS_IN_MEMORY);
        if recent_blocks.is_empty() {
            return None;
        }
        
        // Reconstruct state from RocksDB
        // For now, we still keep state in memory for fast access
        // Future optimization: lazy load accounts from RocksDB
        let total_supply = storage.get_total_supply();
        let total_staked = storage.get_total_staked();
        
        // Load accounts from file (RocksDB account loading is optional optimization)
        let state_path = Path::new(DATA_DIR).join(STATE_FILE);
        let state = if state_path.exists() {
            if let Ok(data) = fs::read_to_string(&state_path) {
                if let Ok((s, _)) = serde_json::from_str::<(ChainState, ChainMetadata)>(&data) {
                    s
                } else {
                    ChainState {
                        accounts: HashMap::new(),
                        data_registry: HashMap::new(),
                        total_supply,
                        total_staked,
                    }
                }
            } else {
                ChainState {
                    accounts: HashMap::new(),
                    data_registry: HashMap::new(),
                    total_supply,
                    total_staked,
                }
            }
        } else {
            ChainState {
                accounts: HashMap::new(),
                data_registry: HashMap::new(),
                total_supply,
                total_staked,
            }
        };
        
        let mut chain = Blockchain {
            chain: recent_blocks,
            pending_transactions: Vec::new(),
            storage: Some(storage),
            state,
            difficulty: metadata.difficulty,
            block_reward: metadata.block_reward,
            data_reward_base: metadata.data_reward_base,
            last_block_time: metadata.last_block_time,
            total_blocks: metadata.total_blocks,
        };
        
        chain.ensure_device_accounts();
        Some(chain)
    }
    
    /// Load from new optimized format and migrate to RocksDB
    fn load_new_format_and_migrate() -> Option<Self> {
        let state_path = Path::new(DATA_DIR).join(STATE_FILE);
        let blocks_path = Path::new(DATA_DIR).join(BLOCKS_FILE);
        
        // Load state
        let state_data = fs::read_to_string(&state_path).ok()?;
        let (state, metadata): (ChainState, ChainMetadata) = serde_json::from_str(&state_data).ok()?;
        
        // Initialize RocksDB and migrate blocks
        let storage = match Storage::open(DATA_DIR) {
            Ok(s) => {
                info!("Migrating {} blocks to RocksDB...", metadata.total_blocks);
                
                // Read all blocks from JSONL and write to RocksDB
                if let Ok(file) = fs::File::open(&blocks_path) {
                    let reader = BufReader::new(file);
                    let mut migrated = 0u64;
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if let Ok(block) = serde_json::from_str::<Block>(&line) {
                                if let Err(e) = s.put_block(&block) {
                                    warn!("Failed to migrate block {}: {}", block.index, e);
                                } else {
                                    migrated += 1;
                                }
                            }
                        }
                    }
                    info!("Migrated {} blocks to RocksDB", migrated);
                }
                
                // Save metadata to RocksDB
                if let Err(e) = s.put_metadata(&metadata) {
                    warn!("Failed to save metadata to RocksDB: {}", e);
                }
                
                // Save accounts to RocksDB
                if let Err(e) = s.put_accounts_batch(&state.accounts) {
                    warn!("Failed to save accounts to RocksDB: {}", e);
                }
                
                // Save supply info
                if let Err(e) = s.put_supply_info(state.total_supply, state.total_staked) {
                    warn!("Failed to save supply info to RocksDB: {}", e);
                }
                
                Some(s)
            }
            Err(e) => {
                warn!("Failed to initialize RocksDB for migration: {}", e);
                None
            }
        };
        
        // Load only the last N blocks into memory
        let recent_blocks = Self::load_recent_blocks(&blocks_path, MAX_BLOCKS_IN_MEMORY)?;
        
        let mut chain = Blockchain {
            chain: recent_blocks,
            pending_transactions: Vec::new(),
            storage,
            state,
            difficulty: metadata.difficulty,
            block_reward: metadata.block_reward,
            data_reward_base: metadata.data_reward_base,
            last_block_time: metadata.last_block_time,
            total_blocks: metadata.total_blocks,
        };
        
        // Ensure simulated device accounts exist
        chain.ensure_device_accounts();
        
        Some(chain)
    }
    
    /// Load recent blocks from JSONL file
    fn load_recent_blocks(path: &Path, count: usize) -> Option<Vec<Block>> {
        let file = fs::File::open(path).ok()?;
        let reader = BufReader::new(file);
        
        // Read all lines and keep only the last N
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        let start = if lines.len() > count { lines.len() - count } else { 0 };
        
        let blocks: Vec<Block> = lines[start..]
            .iter()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        
        if blocks.is_empty() {
            None
        } else {
            Some(blocks)
        }
    }
    
    /// Load and migrate from legacy format
    fn load_and_migrate_legacy() -> Option<Self> {
        let legacy_path = Path::new(DATA_DIR).join("chain.json");
        let data = fs::read_to_string(&legacy_path).ok()?;
        
        #[derive(Deserialize)]
        struct LegacyBlockchain {
            chain: Vec<Block>,
            state: ChainState,
            difficulty: u64,
            block_reward: u64,
            data_reward_base: u64,
            last_block_time: i64,
        }
        
        let legacy: LegacyBlockchain = serde_json::from_str(&data).ok()?;
        let total_blocks = legacy.chain.len() as u64;
        
        // Write all blocks to new format
        let blocks_path = Path::new(DATA_DIR).join(BLOCKS_FILE);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&blocks_path) 
        {
            for block in &legacy.chain {
                if let Ok(json) = serde_json::to_string(block) {
                    let _ = writeln!(file, "{}", json);
                }
            }
        }
        
        // Keep only recent blocks in memory
        let recent_start = if legacy.chain.len() > MAX_BLOCKS_IN_MEMORY {
            legacy.chain.len() - MAX_BLOCKS_IN_MEMORY
        } else {
            0
        };
        let recent_blocks: Vec<Block> = legacy.chain[recent_start..].to_vec();
        
        // Initialize RocksDB and migrate
        let storage = match Storage::open(DATA_DIR) {
            Ok(s) => {
                info!("Migrating {} legacy blocks to RocksDB...", total_blocks);
                for block in &legacy.chain {
                    if let Err(e) = s.put_block(block) {
                        warn!("Failed to migrate block {}: {}", block.index, e);
                    }
                }
                
                let metadata = ChainMetadata {
                    total_blocks,
                    difficulty: legacy.difficulty,
                    block_reward: legacy.block_reward,
                    data_reward_base: legacy.data_reward_base,
                    last_block_time: legacy.last_block_time,
                };
                let _ = s.put_metadata(&metadata);
                let _ = s.put_accounts_batch(&legacy.state.accounts);
                let _ = s.put_supply_info(legacy.state.total_supply, legacy.state.total_staked);
                
                Some(s)
            }
            Err(e) => {
                warn!("Failed to initialize RocksDB: {}", e);
                None
            }
        };
        
        let mut chain = Blockchain {
            chain: recent_blocks,
            pending_transactions: Vec::new(),
            storage,
            state: legacy.state,
            difficulty: legacy.difficulty,
            block_reward: legacy.block_reward,
            data_reward_base: legacy.data_reward_base,
            last_block_time: legacy.last_block_time,
            total_blocks,
        };
        
        // Save state in new format
        chain.save_state_to_disk();
        
        // Remove legacy file
        let _ = fs::remove_file(&legacy_path);
        info!("Migration complete: {} blocks migrated to RocksDB", total_blocks);
        
        chain.ensure_device_accounts();
        Some(chain)
    }
    
    /// Ensure simulated device accounts exist
    fn ensure_device_accounts(&mut self) {
        let simulated_devices = [
            "edge_node_001", "edge_node_002", "edge_node_003",
            "edge_node_004", "edge_node_005", "edge_node_006",
            "edge_node_007", "edge_node_008", "edge_node_009",
            "edge_node_010", "factory_hub_a", "factory_hub_b",
            "city_gateway", "agri_node_1", "med_device_1",
            "power_grid_01", "transit_hub", "warehouse_sys",
        ];
        
        let mut initialized_count = 0;
        for device in simulated_devices.iter() {
            if !self.state.accounts.contains_key(*device) {
                self.state.accounts.insert(device.to_string(), Account {
                    address: device.to_string(),
                    balance: 100,
                    nonce: 0,
                    data_contributions: 0,
                    reputation_score: 50.0,
                    staked_amount: 0,
                });
                initialized_count += 1;
            }
        }
        if initialized_count > 0 {
            info!("Initialized {} missing device accounts with 100 EDGE", initialized_count);
        }
    }

    /// Append a single block to disk (memory efficient)
    fn append_block_to_disk(&self, block: &Block) {
        if let Err(e) = fs::create_dir_all(DATA_DIR) {
            error!("Failed to create data directory: {}", e);
            return;
        }

        let blocks_path = Path::new(DATA_DIR).join(BLOCKS_FILE);
        
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&blocks_path) 
        {
            Ok(mut file) => {
                match serde_json::to_string(block) {
                    Ok(json) => {
                        if let Err(e) = writeln!(file, "{}", json) {
                            error!("Failed to append block to disk: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("Failed to serialize block: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to open blocks file: {}", e);
            }
        }
    }
    
    /// Save state to disk (separate from blocks)
    fn save_state_to_disk(&self) {
        if let Err(e) = fs::create_dir_all(DATA_DIR) {
            error!("Failed to create data directory: {}", e);
            return;
        }

        let state_path = Path::new(DATA_DIR).join(STATE_FILE);
        
        let metadata = ChainMetadata {
            total_blocks: self.total_blocks,
            difficulty: self.difficulty,
            block_reward: self.block_reward,
            data_reward_base: self.data_reward_base,
            last_block_time: self.last_block_time,
        };
        
        match serde_json::to_string(&(&self.state, &metadata)) {
            Ok(data) => {
                if let Err(e) = fs::write(&state_path, data) {
                    error!("Failed to write state to disk: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to serialize state: {}", e);
            }
        }
    }

    /// Legacy save_to_disk for compatibility - now uses optimized storage
    pub fn save_to_disk(&self) {
        self.persist_state();
    }
    
    /// Persist a block to storage (RocksDB primary, file fallback)
    fn persist_block(&self, block: &Block) {
        // Write to RocksDB if available
        if let Some(ref storage) = self.storage {
            if let Err(e) = storage.put_block(block) {
                error!("Failed to write block to RocksDB: {}", e);
            }
        }
        
        // Also write to file for compatibility during migration period
        self.append_block_to_disk(block);
    }
    
    /// Persist state to storage (RocksDB primary, file fallback)
    fn persist_state(&self) {
        // Write to RocksDB if available
        if let Some(ref storage) = self.storage {
            let metadata = ChainMetadata {
                total_blocks: self.total_blocks,
                difficulty: self.difficulty,
                block_reward: self.block_reward,
                data_reward_base: self.data_reward_base,
                last_block_time: self.last_block_time,
            };
            
            if let Err(e) = storage.put_metadata(&metadata) {
                error!("Failed to write metadata to RocksDB: {}", e);
            }
            
            if let Err(e) = storage.put_supply_info(self.state.total_supply, self.state.total_staked) {
                error!("Failed to write supply info to RocksDB: {}", e);
            }
            
            // Optionally flush to ensure durability
            if let Err(e) = storage.flush() {
                warn!("Failed to flush RocksDB: {}", e);
            }
        }
        
        // Also write to file for compatibility
        self.save_state_to_disk();
    }
    
    /// Prune old blocks from memory to prevent OOM
    fn prune_memory(&mut self) {
        if self.chain.len() > MAX_BLOCKS_IN_MEMORY {
            let excess = self.chain.len() - MAX_BLOCKS_IN_MEMORY;
            self.chain.drain(0..excess);
            info!("Pruned {} old blocks from memory, {} blocks remain", excess, self.chain.len());
        }
    }
    
    /// Prune old blocks from disk to save space
    /// Keeps only the last KEEP_FULL_BLOCKS blocks with full transaction data
    fn prune_disk_blocks(&self) {
        const KEEP_FULL_BLOCKS: u64 = 10000; // Keep last 10k blocks with full data
        
        if let Some(ref storage) = self.storage {
            match storage.prune_old_blocks(self.total_blocks, KEEP_FULL_BLOCKS) {
                Ok(pruned) => {
                    if pruned > 0 {
                        info!("Pruned {} old blocks from disk", pruned);
                        // Trigger compaction to reclaim space
                        if let Err(e) = storage.compact() {
                            warn!("Failed to compact RocksDB: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to prune old blocks: {}", e);
                }
            }
        }
    }
    
    /// Get the latest block
    pub fn latest_block(&self) -> &Block {
        self.chain.last().unwrap()
    }
    
    /// Get block by index - may need to load from disk for old blocks
    pub fn get_block(&self, index: u64) -> Option<&Block> {
        // Check if block is in memory
        if let Some(first_in_memory) = self.chain.first() {
            if index >= first_in_memory.index {
                let offset = (index - first_in_memory.index) as usize;
                return self.chain.get(offset);
            }
        }
        // Block is not in memory - would need disk access
        // For now, return None for old blocks
        None
    }
    
    /// Get block by index with disk fallback (RocksDB primary, file fallback)
    pub fn get_block_with_disk_fallback(&self, index: u64) -> Option<Block> {
        // Check memory first (fastest)
        if let Some(block) = self.get_block(index) {
            return Some(block.clone());
        }
        
        // Try RocksDB (O(1) lookup)
        if let Some(ref storage) = self.storage {
            if let Some(block) = storage.get_block(index) {
                return Some(block);
            }
        }
        
        // Fall back to file (O(n) scan - legacy compatibility)
        let blocks_path = Path::new(DATA_DIR).join(BLOCKS_FILE);
        if let Ok(file) = fs::File::open(&blocks_path) {
            let reader = BufReader::new(file);
            for (i, line) in reader.lines().enumerate() {
                if i as u64 == index {
                    if let Ok(line) = line {
                        return serde_json::from_str(&line).ok();
                    }
                }
            }
        }
        None
    }
    
    /// Get block by hash
    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.chain.iter().find(|b| b.hash == hash)
    }

    /// Get transaction by hash (returns a clone to avoid lifetime issues)
    /// Now supports RocksDB lookup for O(1) transaction retrieval
    pub fn get_transaction(&self, hash: &str) -> Option<Transaction> {
        // Search in pending transactions first
        if let Some(tx) = self.pending_transactions.iter().find(|tx| tx.hash == hash) {
            return Some(tx.clone());
        }
        
        // Search in in-memory blocks (fast for recent transactions)
        for block in self.chain.iter().rev() {
            if let Some(tx) = block.transactions.iter().find(|tx| tx.hash == hash) {
                return Some(tx.clone());
            }
        }
        
        // Try RocksDB (O(1) lookup for historical transactions)
        if let Some(ref storage) = self.storage {
            if let Some(tx) = storage.get_transaction(hash) {
                return Some(tx);
            }
        }
        
        None
    }
    
    /// Add a transaction to pending pool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<String, String> {
        // Validate transaction hash
        if !tx.verify_hash() {
            log::warn!("Transaction {} failed hash verification (type: {:?})", &tx.hash[..8], tx.tx_type);
            return Err("Invalid transaction hash".to_string());
        }
        
        // Apply validation rules based on transaction type
        match tx.tx_type {
            TransactionType::Transfer => {
                let sender_balance = self.get_balance(&tx.sender);
                let required = tx.total_output();
                if sender_balance < required {
                    log::debug!("Transfer rejected: {} has {} EDGE, needs {}", &tx.sender, sender_balance, required);
                    return Err(format!("Insufficient balance: has {}, needs {}", sender_balance, required));
                }
            },
            TransactionType::DataContribution => {
                // Future: Add data quality validation
            },
            TransactionType::DataPurchase => {
                let sender_balance = self.get_balance(&tx.sender);
                if sender_balance < tx.total_output() {
                    return Err("Insufficient balance".to_string());
                }
            },
            TransactionType::ContractDeploy | TransactionType::ContractCall => {
                // For now, allow contract operations without balance check
            },
            _ => {}
        }
        
        let tx_hash = tx.hash.clone();
        let tx_type = tx.tx_type.clone();
        self.pending_transactions.push(tx);
        info!("Transaction {} added to pending pool (type: {:?})", &tx_hash[..8], tx_type);
        
        Ok(tx_hash)
    }
    
    /// Validate a single transaction (pure function for parallel processing)
    fn validate_transaction_pure(&self, tx: &Transaction) -> Result<(), String> {
        // Validate transaction hash
        if !tx.verify_hash() {
            return Err(format!("Invalid transaction hash: {}", &tx.hash[..8.min(tx.hash.len())]));
        }
        
        // Apply validation rules based on transaction type
        match tx.tx_type {
            TransactionType::Transfer => {
                let sender_balance = self.get_balance(&tx.sender);
                let required = tx.total_output();
                if sender_balance < required {
                    return Err(format!("Insufficient balance: has {}, needs {}", sender_balance, required));
                }
            },
            TransactionType::DataPurchase => {
                let sender_balance = self.get_balance(&tx.sender);
                if sender_balance < tx.total_output() {
                    return Err("Insufficient balance".to_string());
                }
            },
            // DataContribution, ContractDeploy, ContractCall, etc. - no balance check needed
            _ => {}
        }
        
        Ok(())
    }
    
    /// Add multiple transactions in parallel (high-performance batch processing)
    /// Returns (successful_count, failed_count, successful_hashes)
    pub fn add_transactions_batch(&mut self, txs: Vec<Transaction>) -> (usize, usize, Vec<String>) {
        let batch_size = txs.len();
        
        if batch_size == 0 {
            return (0, 0, Vec::new());
        }
        
        // Phase 1: Parallel validation (CPU-intensive hash verification)
        let validation_results: Vec<(Transaction, Result<(), String>)> = txs
            .into_par_iter()
            .map(|tx| {
                let result = self.validate_transaction_pure(&tx);
                (tx, result)
            })
            .collect();
        
        // Phase 2: Sequential insertion (requires mutable access to pending_transactions)
        let mut successful_count = 0;
        let mut failed_count = 0;
        let mut successful_hashes = Vec::new();
        
        for (tx, result) in validation_results {
            match result {
                Ok(()) => {
                    successful_hashes.push(tx.hash.clone());
                    self.pending_transactions.push(tx);
                    successful_count += 1;
                },
                Err(e) => {
                    warn!("Batch tx validation failed: {}", e);
                    failed_count += 1;
                }
            }
        }
        
        info!("Batch processed: {}/{} transactions added to pending pool (parallel validation)", 
              successful_count, batch_size);
        
        (successful_count, failed_count, successful_hashes)
    }
    
    /// Mine a new block with pending transactions
    pub fn mine_block(&mut self, validator: String) -> Result<Block, String> {
        let previous_hash = self.latest_block().hash.clone();
        let index = self.total_blocks;  // Use total_blocks instead of chain.len()
        
        // Select transactions for the block (max 150 for Phase 1)
        let transactions: Vec<Transaction> = self.pending_transactions
            .drain(..self.pending_transactions.len().min(150))
            .collect();
        
        // Create block reward transaction
        let reward_tx = Transaction::reward(
            validator.clone(),
            self.block_reward,
            format!("Block {} mining reward", index),
        );
        
        let mut block_txs = vec![reward_tx];
        block_txs.extend(transactions);
        
        // Calculate PoIE adjusted difficulty
        let data_entropy = Block::calculate_data_entropy(&block_txs);
        let entropy_bonus = (data_entropy * 0.5) as u64; 
        let base_difficulty = 2;
        
        let adjusted_difficulty = if base_difficulty > entropy_bonus {
            base_difficulty - entropy_bonus
        } else {
            1
        };

        info!("Mining block {} with PoIE difficulty: {} (Base: {}, Entropy Bonus: {})", 
            index, adjusted_difficulty, base_difficulty, entropy_bonus);

        // Create and mine the block
        let mut block = Block::new(
            index,
            previous_hash,
            block_txs,
            adjusted_difficulty,
            validator.clone(),
        );
        
        block.mine(adjusted_difficulty);
        
        self.last_block_time = Utc::now().timestamp();
        
        // Apply block to state
        self.apply_block(&block)?;
        
        // Add block to in-memory chain
        self.chain.push(block.clone());
        self.total_blocks += 1;
        
        info!("Block {} mined by {} ({} blocks in memory)", 
              index, &validator[..8.min(validator.len())], self.chain.len());
        
        // Persist block to RocksDB and file (dual-write for migration period)
        self.persist_block(&block);
        
        // Save state periodically (every 10 blocks to reduce I/O)
        if self.total_blocks % 10 == 0 {
            self.persist_state();
        }
        
        // Prune old blocks from memory to prevent OOM
        self.prune_memory();
        
        // Prune old blocks from disk every 1000 blocks to save space
        if self.total_blocks % 1000 == 0 {
            self.prune_disk_blocks();
        }
        
        Ok(block)
    }
    
    /// Apply block transactions to state
    fn apply_block(&mut self, block: &Block) -> Result<(), String> {
        for tx in &block.transactions {
            if let Err(e) = self.apply_transaction(tx) {
                log::warn!("Transaction {} failed to apply: {} (skipping)", &tx.hash[..8], e);
                continue;
            }
        }
        Ok(())
    }
    
    /// Apply a single transaction to state
    fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        match tx.tx_type {
            TransactionType::Transfer => {
                self.transfer(&tx.sender, &tx.outputs[0].recipient, tx.outputs[0].amount)?;
            }
            TransactionType::DataContribution => {
                self.process_data_contribution(tx)?;
            }
            TransactionType::DataPurchase => {
                self.process_data_purchase(tx)?;
            }
            TransactionType::Reward => {
                self.process_reward(tx)?;
            }
            TransactionType::Stake => {
                self.process_stake(tx)?;
            }
            TransactionType::Unstake => {
                self.process_unstake(tx)?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Transfer tokens between accounts
    fn transfer(&mut self, from: &str, to: &str, amount: u64) -> Result<(), String> {
        // Get or create sender account
        let sender = self.state.accounts.entry(from.to_string())
            .or_insert_with(|| Account::new(from.to_string()));
        
        if sender.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        sender.balance -= amount;
        sender.nonce += 1;
        
        // Get or create recipient account
        let recipient = self.state.accounts.entry(to.to_string())
            .or_insert_with(|| Account::new(to.to_string()));
        recipient.balance += amount;
        
        Ok(())
    }
    
    /// Process data contribution (PoIE reward)
    fn process_data_contribution(&mut self, tx: &Transaction) -> Result<(), String> {
        let device = &tx.sender;
        let reward = tx.outputs.get(0).map(|o| o.amount).unwrap_or(0);
        
        // Get or create device account
        let account = self.state.accounts.entry(device.to_string())
            .or_insert_with(|| Account::new(device.to_string()));
        
        account.balance += reward;
        account.data_contributions += 1;
        account.reputation_score = (account.reputation_score + 0.1).min(100.0);
        
        // Register data if hash provided
        if let Some(output) = tx.outputs.get(0) {
            if let Some(data_hash) = &output.data_hash {
                let quality = tx.data_quality.as_ref()
                    .map(|q| q.overall_score)
                    .unwrap_or(0.5);
                
                self.state.data_registry.insert(data_hash.clone(), DataEntry {
                    hash: data_hash.clone(),
                    owner: device.to_string(),
                    price: 10,
                    quality_score: quality,
                    timestamp: Utc::now().timestamp(),
                    purchases: 0,
                    category: "IoT".to_string(),
                });
            }
        }
        
        self.state.total_supply += reward;
        
        Ok(())
    }
    
    /// Process reward transaction
    fn process_reward(&mut self, tx: &Transaction) -> Result<(), String> {
        for output in &tx.outputs {
            let account = self.state.accounts.entry(output.recipient.clone())
                .or_insert_with(|| Account::new(output.recipient.clone()));
            account.balance += output.amount;
        }
        self.state.total_supply += tx.total_output();
        Ok(())
    }
    
    /// Process data purchase
    fn process_data_purchase(&mut self, tx: &Transaction) -> Result<(), String> {
        let buyer = &tx.sender;
        let amount = tx.total_output();
        
        // Deduct from buyer
        let buyer_account = self.state.accounts.get_mut(buyer)
            .ok_or("Buyer account not found")?;
        
        if buyer_account.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        buyer_account.balance -= amount;
        
        // Pay seller
        for output in &tx.outputs {
            let seller_account = self.state.accounts.entry(output.recipient.clone())
                .or_insert_with(|| Account::new(output.recipient.clone()));
            seller_account.balance += output.amount;
            
            // Update data entry if exists
            if let Some(data_hash) = &output.data_hash {
                if let Some(entry) = self.state.data_registry.get_mut(data_hash) {
                    entry.purchases += 1;
                }
            }
        }
        
        Ok(())
    }
    
    /// Process stake
    fn process_stake(&mut self, tx: &Transaction) -> Result<(), String> {
        let amount = tx.outputs[0].amount;
        
        let account = self.state.accounts.get_mut(&tx.sender)
            .ok_or("Account not found")?;
        
        if account.balance < amount {
            return Err("Insufficient balance for staking".to_string());
        }
        
        account.balance -= amount;
        account.staked_amount += amount;
        self.state.total_staked += amount;
        
        Ok(())
    }
    
    /// Process unstake
    fn process_unstake(&mut self, tx: &Transaction) -> Result<(), String> {
        let amount = tx.outputs[0].amount;
        
        let account = self.state.accounts.get_mut(&tx.sender)
            .ok_or("Account not found")?;
        
        if account.staked_amount < amount {
            return Err("Insufficient staked amount".to_string());
        }
        
        account.staked_amount -= amount;
        account.balance += amount;
        self.state.total_staked -= amount;
        
        Ok(())
    }
    
    /// Get account state
    pub fn get_account(&self, address: &str) -> Option<&Account> {
        self.state.accounts.get(address)
    }
    
    /// Get account balance (read-only)
    pub fn get_balance(&self, address: &str) -> u64 {
        self.state.accounts.get(address).map(|a| a.balance).unwrap_or(0)
    }
    
    /// Get transactions for an address (only from in-memory blocks)
    pub fn get_transactions_for_address(&self, address: &str) -> Vec<&Transaction> {
        let mut txs = Vec::new();
        
        for block in &self.chain {
            for tx in &block.transactions {
                if tx.sender == address {
                    txs.push(tx);
                    continue;
                }
                
                for output in &tx.outputs {
                    if output.recipient == address {
                        txs.push(tx);
                        break;
                    }
                }
            }
        }
        
        txs
    }
    
    /// Get blockchain stats with PoIE network metrics
    pub fn get_stats(&self) -> ChainStats {
        let height = self.total_blocks;
        let total_transactions: u64 = self.chain.iter().map(|b| b.transactions.len() as u64).sum();
        
        // Estimate total transactions based on average
        let avg_tx_per_block = if !self.chain.is_empty() {
            total_transactions as f64 / self.chain.len() as f64
        } else {
            0.0
        };
        let estimated_total_tx = (avg_tx_per_block * height as f64) as u64;
        
        // Calculate network entropy from in-memory blocks
        let network_entropy: f64 = self.chain.iter()
            .map(|b| b.header.data_entropy)
            .sum();
        
        // Calculate data throughput
        let data_throughput = if height > 0 {
            (estimated_total_tx as f64 * 256.0) / (height as f64 * 10.0)
        } else {
            0.0
        };
        
        // Calculate TPS
        let tps = if height > 0 {
            estimated_total_tx as f64 / (height as f64 * 10.0)
        } else {
            0.0
        };
        
        // Calculate validator power index
        let validator_power = {
            let active = self.state.accounts.len() as f64;
            let data = self.state.data_registry.len() as f64;
            let entropy_factor = network_entropy / (self.chain.len().max(1) as f64);
            (active * 0.3 + data * 0.3 + entropy_factor * 100.0 * 0.4).max(0.0)
        };
        
        ChainStats {
            height,
            total_transactions: estimated_total_tx,
            total_supply: self.state.total_supply,
            total_staked: self.state.total_staked,
            active_accounts: self.state.accounts.len() as u64,
            data_entries: self.state.data_registry.len() as u64,
            difficulty: self.difficulty,
            last_block_time: self.last_block_time,
            network_entropy,
            avg_tx_per_block,
            data_throughput,
            tps,
            validator_power,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainStats {
    pub height: u64,
    pub total_transactions: u64,
    pub total_supply: u64,
    pub total_staked: u64,
    pub active_accounts: u64,
    pub data_entries: u64,
    pub difficulty: u64,
    pub last_block_time: i64,
    // PoIE Network Metrics
    pub network_entropy: f64,
    pub avg_tx_per_block: f64,
    pub data_throughput: f64,
    pub tps: f64,
    pub validator_power: f64,
}
