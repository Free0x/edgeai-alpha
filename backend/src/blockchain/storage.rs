//! RocksDB-based storage layer for EdgeAI Blockchain
//! 
//! This module provides high-performance persistent storage using RocksDB,
//! replacing the previous file-based storage (blocks.jsonl, state.json).
//! 
//! ## Key Features
//! - O(1) block and transaction lookups by index/hash
//! - Efficient account state queries
//! - Atomic batch writes for consistency
//! - Automatic data migration from legacy format

#![allow(dead_code)]

use rocksdb::{DB, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use log::info;
use std::path::Path;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use super::chain::{Account, DataEntry, ChainMetadata};

/// Column family names for organizing data
const CF_BLOCKS: &str = "blocks";           // block_index -> Block (serialized)
const CF_BLOCK_HASHES: &str = "block_hashes"; // block_hash -> block_index
const CF_TRANSACTIONS: &str = "transactions"; // tx_hash -> (block_index, tx_index)
const CF_ACCOUNTS: &str = "accounts";       // address -> Account
const CF_DATA_REGISTRY: &str = "data_registry"; // data_hash -> DataEntry
const CF_METADATA: &str = "metadata";       // key -> value (chain metadata)

/// Keys for metadata
const META_TOTAL_BLOCKS: &[u8] = b"total_blocks";
const META_DIFFICULTY: &[u8] = b"difficulty";
const META_BLOCK_REWARD: &[u8] = b"block_reward";
const META_DATA_REWARD_BASE: &[u8] = b"data_reward_base";
const META_LAST_BLOCK_TIME: &[u8] = b"last_block_time";
const META_TOTAL_SUPPLY: &[u8] = b"total_supply";
const META_TOTAL_STAKED: &[u8] = b"total_staked";

/// RocksDB-based storage engine
pub struct Storage {
    db: DB,
    data_dir: String,
}

impl Storage {
    /// Open or create the storage database
    pub fn open(data_dir: &str) -> Result<Self, String> {
        let db_path = Path::new(data_dir).join("rocksdb");
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        // Memory optimization: reduce cache sizes significantly
        opts.set_max_open_files(64);  // Reduced from 256
        opts.set_write_buffer_size(8 * 1024 * 1024); // 8MB write buffer (reduced from 32MB)
        opts.set_max_write_buffer_number(2);
        opts.set_target_file_size_base(16 * 1024 * 1024); // 16MB SST files (reduced from 32MB)
        opts.set_level_zero_file_num_compaction_trigger(2); // More aggressive compaction
        
        // Disable block cache to save memory (use OS page cache instead)
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(8 * 1024 * 1024)); // 8MB block cache (minimal)
        block_opts.set_cache_index_and_filter_blocks(false); // Don't cache index/filter
        opts.set_block_based_table_factory(&block_opts);
        
        // Compression settings
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd); // Better compression
        opts.set_bottommost_compression_type(rocksdb::DBCompressionType::Zstd);
        opts.set_compression_per_level(&[
            rocksdb::DBCompressionType::Lz4,   // L0: fast compression
            rocksdb::DBCompressionType::Lz4,   // L1: fast compression
            rocksdb::DBCompressionType::Zstd,  // L2+: best compression
            rocksdb::DBCompressionType::Zstd,
            rocksdb::DBCompressionType::Zstd,
            rocksdb::DBCompressionType::Zstd,
        ]);
        opts.set_level_compaction_dynamic_level_bytes(true);
        
        // Additional memory optimizations
        opts.set_optimize_filters_for_hits(true);  // Optimize bloom filters
        opts.set_max_background_jobs(2);  // Limit background threads
        
        // Define column families
        let cf_names = vec![
            CF_BLOCKS, CF_BLOCK_HASHES, CF_TRANSACTIONS, 
            CF_ACCOUNTS, CF_DATA_REGISTRY, CF_METADATA
        ];
        
        // Try to open existing DB with column families
        let db = match DB::open_cf(&opts, &db_path, &cf_names) {
            Ok(db) => db,
            Err(_) => {
                // Create new DB with column families
                info!("Creating new RocksDB database at {:?}", db_path);
                let db = DB::open(&opts, &db_path)
                    .map_err(|e| format!("Failed to open RocksDB: {}", e))?;
                
                // Create column families
                for cf_name in &cf_names {
                    if db.cf_handle(cf_name).is_none() {
                        // Column family will be created on next open
                    }
                }
                
                // Reopen with column families
                drop(db);
                DB::open_cf(&opts, &db_path, &cf_names)
                    .map_err(|e| format!("Failed to open RocksDB with CFs: {}", e))?
            }
        };
        
        info!("RocksDB storage opened at {:?}", db_path);
        
        Ok(Storage {
            db,
            data_dir: data_dir.to_string(),
        })
    }
    
    /// Store a block
    pub fn put_block(&self, block: &Block) -> Result<(), String> {
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)
            .ok_or("CF_BLOCKS not found")?;
        let cf_hashes = self.db.cf_handle(CF_BLOCK_HASHES)
            .ok_or("CF_BLOCK_HASHES not found")?;
        let cf_txs = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or("CF_TRANSACTIONS not found")?;
        
        let mut batch = WriteBatch::default();
        
        // Serialize block
        let block_data = serde_json::to_vec(block)
            .map_err(|e| format!("Failed to serialize block: {}", e))?;
        
        // Store block by index
        let index_key = block.index.to_be_bytes();
        batch.put_cf(&cf_blocks, &index_key, &block_data);
        
        // Store block hash -> index mapping
        batch.put_cf(&cf_hashes, block.hash.as_bytes(), &index_key);
        
        // Store transaction hash -> (block_index, tx_index) mappings
        for (tx_idx, tx) in block.transactions.iter().enumerate() {
            let tx_location = TxLocation {
                block_index: block.index,
                tx_index: tx_idx as u32,
            };
            let location_data = serde_json::to_vec(&tx_location)
                .map_err(|e| format!("Failed to serialize tx location: {}", e))?;
            batch.put_cf(&cf_txs, tx.hash.as_bytes(), &location_data);
        }
        
        self.db.write(batch)
            .map_err(|e| format!("Failed to write block batch: {}", e))?;
        
        Ok(())
    }
    
    /// Get a block by index
    pub fn get_block(&self, index: u64) -> Option<Block> {
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)?;
        let index_key = index.to_be_bytes();
        
        match self.db.get_cf(&cf_blocks, &index_key) {
            Ok(Some(data)) => {
                serde_json::from_slice(&data).ok()
            }
            _ => None
        }
    }
    
    /// Get a block by hash
    pub fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        let cf_hashes = self.db.cf_handle(CF_BLOCK_HASHES)?;
        
        match self.db.get_cf(&cf_hashes, hash.as_bytes()) {
            Ok(Some(index_bytes)) => {
                if index_bytes.len() == 8 {
                    let index = u64::from_be_bytes(index_bytes.try_into().ok()?);
                    self.get_block(index)
                } else {
                    None
                }
            }
            _ => None
        }
    }
    
    /// Get transaction location by hash
    pub fn get_transaction_location(&self, tx_hash: &str) -> Option<TxLocation> {
        let cf_txs = self.db.cf_handle(CF_TRANSACTIONS)?;
        
        match self.db.get_cf(&cf_txs, tx_hash.as_bytes()) {
            Ok(Some(data)) => {
                serde_json::from_slice(&data).ok()
            }
            _ => None
        }
    }
    
    /// Get a transaction by hash
    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        let location = self.get_transaction_location(tx_hash)?;
        let block = self.get_block(location.block_index)?;
        block.transactions.get(location.tx_index as usize).cloned()
    }
    
    /// Store an account
    pub fn put_account(&self, account: &Account) -> Result<(), String> {
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or("CF_ACCOUNTS not found")?;
        
        let data = serde_json::to_vec(account)
            .map_err(|e| format!("Failed to serialize account: {}", e))?;
        
        self.db.put_cf(&cf_accounts, account.address.as_bytes(), &data)
            .map_err(|e| format!("Failed to write account: {}", e))?;
        
        Ok(())
    }
    
    /// Get an account by address
    pub fn get_account(&self, address: &str) -> Option<Account> {
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS)?;
        
        match self.db.get_cf(&cf_accounts, address.as_bytes()) {
            Ok(Some(data)) => {
                serde_json::from_slice(&data).ok()
            }
            _ => None
        }
    }
    
    /// Store a data entry
    pub fn put_data_entry(&self, entry: &DataEntry) -> Result<(), String> {
        let cf_data = self.db.cf_handle(CF_DATA_REGISTRY)
            .ok_or("CF_DATA_REGISTRY not found")?;
        
        let data = serde_json::to_vec(entry)
            .map_err(|e| format!("Failed to serialize data entry: {}", e))?;
        
        self.db.put_cf(&cf_data, entry.hash.as_bytes(), &data)
            .map_err(|e| format!("Failed to write data entry: {}", e))?;
        
        Ok(())
    }
    
    /// Get a data entry by hash
    pub fn get_data_entry(&self, hash: &str) -> Option<DataEntry> {
        let cf_data = self.db.cf_handle(CF_DATA_REGISTRY)?;
        
        match self.db.get_cf(&cf_data, hash.as_bytes()) {
            Ok(Some(data)) => {
                serde_json::from_slice(&data).ok()
            }
            _ => None
        }
    }
    
    /// Store chain metadata
    pub fn put_metadata(&self, metadata: &ChainMetadata) -> Result<(), String> {
        let cf_meta = self.db.cf_handle(CF_METADATA)
            .ok_or("CF_METADATA not found")?;
        
        let mut batch = WriteBatch::default();
        
        batch.put_cf(&cf_meta, META_TOTAL_BLOCKS, &metadata.total_blocks.to_be_bytes());
        batch.put_cf(&cf_meta, META_DIFFICULTY, &metadata.difficulty.to_be_bytes());
        batch.put_cf(&cf_meta, META_BLOCK_REWARD, &metadata.block_reward.to_be_bytes());
        batch.put_cf(&cf_meta, META_DATA_REWARD_BASE, &metadata.data_reward_base.to_be_bytes());
        batch.put_cf(&cf_meta, META_LAST_BLOCK_TIME, &metadata.last_block_time.to_be_bytes());
        
        self.db.write(batch)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;
        
        Ok(())
    }
    
    /// Get chain metadata
    pub fn get_metadata(&self) -> Option<ChainMetadata> {
        let cf_meta = self.db.cf_handle(CF_METADATA)?;
        
        let total_blocks = self.get_u64(&cf_meta, META_TOTAL_BLOCKS)?;
        let difficulty = self.get_u64(&cf_meta, META_DIFFICULTY).unwrap_or(2);
        let block_reward = self.get_u64(&cf_meta, META_BLOCK_REWARD).unwrap_or(100);
        let data_reward_base = self.get_u64(&cf_meta, META_DATA_REWARD_BASE).unwrap_or(10);
        let last_block_time = self.get_i64(&cf_meta, META_LAST_BLOCK_TIME).unwrap_or(0);
        
        Some(ChainMetadata {
            total_blocks,
            difficulty,
            block_reward,
            data_reward_base,
            last_block_time,
        })
    }
    
    /// Store total supply and staked amounts
    pub fn put_supply_info(&self, total_supply: u64, total_staked: u64) -> Result<(), String> {
        let cf_meta = self.db.cf_handle(CF_METADATA)
            .ok_or("CF_METADATA not found")?;
        
        let mut batch = WriteBatch::default();
        batch.put_cf(&cf_meta, META_TOTAL_SUPPLY, &total_supply.to_be_bytes());
        batch.put_cf(&cf_meta, META_TOTAL_STAKED, &total_staked.to_be_bytes());
        
        self.db.write(batch)
            .map_err(|e| format!("Failed to write supply info: {}", e))?;
        
        Ok(())
    }
    
    /// Get total supply
    pub fn get_total_supply(&self) -> u64 {
        let cf_meta = match self.db.cf_handle(CF_METADATA) {
            Some(cf) => cf,
            None => return 0,
        };
        self.get_u64(&cf_meta, META_TOTAL_SUPPLY).unwrap_or(0)
    }
    
    /// Get total staked
    pub fn get_total_staked(&self) -> u64 {
        let cf_meta = match self.db.cf_handle(CF_METADATA) {
            Some(cf) => cf,
            None => return 0,
        };
        self.get_u64(&cf_meta, META_TOTAL_STAKED).unwrap_or(0)
    }
    
    /// Get recent blocks (for API queries)
    pub fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let metadata = match self.get_metadata() {
            Some(m) => m,
            None => return Vec::new(),
        };
        
        let total = metadata.total_blocks;
        let start = if total > count as u64 { total - count as u64 } else { 0 };
        
        let mut blocks = Vec::with_capacity(count);
        for i in start..total {
            if let Some(block) = self.get_block(i) {
                blocks.push(block);
            }
        }
        
        blocks
    }
    
    /// Batch store accounts (for migration)
    pub fn put_accounts_batch(&self, accounts: &std::collections::HashMap<String, Account>) -> Result<(), String> {
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or("CF_ACCOUNTS not found")?;
        
        let mut batch = WriteBatch::default();
        
        for (_, account) in accounts {
            let data = serde_json::to_vec(account)
                .map_err(|e| format!("Failed to serialize account: {}", e))?;
            batch.put_cf(&cf_accounts, account.address.as_bytes(), &data);
        }
        
        self.db.write(batch)
            .map_err(|e| format!("Failed to write accounts batch: {}", e))?;
        
        Ok(())
    }
    
    /// Batch store data entries (for migration)
    pub fn put_data_entries_batch(&self, entries: &std::collections::HashMap<String, DataEntry>) -> Result<(), String> {
        let cf_data = self.db.cf_handle(CF_DATA_REGISTRY)
            .ok_or("CF_DATA_REGISTRY not found")?;
        
        let mut batch = WriteBatch::default();
        
        for (_, entry) in entries {
            let data = serde_json::to_vec(entry)
                .map_err(|e| format!("Failed to serialize data entry: {}", e))?;
            batch.put_cf(&cf_data, entry.hash.as_bytes(), &data);
        }
        
        self.db.write(batch)
            .map_err(|e| format!("Failed to write data entries batch: {}", e))?;
        
        Ok(())
    }
    
    /// Check if database is empty (for migration detection)
    pub fn is_empty(&self) -> bool {
        self.get_metadata().is_none()
    }
    
    /// Flush all pending writes to disk
    pub fn flush(&self) -> Result<(), String> {
        self.db.flush()
            .map_err(|e| format!("Failed to flush RocksDB: {}", e))
    }
    
    // Helper methods
    fn get_u64(&self, cf: &rocksdb::ColumnFamily, key: &[u8]) -> Option<u64> {
        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) if bytes.len() == 8 => {
                Some(u64::from_be_bytes(bytes.try_into().ok()?))
            }
            _ => None
        }
    }
    
    fn get_i64(&self, cf: &rocksdb::ColumnFamily, key: &[u8]) -> Option<i64> {
        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) if bytes.len() == 8 => {
                Some(i64::from_be_bytes(bytes.try_into().ok()?))
            }
            _ => None
        }
    }
    
    /// Prune old blocks to save disk space
    /// - Blocks older than `keep_blocks * 2` are completely deleted
    /// - Blocks between `keep_blocks` and `keep_blocks * 2` are pruned to headers only
    pub fn prune_old_blocks(&self, current_height: u64, keep_blocks: u64) -> Result<usize, String> {
        if current_height <= keep_blocks {
            return Ok(0);
        }
        
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)
            .ok_or("CF_BLOCKS not found")?;
        let cf_txs = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or("CF_TRANSACTIONS not found")?;
        
        let prune_to_header_before = current_height - keep_blocks;
        let delete_before = if current_height > keep_blocks * 2 {
            current_height - keep_blocks * 2
        } else {
            0
        };
        
        let mut pruned_count = 0;
        let mut deleted_count = 0;
        let mut batch = WriteBatch::default();
        
        // Completely delete very old blocks (older than keep_blocks * 2)
        for index in 0..delete_before {
            let index_key = index.to_be_bytes();
            batch.delete_cf(&cf_blocks, &index_key);
            deleted_count += 1;
        }
        
        // Prune blocks to headers only (between keep_blocks and keep_blocks * 2)
        for index in delete_before..prune_to_header_before {
            let index_key = index.to_be_bytes();
            
            if let Ok(Some(block_data)) = self.db.get_cf(&cf_blocks, &index_key) {
                if let Ok(block) = serde_json::from_slice::<Block>(&block_data) {
                    // Skip if already pruned (no transactions)
                    if block.transactions.is_empty() {
                        continue;
                    }
                    
                    // Create a pruned version (header only, no transactions)
                    let pruned_block = Block {
                        index: block.index,
                        header: block.header,
                        transactions: Vec::new(),
                        hash: block.hash,
                        validator: block.validator,
                    };
                    
                    if let Ok(pruned_data) = serde_json::to_vec(&pruned_block) {
                        batch.put_cf(&cf_blocks, &index_key, &pruned_data);
                        pruned_count += 1;
                    }
                }
            }
        }
        
        let total_affected = deleted_count + pruned_count;
        if total_affected > 0 {
            self.db.write(batch)
                .map_err(|e| format!("Failed to write pruned blocks: {}", e))?;
            if deleted_count > 0 {
                info!("Deleted {} very old blocks completely", deleted_count);
            }
            if pruned_count > 0 {
                info!("Pruned {} blocks to headers only", pruned_count);
            }
        }
        
        Ok(total_affected)
    }
    
    /// Trigger manual compaction to reclaim disk space
    pub fn compact(&self) -> Result<(), String> {
        info!("Starting RocksDB compaction...");
        
        // Compact all column families
        let cf_names = vec![
            CF_BLOCKS, CF_BLOCK_HASHES, CF_TRANSACTIONS, 
            CF_ACCOUNTS, CF_DATA_REGISTRY, CF_METADATA
        ];
        
        for cf_name in cf_names {
            if let Some(cf) = self.db.cf_handle(cf_name) {
                self.db.compact_range_cf(&cf, None::<&[u8]>, None::<&[u8]>);
            }
        }
        
        info!("RocksDB compaction completed");
        Ok(())
    }
    
    /// Get database size estimate
    pub fn get_db_size(&self) -> u64 {
        let db_path = Path::new(&self.data_dir).join("rocksdb");
        Self::dir_size(&db_path).unwrap_or(0)
    }
    
    fn dir_size(path: &Path) -> std::io::Result<u64> {
        let mut size = 0;
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    size += Self::dir_size(&path)?;
                } else {
                    size += entry.metadata()?.len();
                }
            }
        }
        Ok(size)
    }
}

/// Transaction location in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLocation {
    pub block_index: u64,
    pub tx_index: u32,
}
