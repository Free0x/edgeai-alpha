//! Security Middleware Module
//!
//! Provides security features including:
//! - Request validation and sanitization
//! - IP-based blocking
//! - Request size limits
//! - Security headers

use actix_web::{HttpRequest, HttpResponse};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Security configuration
#[derive(Clone)]
pub struct SecurityConfig {
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Maximum URL length
    pub max_url_length: usize,
    /// Maximum header size
    pub max_header_size: usize,
    /// Block duration for suspicious IPs (seconds)
    pub block_duration: u64,
    /// Maximum failed requests before blocking
    pub max_failed_requests: u32,
    /// Time window for failed request counting (seconds)
    pub failed_request_window: u64,
    /// Enable strict transport security
    pub enable_hsts: bool,
    /// Enable content security policy
    pub enable_csp: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10 MB
            max_url_length: 2048,
            max_header_size: 8192,
            block_duration: 3600, // 1 hour
            max_failed_requests: 100,
            failed_request_window: 60,
            enable_hsts: true,
            enable_csp: true,
        }
    }
}

/// IP block entry
struct BlockEntry {
    blocked_at: Instant,
    reason: String,
    expires_at: Instant,
}

/// Failed request tracker
struct FailedRequestTracker {
    count: u32,
    window_start: Instant,
}

/// Security manager
pub struct SecurityManager {
    config: SecurityConfig,
    /// Blocked IPs
    blocked_ips: RwLock<std::collections::HashMap<String, BlockEntry>>,
    /// Failed request counters per IP
    failed_requests: RwLock<std::collections::HashMap<String, FailedRequestTracker>>,
    /// Permanently blocked IPs (from config)
    permanent_blocklist: HashSet<String>,
    /// Allowed IPs (bypass all checks)
    allowlist: HashSet<String>,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            blocked_ips: RwLock::new(std::collections::HashMap::new()),
            failed_requests: RwLock::new(std::collections::HashMap::new()),
            permanent_blocklist: HashSet::new(),
            allowlist: HashSet::new(),
        }
    }

    /// Add IP to permanent blocklist
    pub fn add_to_blocklist(&mut self, ip: String) {
        self.permanent_blocklist.insert(ip);
    }

    /// Add IP to allowlist
    pub fn add_to_allowlist(&mut self, ip: String) {
        self.allowlist.insert(ip);
    }

    /// Check if IP is blocked
    pub async fn is_blocked(&self, ip: &str) -> Option<String> {
        // Check permanent blocklist
        if self.permanent_blocklist.contains(ip) {
            return Some("IP is permanently blocked".to_string());
        }

        // Check allowlist
        if self.allowlist.contains(ip) {
            return None;
        }

        // Check temporary blocks
        let blocked = self.blocked_ips.read().await;
        if let Some(entry) = blocked.get(ip) {
            if Instant::now() < entry.expires_at {
                return Some(entry.reason.clone());
            }
        }

        None
    }

    /// Block an IP temporarily
    pub async fn block_ip(&self, ip: String, reason: String) {
        let now = Instant::now();
        let expires_at = now + Duration::from_secs(self.config.block_duration);

        let mut blocked = self.blocked_ips.write().await;
        blocked.insert(
            ip.clone(),
            BlockEntry {
                blocked_at: now,
                reason: reason.clone(),
                expires_at,
            },
        );

        warn!("Blocked IP {} for: {}", ip, reason);
    }

    /// Record a failed request
    pub async fn record_failed_request(&self, ip: &str) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.failed_request_window);

        let mut trackers = self.failed_requests.write().await;
        let tracker = trackers
            .entry(ip.to_string())
            .or_insert(FailedRequestTracker {
                count: 0,
                window_start: now,
            });

        // Reset if window expired
        if now.duration_since(tracker.window_start) > window {
            tracker.count = 0;
            tracker.window_start = now;
        }

        tracker.count += 1;
        let count = tracker.count;
        let should_block = count >= self.config.max_failed_requests;

        // Release lock before potentially blocking
        drop(trackers);

        // Check if should block
        if should_block {
            self.block_ip(
                ip.to_string(),
                format!("Too many failed requests: {}", count),
            )
            .await;
            return true;
        }

        false
    }

    /// Validate request
    pub fn validate_request(&self, req: &HttpRequest) -> Result<(), SecurityError> {
        // Check URL length
        let url_len = req.uri().to_string().len();
        if url_len > self.config.max_url_length {
            return Err(SecurityError::UrlTooLong(
                url_len,
                self.config.max_url_length,
            ));
        }

        // Check for suspicious patterns in URL
        let url = req.uri().to_string();
        if self.contains_suspicious_pattern(&url) {
            return Err(SecurityError::SuspiciousPattern);
        }

        // Check headers
        let header_size: usize = req
            .headers()
            .iter()
            .map(|(k, v)| k.as_str().len() + v.len())
            .sum();

        if header_size > self.config.max_header_size {
            return Err(SecurityError::HeadersTooLarge(
                header_size,
                self.config.max_header_size,
            ));
        }

        Ok(())
    }

    /// Check for suspicious patterns (SQL injection, XSS, etc.)
    fn contains_suspicious_pattern(&self, input: &str) -> bool {
        let suspicious_patterns = [
            // SQL injection
            "' OR ",
            "' AND ",
            "1=1",
            "DROP TABLE",
            "DELETE FROM",
            "INSERT INTO",
            "UNION SELECT",
            "--",
            "/*",
            "*/",
            // XSS
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
            // Path traversal
            "../",
            "..\\",
            "%2e%2e",
            // Command injection
            "; ls",
            "; cat",
            "| cat",
            "$(",
            "`",
        ];

        let lower = input.to_lowercase();
        suspicious_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Get security headers
    pub fn get_security_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            ("X-Content-Type-Options", "nosniff".to_string()),
            ("X-Frame-Options", "DENY".to_string()),
            ("X-XSS-Protection", "1; mode=block".to_string()),
            (
                "Referrer-Policy",
                "strict-origin-when-cross-origin".to_string(),
            ),
        ];

        if self.config.enable_hsts {
            headers.push((
                "Strict-Transport-Security",
                "max-age=31536000; includeSubDomains".to_string(),
            ));
        }

        if self.config.enable_csp {
            headers.push((
                "Content-Security-Policy",
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self' https:".to_string(),
            ));
        }

        headers
    }

    /// Clean up expired blocks
    pub async fn cleanup_expired(&self) {
        let now = Instant::now();

        // Clean blocked IPs
        let mut blocked = self.blocked_ips.write().await;
        let before = blocked.len();
        blocked.retain(|_, entry| now < entry.expires_at);
        let removed = before - blocked.len();

        if removed > 0 {
            info!("Cleaned up {} expired IP blocks", removed);
        }

        // Clean failed request trackers
        let window = Duration::from_secs(self.config.failed_request_window * 2);
        let mut trackers = self.failed_requests.write().await;
        trackers.retain(|_, tracker| now.duration_since(tracker.window_start) < window);
    }

    /// Get security statistics
    pub async fn stats(&self) -> SecurityStats {
        let blocked = self.blocked_ips.read().await;
        let trackers = self.failed_requests.read().await;

        SecurityStats {
            blocked_ips: blocked.len(),
            permanent_blocklist_size: self.permanent_blocklist.len(),
            allowlist_size: self.allowlist.len(),
            tracked_ips: trackers.len(),
        }
    }
}

