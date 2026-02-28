//! Authentication Middleware
//!
//! Provides signature-based authentication for write operations.
//! Uses ed25519 signatures to verify that requests are authorized by the wallet owner.

use crate::crypto::{address_from_public_key, verify_signature};
use actix_web::{http::StatusCode, HttpResponse};
use serde::{Deserialize, Serialize};

/// Signed request wrapper
/// All write operations should include this authentication data
#[derive(Debug, Deserialize)]
pub struct SignedRequest<T> {
    /// The actual request payload
    pub data: T,
    /// Authentication information
    pub auth: AuthData,
}

/// Authentication data for signed requests
#[derive(Debug, Deserialize)]
pub struct AuthData {
    /// Public key of the signer (hex encoded)
    pub public_key: String,
    /// Signature of the serialized data (hex encoded)
    pub signature: String,
    /// Unix timestamp of the request (for replay protection)
    pub timestamp: u64,
    /// Optional nonce for additional replay protection
    #[serde(default)]
    pub nonce: Option<String>,
}

/// Authentication error response
#[derive(Debug, Serialize)]
pub struct AuthError {
    pub success: bool,
    pub error: String,
    pub error_code: String,
}

impl AuthError {
    pub fn new(error: &str, code: &str) -> Self {
        Self {
            success: false,
            error: error.to_string(),
            error_code: code.to_string(),
        }
    }

    pub fn to_response(&self, status: StatusCode) -> HttpResponse {
        HttpResponse::build(status).json(self)
    }
}

/// Verify a signed request
///
/// # Arguments
/// * `auth` - Authentication data containing public key and signature
/// * `message` - The message that was signed (typically serialized request data)
/// * `expected_address` - Optional expected address (if the request includes an address field)
/// * `max_age_secs` - Maximum age of the request in seconds (for replay protection)
///
/// # Returns
/// * `Ok(address)` - The verified wallet address
/// * `Err(HttpResponse)` - An error response if verification fails
pub fn verify_signed_request(
    auth: &AuthData,
    message: &[u8],
    expected_address: Option<&str>,
    max_age_secs: u64,
) -> Result<String, HttpResponse> {
    // Check timestamp (replay protection)
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if auth.timestamp > current_time + 60 {
        // Allow 60 seconds clock skew into the future
        return Err(
            AuthError::new("Request timestamp is in the future", "TIMESTAMP_FUTURE")
                .to_response(StatusCode::BAD_REQUEST),
        );
    }

    if current_time - auth.timestamp > max_age_secs {
        return Err(AuthError::new(
            &format!("Request expired. Max age is {} seconds", max_age_secs),
            "REQUEST_EXPIRED",
        )
        .to_response(StatusCode::BAD_REQUEST));
    }

    // Verify signature
    match verify_signature(&auth.public_key, message, &auth.signature) {
        Ok(true) => {
            // Derive address from public key
            match address_from_public_key(&auth.public_key) {
                Ok(address) => {
                    // If expected address is provided, verify it matches
                    if let Some(expected) = expected_address {
                        if address != expected {
                            return Err(AuthError::new(
                                "Signer address does not match the request address",
                                "ADDRESS_MISMATCH",
                            )
                            .to_response(StatusCode::FORBIDDEN));
                        }
                    }
                    Ok(address)
                }
                Err(_) => Err(
                    AuthError::new("Invalid public key format", "INVALID_PUBLIC_KEY")
                        .to_response(StatusCode::BAD_REQUEST),
                ),
            }
        }
        Ok(false) => Err(AuthError::new("Invalid signature", "INVALID_SIGNATURE")
            .to_response(StatusCode::UNAUTHORIZED)),
        Err(_) => Err(
            AuthError::new("Signature verification failed", "SIGNATURE_ERROR")
                .to_response(StatusCode::BAD_REQUEST),
        ),
    }
}

/// Helper to create the message to sign for a request
/// The message format is: "{method}:{path}:{timestamp}:{body_hash}"
pub fn create_sign_message(method: &str, path: &str, timestamp: u64, body: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(body);
    let body_hash = hex::encode(hasher.finalize());

    format!("{}:{}:{}:{}", method, path, timestamp, body_hash).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Wallet;

    #[test]
    fn test_signed_request_verification() {
        let wallet = Wallet::new();
        let message = b"test message";
        let signature = wallet.sign(message);

        let auth = AuthData {
            public_key: wallet.public_key_hex(),
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            nonce: None,
        };

        let result = verify_signed_request(&auth, message, None, 300);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wallet.address());
    }
}
