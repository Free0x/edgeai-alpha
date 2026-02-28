//! OceanBase/MySQL storage layer for EdgeAI Blockchain
//!
//! This module provides cloud database storage using OceanBase (MySQL compatible),
//! replacing the local RocksDB + state.json storage for better scalability and
//! reduced memory footprint.
//!
//! ## Key Features
//! - Connection pool management via sqlx
//! - Async CRUD operations for all blockchain data
//! - Graceful fallback if database is unavailable
//! - Batch operations for data migration

#![allow(dead_code)]

use log::{error, info, warn};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::Row;
use std::collections::HashMap;

use super::chain::{Account, ChainMetadata, DataEntry};
use crate::blockchain::block::{Block, BlockHeader};
use crate::blockchain::transaction::{
    DataQuality, Transaction, TransactionType, TxInput, TxOutput,
};

/// OceanBase storage engine
pub struct OceanBaseStorage {
    pool: MySqlPool,
}

impl OceanBaseStorage {
    /// Create a new OceanBase connection pool
    pub async fn connect(database_url: &str) -> Result<Self, String> {
        info!(
            "Connecting to OceanBase: {}...",
            &database_url[..database_url.len().min(50)]
        );

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .connect(database_url)
            .await
            .map_err(|e| format!("Failed to connect to OceanBase: {}", e))?;

        info!("OceanBase connection pool established");

        // Auto-create bridge_requests table if it doesn't exist
        let create_bridge_table = sqlx::query(
            "CREATE TABLE IF NOT EXISTS bridge_requests (\
                request_id VARCHAR(64) PRIMARY KEY, \
                direction VARCHAR(20) NOT NULL, \
                target_chain VARCHAR(20) NOT NULL DEFAULT 'bsc', \
                edge_address VARCHAR(128) NOT NULL, \
                evm_address VARCHAR(128) NOT NULL, \
                amount BIGINT UNSIGNED NOT NULL, \
                fee BIGINT UNSIGNED NOT NULL DEFAULT 0, \
                status VARCHAR(20) NOT NULL DEFAULT 'pending', \
                tx_hash_edge VARCHAR(128), \
                tx_hash_evm VARCHAR(128), \
                admin_note TEXT, \
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, \
                completed_at DATETIME, \
                INDEX idx_status (status), \
                INDEX idx_edge_address (edge_address), \
                INDEX idx_evm_address (evm_address), \
                INDEX idx_created_at (created_at)\
            ) DEFAULT CHARSET=utf8mb4",
        )
        .execute(&pool)
        .await;

        match create_bridge_table {
            Ok(_) => info!("bridge_requests table ensured"),
            Err(e) => warn!(
                "Failed to create bridge_requests table (may already exist): {}",
                e
            ),
        }

        Ok(OceanBaseStorage { pool })
    }

