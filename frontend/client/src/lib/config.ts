// API Configuration
// In production (Vercel), use relative path to leverage Vercel's rewrite proxy
// In development (localhost), use direct API URL

const isDevelopment = typeof window !== 'undefined' &&
  (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');

export const API_BASE_URL = isDevelopment
  ? 'https://edgeai-blockchain-node.fly.dev/api'  // Direct access in development
  : '/api';  // Use Vercel proxy in production (works for any domain)

export const getApiUrl = (path: string) => {
  const cleanPath = path.startsWith('/') ? path.slice(1) : path;
  return `${API_BASE_URL}/${cleanPath}`;
};
