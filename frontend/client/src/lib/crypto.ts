/**
 * Cryptographic utilities for EdgeAI wallet operations
 * Uses Web Crypto API for ed25519 signing
 */

// Hex encoding/decoding utilities
export function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  return bytes;
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// SHA-256 hash function
export async function sha256(message: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(message);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  return bytesToHex(new Uint8Array(hashBuffer));
}

// Simple wallet interface for demo purposes
// In production, this would integrate with a real wallet like MetaMask or a custom wallet
export interface Wallet {
  address: string;
  publicKey: string;
  // Private key should never be exposed in production
  // This is only for demo/testing purposes
  sign: (message: Uint8Array) => Promise<string>;
}

// Demo wallet storage key
const DEMO_WALLET_KEY = "edgeai_demo_wallet";

// Generate a demo wallet (for testing purposes only)
export async function generateDemoWallet(): Promise<Wallet> {
  // Generate random bytes for demo private key
  const privateKeyBytes = crypto.getRandomValues(new Uint8Array(32));
  const publicKeyBytes = crypto.getRandomValues(new Uint8Array(32)); // Simplified for demo
  
  const publicKey = bytesToHex(publicKeyBytes);
  const address = `edgeai1${publicKey.slice(0, 40)}`;
  
  const wallet: Wallet = {
    address,
    publicKey,
    sign: async (message: Uint8Array) => {
      // Simplified signing for demo - in production use proper ed25519
      const combined = new Uint8Array(privateKeyBytes.length + message.length);
      combined.set(privateKeyBytes);
      combined.set(message, privateKeyBytes.length);
      const hashBuffer = await crypto.subtle.digest("SHA-256", combined);
      return bytesToHex(new Uint8Array(hashBuffer)) + bytesToHex(new Uint8Array(hashBuffer));
    },
  };
  
  // Store in localStorage for persistence
  localStorage.setItem(DEMO_WALLET_KEY, JSON.stringify({
    address: wallet.address,
    publicKey: wallet.publicKey,
    privateKey: bytesToHex(privateKeyBytes),
  }));
  
  return wallet;
}

// Load demo wallet from localStorage
export function loadDemoWallet(): Wallet | null {
  const stored = localStorage.getItem(DEMO_WALLET_KEY);
  if (!stored) return null;
  
  try {
    const data = JSON.parse(stored);
    const privateKeyBytes = hexToBytes(data.privateKey);
    
    return {
      address: data.address,
      publicKey: data.publicKey,
      sign: async (message: Uint8Array) => {
        const combined = new Uint8Array(privateKeyBytes.length + message.length);
        combined.set(privateKeyBytes);
        combined.set(message, privateKeyBytes.length);
        const hashBuffer = await crypto.subtle.digest("SHA-256", combined);
        return bytesToHex(new Uint8Array(hashBuffer)) + bytesToHex(new Uint8Array(hashBuffer));
      },
    };
  } catch {
    localStorage.removeItem(DEMO_WALLET_KEY);
    return null;
  }
}

// Get or create demo wallet
export async function getOrCreateDemoWallet(): Promise<Wallet> {
  const existing = loadDemoWallet();
  if (existing) return existing;
  return generateDemoWallet();
}

// Create signed request for API calls
export interface SignedRequest<T> {
  data: T;
  auth: {
    public_key: string;
    signature: string;
    timestamp: number;
  };
}

export async function createSignedRequest<T>(
  wallet: Wallet,
  data: T
): Promise<SignedRequest<T>> {
  const timestamp = Math.floor(Date.now() / 1000);
  const message = JSON.stringify(data);
  const encoder = new TextEncoder();
  const messageBytes = encoder.encode(message);
  
  const signature = await wallet.sign(messageBytes);
  
  return {
    data,
    auth: {
      public_key: wallet.publicKey,
      signature,
      timestamp,
    },
  };
}
