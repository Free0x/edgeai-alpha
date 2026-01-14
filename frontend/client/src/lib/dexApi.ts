import { API_BASE_URL } from './config';

const API_BASE = API_BASE_URL.replace('/api', '');

export interface TradingPair {
  id: string;
  base_token: string;
  quote_token: string;
  base_reserve: number;
  quote_reserve: number;
  total_liquidity: number;
  fee_rate: number;
  volume_24h: number;
  created_at: number;
}

export interface PairStats {
  pair: TradingPair;
  price: number;
  price_change_24h: number;
  high_24h: number;
  low_24h: number;
}

export interface SwapQuote {
  amount_in: number;
  amount_out: number;
  fee: number;
  price_impact: number;
  exchange_rate: number;
}

export interface Trade {
  id: string;
  pair_id: string;
  buyer: string;
  seller: string;
  price: number;
  amount: number;
  total: number;
  fee: number;
  timestamp: number;
}

export interface LiquidityPosition {
  pair_id: string;
  owner: string;
  lp_tokens: number;
  base_deposited: number;
  quote_deposited: number;
  created_at: number;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// Get all trading pairs
export async function fetchPairs(): Promise<PairStats[]> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/pairs`);
    const data: ApiResponse<PairStats[]> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return [];
  } catch (e) {
    console.error('Failed to fetch pairs:', e);
    return [];
  }
}

// Get specific trading pair
export async function fetchPair(pairId: string): Promise<PairStats | null> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/pairs/${pairId}`);
    const data: ApiResponse<PairStats> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return null;
  } catch (e) {
    console.error('Failed to fetch pair:', e);
    return null;
  }
}

// Get swap quote
export async function getSwapQuote(
  pairId: string,
  amountIn: number,
  isBaseToQuote: boolean
): Promise<SwapQuote | null> {
  try {
    const params = new URLSearchParams({
      pair_id: pairId,
      amount_in: amountIn.toString(),
      is_base_to_quote: isBaseToQuote.toString(),
      user: 'preview'
    });
    const res = await fetch(`${API_BASE}/api/dex/quote?${params}`);
    const data: ApiResponse<SwapQuote> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return null;
  } catch (e) {
    console.error('Failed to get swap quote:', e);
    return null;
  }
}

// Execute swap
export async function executeSwap(
  pairId: string,
  amountIn: number,
  isBaseToQuote: boolean,
  user: string,
  minAmountOut?: number
): Promise<Trade | null> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/swap`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        pair_id: pairId,
        amount_in: amountIn,
        is_base_to_quote: isBaseToQuote,
        user,
        min_amount_out: minAmountOut
      })
    });
    const data: ApiResponse<Trade> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return null;
  } catch (e) {
    console.error('Failed to execute swap:', e);
    return null;
  }
}

// Add liquidity
export async function addLiquidity(
  pairId: string,
  baseAmount: number,
  quoteAmount: number,
  user: string
): Promise<LiquidityPosition | null> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/liquidity`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        pair_id: pairId,
        base_amount: baseAmount,
        quote_amount: quoteAmount,
        user
      })
    });
    const data: ApiResponse<LiquidityPosition> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return null;
  } catch (e) {
    console.error('Failed to add liquidity:', e);
    return null;
  }
}

// Get recent trades
export async function fetchTrades(pairId: string): Promise<Trade[]> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/trades/${pairId}`);
    const data: ApiResponse<Trade[]> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return [];
  } catch (e) {
    console.error('Failed to fetch trades:', e);
    return [];
  }
}

// Get user positions
export async function fetchUserPositions(user: string): Promise<LiquidityPosition[]> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/positions/${user}`);
    const data: ApiResponse<LiquidityPosition[]> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return [];
  } catch (e) {
    console.error('Failed to fetch positions:', e);
    return [];
  }
}

// Create new trading pair
export async function createPair(
  baseToken: string,
  quoteToken: string,
  initialBaseAmount: number,
  initialQuoteAmount: number,
  creator: string,
  feeRate?: number
): Promise<TradingPair | null> {
  try {
    const res = await fetch(`${API_BASE}/api/dex/pairs/create`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        base_token: baseToken,
        quote_token: quoteToken,
        initial_base_amount: initialBaseAmount,
        initial_quote_amount: initialQuoteAmount,
        creator,
        fee_rate: feeRate
      })
    });
    const data: ApiResponse<TradingPair> = await res.json();
    if (data.success && data.data) {
      return data.data;
    }
    return null;
  } catch (e) {
    console.error('Failed to create pair:', e);
    return null;
  }
}

// Format large numbers
export function formatNumber(num: number, decimals: number = 2): string {
  if (num >= 1_000_000_000) {
    return (num / 1_000_000_000).toFixed(decimals) + 'B';
  }
  if (num >= 1_000_000) {
    return (num / 1_000_000).toFixed(decimals) + 'M';
  }
  if (num >= 1_000) {
    return (num / 1_000).toFixed(decimals) + 'K';
  }
  return num.toFixed(decimals);
}

// Format price with appropriate decimals
export function formatPrice(price: number): string {
  if (price < 0.0001) {
    return price.toExponential(4);
  }
  if (price < 1) {
    return price.toFixed(6);
  }
  if (price < 100) {
    return price.toFixed(4);
  }
  return price.toFixed(2);
}
