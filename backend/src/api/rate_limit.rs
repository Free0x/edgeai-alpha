//! Rate Limiting Module
//!
//! Provides request rate limiting to prevent abuse and ensure fair usage.
//! Uses token bucket algorithm for smooth rate limiting.

use actix_web::{dev::ServiceRequest, HttpRequest, HttpResponse};
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token bucket for rate limiting
#[derive(Clone)]
pub struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Maximum tokens (burst capacity)
    max_tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume a token, returns true if successful
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Get remaining tokens
    pub fn remaining(&mut self) -> f64 {
        self.refill();
        self.tokens
    }

    /// Get time until next token is available
    pub fn time_until_available(&mut self) -> Duration {
        self.refill();
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let needed = 1.0 - self.tokens;
            Duration::from_secs_f64(needed / self.refill_rate)
        }
    }
}

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window (burst)
    pub max_requests: u32,
    /// Requests per second (sustained rate)
    pub requests_per_second: f64,
    /// Window for tracking (for logging)
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,         // 100 request burst
            requests_per_second: 10.0, // 10 requests/second sustained
            window_secs: 60,
        }
    }
}

/// Different rate limit tiers
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitTier {
    /// Anonymous/unauthenticated requests
    Anonymous,
    /// Authenticated users
    Authenticated,
    /// Premium/paid tier
    Premium,
    /// Internal services
    Internal,
}

impl RateLimitTier {
    pub fn config(&self) -> RateLimitConfig {
        match self {
            RateLimitTier::Anonymous => RateLimitConfig {
                max_requests: 60,
                requests_per_second: 5.0,
                window_secs: 60,
            },
            RateLimitTier::Authenticated => RateLimitConfig {
                max_requests: 300,
                requests_per_second: 20.0,
                window_secs: 60,
            },
            RateLimitTier::Premium => RateLimitConfig {
                max_requests: 1000,
                requests_per_second: 100.0,
                window_secs: 60,
            },
            RateLimitTier::Internal => RateLimitConfig {
                max_requests: 10000,
                requests_per_second: 1000.0,
                window_secs: 60,
            },
        }
    }
}

/// Rate limiter state
pub struct RateLimiter {
    buckets: RwLock<HashMap<String, TokenBucket>>,
    cleanup_interval: Duration,
    last_cleanup: RwLock<Instant>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Check if request is allowed for given key and tier
    pub async fn check(&self, key: &str, tier: RateLimitTier) -> RateLimitResult {
        let config = tier.config();

        // Periodic cleanup of old buckets
        self.maybe_cleanup().await;

        let mut buckets = self.buckets.write().await;
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(config.max_requests as f64, config.requests_per_second)
        });

        if bucket.try_consume() {
            RateLimitResult::Allowed {
                remaining: bucket.remaining() as u32,
                limit: config.max_requests,
            }
        } else {
            let retry_after = bucket.time_until_available();
            warn!("Rate limit exceeded for key: {}", key);
            RateLimitResult::Exceeded {
                retry_after_secs: retry_after.as_secs() + 1,
                limit: config.max_requests,
            }
        }
    }

    /// Cleanup old buckets that haven't been used
    async fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read().await;
            last.elapsed() > self.cleanup_interval
        };

        if should_cleanup {
            let mut buckets = self.buckets.write().await;
            let before = buckets.len();

            // Remove buckets that are full (haven't been used recently)
            buckets.retain(|_, bucket| bucket.remaining() < bucket.max_tokens * 0.9);

            let removed = before - buckets.len();
            if removed > 0 {
                log::info!("Rate limiter cleanup: removed {} inactive buckets", removed);
            }

            *self.last_cleanup.write().await = Instant::now();
        }
    }

    /// Get client identifier from request
    pub fn get_client_key(req: &HttpRequest) -> String {
        // Try to get real IP from X-Forwarded-For header (for proxied requests)
        if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
            if let Ok(ip_str) = forwarded.to_str() {
                if let Some(first_ip) = ip_str.split(',').next() {
                    return first_ip.trim().to_string();
                }
            }
        }

        // Fall back to peer address
        req.peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Result of rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed { remaining: u32, limit: u32 },
    Exceeded { retry_after_secs: u64, limit: u32 },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }

    /// Add rate limit headers to response
    pub fn add_headers(&self, mut response: HttpResponse) -> HttpResponse {
        match self {
            RateLimitResult::Allowed { remaining, limit } => {
                response.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                    actix_web::http::header::HeaderValue::from_str(&limit.to_string()).unwrap(),
                );
                response.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                    actix_web::http::header::HeaderValue::from_str(&remaining.to_string()).unwrap(),
                );
            }
            RateLimitResult::Exceeded {
                retry_after_secs,
                limit,
            } => {
                response.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                    actix_web::http::header::HeaderValue::from_str(&limit.to_string()).unwrap(),
                );
                response.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                    actix_web::http::header::HeaderValue::from_static("0"),
                );
                response.headers_mut().insert(
                    actix_web::http::header::HeaderName::from_static("retry-after"),
                    actix_web::http::header::HeaderValue::from_str(&retry_after_secs.to_string())
                        .unwrap(),
                );
            }
        }
        response
    }
}

/// Global rate limiter instance
pub type GlobalRateLimiter = Arc<RateLimiter>;

pub fn create_rate_limiter() -> GlobalRateLimiter {
    Arc::new(RateLimiter::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        // Should be able to consume 10 tokens
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }

        // 11th should fail
        assert!(!bucket.try_consume());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();

        // First request should be allowed
        let result = limiter.check("test_client", RateLimitTier::Anonymous).await;
        assert!(result.is_allowed());
    }
}