    /// Build database URL from environment variables
    pub fn build_url() -> Option<String> {
        let host = std::env::var("OCEANBASE_HOST").ok()?;
        let port = std::env::var("OCEANBASE_PORT").unwrap_or_else(|_| "3306".to_string());
        let user = std::env::var("OCEANBASE_USER").ok()?;
        let password = std::env::var("OCEANBASE_PASSWORD").ok()?;
        let database =
            std::env::var("OCEANBASE_DATABASE").unwrap_or_else(|_| "default_database".to_string());

        // URL-encode password to handle special characters like $, /, @, etc.
        let encoded_password = password
            .replace('%', "%25")
            .replace('$', "%24")
            .replace('/', "%2F")
            .replace('@', "%40")
            .replace('#', "%23")
            .replace('?', "%3F")
            .replace('&', "%26")
            .replace('=', "%3D")
            .replace(' ', "%20");

        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            user, encoded_password, host, port, database
        );
        info!(
            "OceanBase URL built: mysql://{}:***@{}:{}/{}",
            user, host, port, database
        );
        Some(url)
    }

    // ==================== Chain Metadata ====================

    /// Get chain metadata from database
    pub async fn get_metadata(&self) -> Option<ChainMetadata> {
        let rows = sqlx::query("SELECT meta_key, meta_value FROM chain_metadata")
            .fetch_all(&self.pool)
            .await
            .ok()?;

        let mut meta_map: HashMap<String, String> = HashMap::new();
        for row in rows {
            let key: String = row.get("meta_key");
            let value: String = row.get("meta_value");
            meta_map.insert(key, value);
        }

        let total_blocks = meta_map
            .get("total_blocks")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        if total_blocks == 0 {
            return None;
        }

        Some(ChainMetadata {
            total_blocks,
            difficulty: meta_map
                .get("difficulty")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            block_reward: meta_map
                .get("block_reward")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            data_reward_base: meta_map
                .get("data_reward_base")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            last_block_time: meta_map
                .get("last_block_time")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        })
    }

    /// Save chain metadata
    pub async fn put_metadata(&self, metadata: &ChainMetadata) -> Result<(), String> {
        let pairs = vec![
            ("total_blocks", metadata.total_blocks.to_string()),
            ("difficulty", metadata.difficulty.to_string()),
            ("block_reward", metadata.block_reward.to_string()),
            ("data_reward_base", metadata.data_reward_base.to_string()),
            ("last_block_time", metadata.last_block_time.to_string()),
        ];

        for (key, value) in pairs {
            sqlx::query(
                "INSERT INTO chain_metadata (meta_key, meta_value) VALUES (?, ?) \
                 ON DUPLICATE KEY UPDATE meta_value = VALUES(meta_value)",
            )
            .bind(key)
            .bind(&value)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to write metadata {}: {}", key, e))?;
        }

        Ok(())
    }

    /// Save supply info
    pub async fn put_supply_info(
        &self,
        total_supply: u64,
        total_staked: u64,
    ) -> Result<(), String> {
        let pairs = vec![
            ("total_supply", total_supply.to_string()),
            ("total_staked", total_staked.to_string()),
        ];

        for (key, value) in pairs {
            sqlx::query(
                "INSERT INTO chain_metadata (meta_key, meta_value) VALUES (?, ?) \
                 ON DUPLICATE KEY UPDATE meta_value = VALUES(meta_value)",
            )
            .bind(key)
            .bind(&value)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to write supply info {}: {}", key, e))?;
        }

        Ok(())
    }

    /// Get total supply
    pub async fn get_total_supply(&self) -> u64 {
        sqlx::query_scalar::<_, String>(
            "SELECT meta_value FROM chain_metadata WHERE meta_key = 'total_supply'",
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
    }

    /// Get total staked
    pub async fn get_total_staked(&self) -> u64 {
        sqlx::query_scalar::<_, String>(
            "SELECT meta_value FROM chain_metadata WHERE meta_key = 'total_staked'",
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
    }

    // ==================== Accounts ====================

    /// Load all accounts from database
    pub async fn get_all_accounts(&self) -> HashMap<String, Account> {
        let mut accounts = HashMap::new();

        match sqlx::query(
            "SELECT address, balance, nonce, data_contributions, reputation_score, staked_amount FROM accounts"
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => {
                for row in rows {
                    let address: String = row.get("address");
                    let account = Account {
                        address: address.clone(),
                        balance: row.get::<u64, _>("balance"),
                        nonce: row.get::<u64, _>("nonce"),
                        data_contributions: row.get::<u64, _>("data_contributions"),
                        reputation_score: row.get("reputation_score"),
                        staked_amount: row.get::<u64, _>("staked_amount"),
                    };
                    accounts.insert(address, account);
                }
                info!("Loaded {} accounts from OceanBase", accounts.len());
            }
            Err(e) => {
                error!("Failed to load accounts from OceanBase: {}", e);
            }
        }

        accounts
    }

    /// Save a single account
    pub async fn put_account(&self, account: &Account) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO accounts (address, balance, nonce, data_contributions, reputation_score, staked_amount) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             balance = VALUES(balance), nonce = VALUES(nonce), \
             data_contributions = VALUES(data_contributions), \
             reputation_score = VALUES(reputation_score), \
             staked_amount = VALUES(staked_amount)"
        )
        .bind(&account.address)
        .bind(account.balance)
        .bind(account.nonce)
        .bind(account.data_contributions)
        .bind(account.reputation_score)
        .bind(account.staked_amount)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to write account {}: {}", account.address, e))?;

        Ok(())
    }

    /// Batch save accounts
    pub async fn put_accounts_batch(
        &self,
        accounts: &HashMap<String, Account>,
    ) -> Result<(), String> {
        // Process in chunks of 100 to avoid query size limits
        let accounts_vec: Vec<&Account> = accounts.values().collect();
        for chunk in accounts_vec.chunks(100) {
            for account in chunk {
                self.put_account(account).await?;
            }
        }
        info!("Saved {} accounts to OceanBase", accounts.len());
        Ok(())
    }

    // ==================== Blocks ====================

    /// Save a block (header only, transactions stored separately)
    pub async fn put_block(&self, block: &Block) -> Result<(), String> {
        // Insert block header
        sqlx::query(
            "INSERT INTO blocks (block_index, hash, previous_hash, merkle_root, timestamp, \
             difficulty, nonce, data_entropy, validator, tx_count) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             hash = VALUES(hash), tx_count = VALUES(tx_count)",
        )
        .bind(block.index)
        .bind(&block.hash)
        .bind(&block.header.previous_hash)
        .bind(&block.header.merkle_root)
        .bind(block.header.timestamp.timestamp())
        .bind(block.header.difficulty)
        .bind(block.header.nonce)
        .bind(block.header.data_entropy)
        .bind(&block.validator)
        .bind(block.transactions.len() as u32)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to write block {}: {}", block.index, e))?;

        // Insert transactions
        for tx in &block.transactions {
            self.put_transaction(tx, block.index).await?;
        }

        Ok(())
    }

    /// Get recent blocks (for in-memory cache)
    pub async fn get_recent_blocks(&self, count: usize) -> Vec<Block> {
        let mut blocks = Vec::new();

        match sqlx::query(
            "SELECT block_index, hash, previous_hash, merkle_root, timestamp, \
             difficulty, nonce, data_entropy, validator, tx_count \
             FROM blocks ORDER BY block_index DESC LIMIT ?",
        )
        .bind(count as u64)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => {
                for row in rows.iter().rev() {
                    let block_index: u64 = row.get("block_index");
                    let hash: String = row.get("hash");
                    let previous_hash: String = row.get("previous_hash");
                    let merkle_root: String = row.get("merkle_root");
                    let timestamp_secs: i64 = row.get("timestamp");
                    let difficulty: u64 = row.get("difficulty");
                    let nonce: u64 = row.get("nonce");
                    let data_entropy: f64 = row.get("data_entropy");
                    let validator: String = row.get("validator");

                    // Load transactions for this block
                    let transactions = self.get_transactions_for_block(block_index).await;

                    let header = BlockHeader {
                        version: 1,
                        previous_hash,
                        merkle_root,
                        timestamp: chrono::DateTime::from_timestamp(timestamp_secs, 0)
                            .unwrap_or_else(|| chrono::Utc::now()),
                        difficulty,
                        nonce,
                        data_entropy,
                    };

                    blocks.push(Block {
                        index: block_index,
                        header,
                        transactions,
                        hash,
                        validator,
                    });
                }
                info!("Loaded {} recent blocks from OceanBase", blocks.len());
            }
            Err(e) => {
                error!("Failed to load recent blocks from OceanBase: {}", e);
            }
        }

        blocks
    }

    /// Get a single block by index
    pub async fn get_block(&self, index: u64) -> Option<Block> {
        let row = sqlx::query(
            "SELECT block_index, hash, previous_hash, merkle_root, timestamp, \
             difficulty, nonce, data_entropy, validator \
             FROM blocks WHERE block_index = ?",
        )
        .bind(index)
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        let hash: String = row.get("hash");
        let previous_hash: String = row.get("previous_hash");
        let merkle_root: String = row.get("merkle_root");
        let timestamp_secs: i64 = row.get("timestamp");
        let difficulty: u64 = row.get("difficulty");
        let nonce: u64 = row.get("nonce");
        let data_entropy: f64 = row.get("data_entropy");
        let validator: String = row.get("validator");

        let transactions = self.get_transactions_for_block(index).await;

        let header = BlockHeader {
            version: 1,
            previous_hash,
            merkle_root,
            timestamp: chrono::DateTime::from_timestamp(timestamp_secs, 0)
                .unwrap_or_else(|| chrono::Utc::now()),
            difficulty,
            nonce,
            data_entropy,
        };

        Some(Block {
            index,
            header,
            transactions,
            hash,
            validator,
        })
    }

    // ==================== Transactions ====================

    /// Save a transaction
    pub async fn put_transaction(&self, tx: &Transaction, block_index: u64) -> Result<(), String> {
        let tx_type_str = match tx.tx_type {
            TransactionType::Transfer => "Transfer",
            TransactionType::DataContribution => "DataContribution",
            TransactionType::DataPurchase => "DataPurchase",
            TransactionType::ContractDeploy => "ContractDeploy",
            TransactionType::ContractCall => "ContractCall",
            TransactionType::Stake => "Stake",
            TransactionType::Unstake => "Unstake",
            TransactionType::Reward => "Reward",
            TransactionType::Genesis => "Genesis",
        };

        let (q_entropy, q_uniqueness, q_freshness, q_completeness, q_overall) =
            if let Some(ref dq) = tx.data_quality {
                (
                    Some(dq.entropy_score),
                    Some(dq.uniqueness_score),
                    Some(dq.freshness_score),
                    Some(dq.completeness_score),
                    Some(dq.overall_score),
                )
            } else {
                (None, None, None, None, None)
            };

        sqlx::query(
            "INSERT INTO transactions (id, tx_type, timestamp, sender, \
             data, gas_price, gas_limit, hash, signature, block_index, \
             quality_entropy, quality_uniqueness, quality_freshness, quality_completeness, quality_overall) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE block_index = VALUES(block_index)"
        )
        .bind(&tx.id)
        .bind(tx_type_str)
        .bind(tx.timestamp.timestamp())
        .bind(&tx.sender)
        .bind(&tx.data)
        .bind(tx.gas_price)
        .bind(tx.gas_limit)
        .bind(&tx.hash)
        .bind(&tx.signature)
        .bind(block_index)
        .bind(q_entropy)
        .bind(q_uniqueness)
        .bind(q_freshness)
        .bind(q_completeness)
        .bind(q_overall)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to write transaction {}: {}", &tx.id, e))?;

        // Insert outputs
        for output in &tx.outputs {
            sqlx::query(
                "INSERT INTO tx_outputs (tx_id, recipient, amount) VALUES (?, ?, ?) \
                 ON DUPLICATE KEY UPDATE amount = VALUES(amount)",
            )
            .bind(&tx.id)
            .bind(&output.recipient)
            .bind(output.amount)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to write tx output: {}", e))?;
        }

        Ok(())
    }

    /// Get transactions for a block
    pub async fn get_transactions_for_block(&self, block_index: u64) -> Vec<Transaction> {
        let mut transactions = Vec::new();

        let rows = match sqlx::query(
            "SELECT id, tx_type, timestamp, sender, data, \
             gas_price, gas_limit, hash, signature, \
             quality_entropy, quality_uniqueness, quality_freshness, quality_completeness, quality_overall \
             FROM transactions WHERE block_index = ? ORDER BY id"
        )
        .bind(block_index)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                error!("Failed to load transactions for block {}: {}", block_index, e);
                return transactions;
            }
        };

        for row in rows {
            let id: String = row.get("id");
            let tx_type_str: String = row.get("tx_type");
            let timestamp_secs: i64 = row.get("timestamp");
            let sender: String = row.get("sender");
            let data: Option<String> = row.get("data");
            let gas_price: u64 = row.get("gas_price");
            let gas_limit: u64 = row.get("gas_limit");
            let hash: String = row.get("hash");
            let signature: Option<String> = row.get("signature");
            let sender_public_key: Option<String> = None; // Not stored in OceanBase

            let tx_type = match tx_type_str.as_str() {
                "Transfer" => TransactionType::Transfer,
                "DataContribution" => TransactionType::DataContribution,
                "DataPurchase" => TransactionType::DataPurchase,
                "ContractDeploy" => TransactionType::ContractDeploy,
                "ContractCall" => TransactionType::ContractCall,
                "Stake" => TransactionType::Stake,
                "Unstake" => TransactionType::Unstake,
                "Reward" => TransactionType::Reward,
                "Genesis" => TransactionType::Genesis,
                _ => TransactionType::Transfer,
            };

            let data_quality = {
                let q_entropy: Option<f64> = row.get("quality_entropy");
                let q_uniqueness: Option<f64> = row.get("quality_uniqueness");
                let q_freshness: Option<f64> = row.get("quality_freshness");
                let q_completeness: Option<f64> = row.get("quality_completeness");
                let q_overall: Option<f64> = row.get("quality_overall");

                if let (Some(e), Some(u), Some(f), Some(c), Some(o)) = (
                    q_entropy,
                    q_uniqueness,
                    q_freshness,
                    q_completeness,
                    q_overall,
                ) {
                    Some(DataQuality {
                        entropy_score: e,
                        uniqueness_score: u,
                        freshness_score: f,
                        completeness_score: c,
                        overall_score: o,
                    })
                } else {
                    None
                }
            };

            // Load outputs
            let outputs = self.get_tx_outputs(&id).await;

            let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
                .unwrap_or_else(|| chrono::Utc::now());

            transactions.push(Transaction {
                id,
                tx_type,
                timestamp,
                sender,
                sender_public_key,
                inputs: Vec::new(), // Inputs are rarely used in this chain
                outputs,
                data,
                data_quality,
                gas_price,
                gas_limit,
                hash,
                signature,
            });
        }

        transactions
    }

    /// Get transaction outputs
    async fn get_tx_outputs(&self, tx_id: &str) -> Vec<TxOutput> {
        match sqlx::query("SELECT recipient, amount FROM tx_outputs WHERE tx_id = ?")
            .bind(tx_id)
            .fetch_all(&self.pool)
            .await
        {
            Ok(rows) => {
                rows.iter()
                    .map(|row| {
                        TxOutput {
                            amount: row.get::<u64, _>("amount"),
                            recipient: row.get("recipient"),
                            data_hash: None, // Not stored separately for now
                        }
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Get a transaction by hash
    pub async fn get_transaction(&self, hash: &str) -> Option<Transaction> {
        let row = sqlx::query(
            "SELECT id, tx_type, timestamp, sender, data, \
             gas_price, gas_limit, hash, signature, block_index, \
             quality_entropy, quality_uniqueness, quality_freshness, quality_completeness, quality_overall \
             FROM transactions WHERE hash = ?"
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        let id: String = row.get("id");
        let tx_type_str: String = row.get("tx_type");
        let timestamp_secs: i64 = row.get("timestamp");
        let sender: String = row.get("sender");
        let data: Option<String> = row.get("data");
        let gas_price: u64 = row.get("gas_price");
        let gas_limit: u64 = row.get("gas_limit");
        let tx_hash: String = row.get("hash");
        let signature: Option<String> = row.get("signature");
        let sender_public_key: Option<String> = None; // Not stored in OceanBase

        let tx_type = match tx_type_str.as_str() {
            "Transfer" => TransactionType::Transfer,
            "DataContribution" => TransactionType::DataContribution,
            "DataPurchase" => TransactionType::DataPurchase,
            "ContractDeploy" => TransactionType::ContractDeploy,
            "ContractCall" => TransactionType::ContractCall,
            "Stake" => TransactionType::Stake,
            "Unstake" => TransactionType::Unstake,
            "Reward" => TransactionType::Reward,
            "Genesis" => TransactionType::Genesis,
            _ => TransactionType::Transfer,
        };

        let data_quality = {
            let q_entropy: Option<f64> = row.get("quality_entropy");
            let q_uniqueness: Option<f64> = row.get("quality_uniqueness");
            let q_freshness: Option<f64> = row.get("quality_freshness");
            let q_completeness: Option<f64> = row.get("quality_completeness");
            let q_overall: Option<f64> = row.get("quality_overall");

            if let (Some(e), Some(u), Some(f), Some(c), Some(o)) = (
                q_entropy,
                q_uniqueness,
                q_freshness,
                q_completeness,
                q_overall,
            ) {
                Some(DataQuality {
                    entropy_score: e,
                    uniqueness_score: u,
                    freshness_score: f,
                    completeness_score: c,
                    overall_score: o,
                })
            } else {
                None
            }
        };

        let outputs = self.get_tx_outputs(&id).await;
        let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
            .unwrap_or_else(|| chrono::Utc::now());

        Some(Transaction {
            id,
            tx_type,
            timestamp,
            sender,
            sender_public_key,
            inputs: Vec::new(),
            outputs,
            data,
            data_quality,
            gas_price,
            gas_limit,
            hash: tx_hash,
            signature,
        })
    }

    // ==================== Data Registry ====================

    /// Save a data entry
    pub async fn put_data_entry(&self, entry: &DataEntry) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO data_registry (data_hash, owner, price, quality_score, timestamp, purchases, category) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             purchases = VALUES(purchases), quality_score = VALUES(quality_score)"
        )
        .bind(&entry.hash)
        .bind(&entry.owner)
        .bind(entry.price)
        .bind(entry.quality_score)
        .bind(entry.timestamp)
        .bind(entry.purchases)
        .bind(&entry.category)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to write data entry: {}", e))?;

        Ok(())
    }

    /// Get data registry count (without loading all entries into memory)
    pub async fn get_data_registry_count(&self) -> u64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM data_registry")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) as u64
    }

    // ==================== IoT Contributions ====================

    /// Save an IoT contribution
    pub async fn put_iot_contribution(
        &self,
        device_id: &str,
        data_hash: &str,
        data_type: &str,
        data_size: u64,
        quality_score: f64,
        reward_amount: f64,
        timestamp: u64,
        tx_hash: Option<&str>,
        category: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        source: &str,
    ) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO iot_contributions (device_id, data_hash, data_type, data_size_bytes, \
             quality_score, reward_amount, timestamp, tx_hash, category, latitude, longitude, source) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(device_id)
        .bind(data_hash)
        .bind(data_type)
        .bind(data_size)
        .bind(quality_score)
        .bind(reward_amount)
        .bind(timestamp)
        .bind(tx_hash)
        .bind(category)
        .bind(latitude)
        .bind(longitude)
        .bind(source)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to write IoT contribution: {}", e))?;

        Ok(())
    }

    // ==================== Health Check ====================

    /// Check if database connection is healthy
    pub async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }

    /// Get pool reference for direct queries
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}
