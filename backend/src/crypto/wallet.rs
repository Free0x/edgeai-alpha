//! EdgeAI Blockchain - Wallet and Cryptographic Key Management
//!
//! Implements Ed25519 key pairs for secure transaction signing.
//! Provides wallet generation, key management, and signature verification.

#![allow(dead_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hex;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Represents a wallet with a key pair for signing transactions
#[derive(Clone)]
pub struct Wallet {
    /// The Ed25519 signing key (contains both secret and public)
    signing_key: SigningKey,
    /// Human-readable address derived from public key
    address: String,
}

impl Wallet {
    /// Generate a new wallet with a random key pair
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let address = Self::derive_address(&signing_key.verifying_key());

        Wallet {
            signing_key,
            address,
        }
    }

    /// Create a wallet from an existing secret key (hex encoded)
    pub fn from_secret_key(secret_hex: &str) -> Result<Self, WalletError> {
        let secret_bytes = hex::decode(secret_hex).map_err(|_| WalletError::InvalidSecretKey)?;

        if secret_bytes.len() != 32 {
            return Err(WalletError::InvalidSecretKey);
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&secret_bytes);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let address = Self::derive_address(&signing_key.verifying_key());

        Ok(Wallet {
            signing_key,
            address,
        })
    }

    /// Derive a human-readable address from a public key
    /// Format: "edge" + first 40 chars of SHA256(public_key)
    fn derive_address(verifying_key: &VerifyingKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifying_key.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);
        format!("edge{}", &hash_hex[..40])
    }

    /// Get the wallet's address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().as_bytes())
    }

    /// Get the secret key as hex string (KEEP THIS PRIVATE!)
    pub fn secret_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    /// Sign a message and return the signature as hex string
    pub fn sign(&self, message: &[u8]) -> String {
        let signature = self.signing_key.sign(message);
        hex::encode(signature.to_bytes())
    }

    /// Sign a transaction hash
    pub fn sign_transaction(&self, tx_hash: &str) -> String {
        self.sign(tx_hash.as_bytes())
    }

    /// Export wallet as JSON for storage
    pub fn export(&self) -> WalletExport {
        WalletExport {
            address: self.address.clone(),
            public_key: self.public_key_hex(),
            secret_key: self.secret_key_hex(),
        }
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .field("public_key", &self.public_key_hex())
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

/// Exportable wallet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletExport {
    pub address: String,
    pub public_key: String,
    pub secret_key: String,
}

/// Wallet-related errors
#[derive(Debug, Clone)]
pub enum WalletError {
    InvalidSecretKey,
    InvalidPublicKey,
    InvalidSignature,
    SignatureVerificationFailed,
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::InvalidSecretKey => write!(f, "Invalid secret key"),
            WalletError::InvalidPublicKey => write!(f, "Invalid public key"),
            WalletError::InvalidSignature => write!(f, "Invalid signature format"),
            WalletError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
        }
    }
}

/// Verify a signature against a message and public key
pub fn verify_signature(
    public_key_hex: &str,
    message: &[u8],
    signature_hex: &str,
) -> Result<bool, WalletError> {
    // Decode public key
    let public_bytes = hex::decode(public_key_hex).map_err(|_| WalletError::InvalidPublicKey)?;

    if public_bytes.len() != 32 {
        return Err(WalletError::InvalidPublicKey);
    }

    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&public_bytes);

    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|_| WalletError::InvalidPublicKey)?;

    // Decode signature
    let sig_bytes = hex::decode(signature_hex).map_err(|_| WalletError::InvalidSignature)?;
    if sig_bytes.len() != 64 {
        return Err(WalletError::InvalidSignature);
    }
    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_array);

    // Verify
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Derive address from public key hex
pub fn address_from_public_key(public_key_hex: &str) -> Result<String, WalletError> {
    let public_bytes = hex::decode(public_key_hex).map_err(|_| WalletError::InvalidPublicKey)?;

    if public_bytes.len() != 32 {
        return Err(WalletError::InvalidPublicKey);
    }

    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(&public_bytes);

    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|_| WalletError::InvalidPublicKey)?;

    let mut hasher = Sha256::new();
    hasher.update(verifying_key.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);
    Ok(format!("edge{}", &hash_hex[..40]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new();
        assert!(wallet.address().starts_with("edge"));
        assert_eq!(wallet.address().len(), 44); // "edge" + 40 hex chars
    }

    #[test]
    fn test_wallet_from_secret() {
        let wallet1 = Wallet::new();
        let secret = wallet1.secret_key_hex();

        let wallet2 = Wallet::from_secret_key(&secret).unwrap();
        assert_eq!(wallet1.address(), wallet2.address());
        assert_eq!(wallet1.public_key_hex(), wallet2.public_key_hex());
    }

    #[test]
    fn test_sign_and_verify() {
        let wallet = Wallet::new();
        let message = b"Hello, EdgeAI!";

        let signature = wallet.sign(message);
        let is_valid = verify_signature(&wallet.public_key_hex(), message, &signature).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_invalid_signature() {
        let wallet1 = Wallet::new();
        let wallet2 = Wallet::new();
        let message = b"Hello, EdgeAI!";

        let signature = wallet1.sign(message);
        let is_valid = verify_signature(
            &wallet2.public_key_hex(), // Wrong public key
            message,
            &signature,
        )
        .unwrap();

        assert!(!is_valid);
    }
}
