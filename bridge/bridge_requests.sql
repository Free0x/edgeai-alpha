-- Bridge requests table for OceanBase
CREATE TABLE IF NOT EXISTS bridge_requests (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    request_id VARCHAR(64) NOT NULL UNIQUE,
    direction VARCHAR(20) NOT NULL COMMENT 'edge_to_bsc or bsc_to_edge',
    target_chain VARCHAR(20) NOT NULL DEFAULT 'bsc',
    edge_address VARCHAR(128) NOT NULL,
    evm_address VARCHAR(42) NOT NULL,
    amount BIGINT UNSIGNED NOT NULL,
    fee BIGINT UNSIGNED NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' COMMENT 'pending/processing/completed/failed/cancelled',
    tx_hash_edge VARCHAR(128) DEFAULT NULL,
    tx_hash_evm VARCHAR(66) DEFAULT NULL,
    admin_note TEXT DEFAULT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    completed_at TIMESTAMP NULL DEFAULT NULL,
    INDEX idx_status (status),
    INDEX idx_edge_address (edge_address),
    INDEX idx_evm_address (evm_address),
    INDEX idx_created_at (created_at)
) DEFAULT CHARSET=utf8mb4;
