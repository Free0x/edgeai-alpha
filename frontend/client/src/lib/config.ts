// API Configuration
// In production on Vercel, use relative path to leverage Vercel's rewrite proxy
// This avoids CORS issues by routing through the same origin

const isProduction = typeof window !== 'undefined' && 
  window.location.hostname.includes('vercel.app');

export const API_BASE_URL = isProduction 
  ? '/api'  // Use Vercel proxy in production
  : 'https://edgeai-blockchain-node.fly.dev/api';  // Direct access in development

export const getApiUrl = (path: string) => {
  const cleanPath = path.startsWith('/') ? path.slice(1) : path;
  return `${API_BASE_URL}/${cleanPath}`;
};