/// Security error types
#[derive(Debug)]
pub enum SecurityError {
    UrlTooLong(usize, usize),
    HeadersTooLarge(usize, usize),
    BodyTooLarge(usize, usize),
    SuspiciousPattern,
    BlockedIP(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::UrlTooLong(actual, max) => {
                write!(f, "URL too long: {} bytes (max: {})", actual, max)
            }
            SecurityError::HeadersTooLarge(actual, max) => {
                write!(f, "Headers too large: {} bytes (max: {})", actual, max)
            }
            SecurityError::BodyTooLarge(actual, max) => {
                write!(f, "Body too large: {} bytes (max: {})", actual, max)
            }
            SecurityError::SuspiciousPattern => {
                write!(f, "Suspicious pattern detected in request")
            }
            SecurityError::BlockedIP(reason) => {
                write!(f, "IP blocked: {}", reason)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Security statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    pub blocked_ips: usize,
    pub permanent_blocklist_size: usize,
    pub allowlist_size: usize,
    pub tracked_ips: usize,
}

/// Global security manager
pub type GlobalSecurityManager = Arc<SecurityManager>;

pub fn create_security_manager(config: SecurityConfig) -> GlobalSecurityManager {
    Arc::new(SecurityManager::new(config))
}

/// Input sanitization utilities
pub mod sanitize {
    /// Sanitize string input (remove dangerous characters)
    pub fn sanitize_string(input: &str) -> String {
        input
            .chars()
            .filter(|c| !matches!(c, '<' | '>' | '"' | '\'' | '&' | '\\' | '\0'))
            .collect()
    }

    /// Sanitize for SQL (escape quotes)
    pub fn sanitize_sql(input: &str) -> String {
        input.replace('\'', "''").replace('\\', "\\\\")
    }

    /// Sanitize for HTML display
    pub fn sanitize_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Validate and sanitize address format
    pub fn validate_address(address: &str) -> Result<String, &'static str> {
        let sanitized = sanitize_string(address);

        // Check length
        if sanitized.len() < 8 || sanitized.len() > 128 {
            return Err("Invalid address length");
        }

        // Check for valid characters (alphanumeric and underscore)
        if !sanitized.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Invalid characters in address");
        }

        Ok(sanitized)
    }

    /// Validate transaction hash format
    pub fn validate_tx_hash(hash: &str) -> Result<String, &'static str> {
        let sanitized = sanitize_string(hash);

        // Check for hex format
        if !sanitized.chars().all(|c| c.is_ascii_hexdigit() || c == 'x') {
            return Err("Invalid hash format");
        }

        // Check length (with or without 0x prefix)
        let len = if sanitized.starts_with("0x") {
            sanitized.len() - 2
        } else {
            sanitized.len()
        };

        if len != 64 {
            return Err("Invalid hash length");
        }

        Ok(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ip_blocking() {
        let manager = SecurityManager::new(SecurityConfig::default());

        // Initially not blocked
        assert!(manager.is_blocked("192.168.1.1").await.is_none());

        // Block IP
        manager
            .block_ip("192.168.1.1".to_string(), "Test block".to_string())
            .await;

        // Now blocked
        assert!(manager.is_blocked("192.168.1.1").await.is_some());
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize::sanitize_string("hello<script>"), "helloscript");
        assert_eq!(sanitize::sanitize_string("normal text"), "normal text");
    }

    #[test]
    fn test_validate_address() {
        assert!(sanitize::validate_address("valid_address_123").is_ok());
        assert!(sanitize::validate_address("short").is_err());
        assert!(sanitize::validate_address("invalid<>address").is_err());
    }
}
