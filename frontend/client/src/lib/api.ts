import { toast } from "sonner";
import { generateIoTTransaction, getIoTTransactions as getLocalIoTTransactions, IoTTransaction } from "./iotGenerator";
import { getValidators as getLocalValidators, Validator } from "./validators";

import { API_BASE_URL } from './config';

export async function apiCall<T>(endpoint: string, options: RequestInit = {}): Promise<{ success: boolean; data: T; error?: string }> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });

    // Check if response is JSON
    const contentType = response.headers.get("content-type");
    if (contentType && contentType.indexOf("application/json") !== -1) {
      const data = await response.json();
      return data;
    } else {
      // Handle non-JSON response (e.g., 404 page or text error)
      const text = await response.text();
      console.error(`API Error (${endpoint}): Non-JSON response`, text.substring(0, 100));
      return { success: false, data: {} as T, error: "Invalid API response format" };
    }
  } catch (error) {
    // Fallback to simulated data if API fails
    console.warn(`API Error (${endpoint}): ${error}. Falling back to simulated data.`);
    
    if (endpoint.includes('/blocks')) {
      // Generate simulated blocks
      const simulatedBlocks: Block[] = Array(10).fill(0).map((_, i) => ({
        index: 1000 - i,
        timestamp: Date.now() - i * 3000,
        transactions: [],
        previous_hash: "0x" + Math.random().toString(16).substring(2, 42),
        hash: "0x" + Math.random().toString(16).substring(2, 42),
        validator: "edge_node_" + Math.floor(Math.random() * 100).toString().padStart(2, '0'),
        entropy: Math.random() * 5,
        difficulty: 2
      }));
      return { success: true, data: simulatedBlocks as unknown as T };
    }
    
    console.error(`API Error (${endpoint}):`, error);
    return { success: false, data: {} as T, error: String(error) };
  }
}

// ============ Block Types ============

// Block header structure from backend
export interface BlockHeader {
  version: number;
  previous_hash: string;
  merkle_root: string;
  timestamp: string; // ISO 8601 format from backend
  difficulty: number;
  nonce: number;
  data_entropy: number; // Shannon entropy calculated by PoIE
}

// Full block structure from backend
export interface Block {
  index: number;
  header?: BlockHeader; // Optional for backward compatibility with fallback simulation
  transactions: Transaction[];
  hash: string;
  validator: string;
  // Legacy flat fields for backward compatibility (used in fallback simulation)
  timestamp?: number;
  entropy?: number;
  difficulty?: number;
  previous_hash?: string;
}

export interface Transaction {
  id: string;
  tx_type: 'Genesis' | 'Transfer' | 'DataContribution' | 'DataPurchase' | 'Reward';
  from: string;
  to?: string;
  amount: number;
  timestamp: string;
  signature?: string;
  payload?: any;
}

export interface MarketplaceListing {
  id: string;
  owner: string;
  title?: string;
  category?: string;
  price: number;
  quality_score: number;
  data_hash: string;
  timestamp: string;
}

// ============ IoT API ============

export interface IoTTransactionListResponse {
  transactions: IoTTransaction[];
  total: number;
  page: number;
  limit: number;
}

/**
 * 获取 IoT 交易列表
 * 优先从后端 API 获取，失败时回退到本地生成
 */
export async function fetchIoTTransactions(
  page: number = 1,
  limit: number = 20,
  sector?: string
): Promise<IoTTransactionListResponse> {
  try {
    let endpoint = `/iot/transactions?page=${page}&limit=${limit}`;
    if (sector) {
      endpoint += `&sector=${encodeURIComponent(sector)}`;
    }

    const response = await apiCall<IoTTransactionListResponse>(endpoint);
    
    if (response.success && response.data.transactions) {
      return response.data;
    }
    
    throw new Error(response.error || "Failed to fetch IoT transactions");
  } catch (error) {
    console.warn("Falling back to local IoT data generation:", error);
    
    // 回退到本地生成
    const transactions = getLocalIoTTransactions(page, limit);
    return {
      transactions,
      total: 100000,
      page,
      limit,
    };
  }
}

// ============ Validator API ============

export interface ValidatorStats {
  online: number;
  offline: number;
  maintenance: number;
  /** Network Entropy - PoIE 核心指标 */
  network_entropy: number;
  /** 总挖矿区块数 */
  total_blocks_mined: number;
  /** 平均信誉值 */
  avg_reputation: number;
}

export interface ValidatorListResponse {
  validators: Validator[];
  total: number;
  page: number;
  limit: number;
  stats: ValidatorStats;
}

export interface GlobeMarker {
  location: [number, number];
  size: number;
  tooltip: string;
  type: string;
  validator_count: number;
}

export interface ValidatorMapResponse {
  markers: GlobeMarker[];
  total_validators: number;
}

/**
 * 获取验证者节点列表
 * 优先从后端 API 获取，失败时回退到本地生成
 */
export async function fetchValidatorNodes(
  page: number = 1,
  limit: number = 100,
  status?: string
): Promise<ValidatorListResponse> {
  try {
    let endpoint = `/validators/nodes?page=${page}&limit=${limit}`;
    if (status) {
      endpoint += `&status=${encodeURIComponent(status)}`;
    }

    const response = await apiCall<ValidatorListResponse>(endpoint);
    
    if (response.success && response.data.validators) {
      const data = response.data;
      
      // 如果后端还没有返回新字段，前端补充计算
      if (data.stats && data.stats.network_entropy === undefined) {
        const validators = data.validators;
        const totalBlocksMined = validators.reduce((acc, v) => acc + v.blocks_mined, 0);
        const avgReputation = validators.length > 0 
          ? validators.reduce((acc, v) => acc + v.reputation, 0) / validators.length 
          : 0;
        
        // 计算 Network Entropy (Shannon 信息熵)
        let networkEntropy = 0;
        if (totalBlocksMined > 0) {
          for (const v of validators) {
            if (v.blocks_mined > 0) {
              const p = v.blocks_mined / totalBlocksMined;
              networkEntropy -= p * Math.log2(p);
            }
          }
          // 缩放到合理范围
          networkEntropy *= Math.log2(data.total || totalBlocksMined);
        }
        
        data.stats = {
          ...data.stats,
          network_entropy: networkEntropy,
          total_blocks_mined: totalBlocksMined * (data.total / validators.length),
          avg_reputation: avgReputation,
        };
      }
      
      return data;
    }
    
    throw new Error(response.error || "Failed to fetch validators");
  } catch (error) {
    console.warn("Falling back to local validator data generation:", error);
    
    // 回退到本地生成
    const allValidators = getLocalValidators();
    const start = (page - 1) * limit;
    const validators = allValidators.slice(start, start + limit);
    
    // 计算统计信息
    const totalBlocksMined = allValidators.reduce((acc, v) => acc + v.blocks_mined, 0);
    const avgReputation = allValidators.length > 0 
      ? allValidators.reduce((acc, v) => acc + v.reputation, 0) / allValidators.length 
      : 0;
    
    // 计算 Network Entropy (Shannon 信息熵)
    let networkEntropy = 0;
    if (totalBlocksMined > 0) {
      for (const v of allValidators) {
        if (v.blocks_mined > 0) {
          const p = v.blocks_mined / totalBlocksMined;
          networkEntropy -= p * Math.log2(p);
        }
      }
      networkEntropy *= Math.log2(totalBlocksMined);
    }

    const stats: ValidatorStats = {
      online: allValidators.filter(v => v.status === 'online').length,
      offline: allValidators.filter(v => v.status === 'offline').length,
      maintenance: allValidators.filter(v => v.status === 'maintenance').length,
      network_entropy: networkEntropy,
      total_blocks_mined: totalBlocksMined,
      avg_reputation: avgReputation,
    };
    
    return {
      validators,
      total: allValidators.length,
      page,
      limit,
      stats,
    };
  }
}

/**
 * 获取验证者地图数据
 * 优先从后端 API 获取，失败时回退到本地生成
 */
export async function fetchValidatorMap(): Promise<ValidatorMapResponse> {
  try {
    const response = await apiCall<ValidatorMapResponse>('/validators/map');
    
    if (response.success && response.data.markers) {
      return response.data;
    }
    
    throw new Error(response.error || "Failed to fetch validator map");
  } catch (error) {
    console.warn("Falling back to local validator map generation:", error);
    
    // 回退到本地生成（简化版）
    const allValidators = getLocalValidators();
    
    // 按位置聚合
    const locationCounts = new Map<string, { lat: number; lng: number; count: number }>();
    
    for (const v of allValidators) {
      const key = v.location;
      if (locationCounts.has(key)) {
        locationCounts.get(key)!.count++;
      } else {
        locationCounts.set(key, { lat: v.lat, lng: v.lng, count: 1 });
      }
    }
    
    const markers: GlobeMarker[] = Array.from(locationCounts.entries()).map(([name, data]) => ({
      location: [data.lat, data.lng] as [number, number],
      size: 0.03 + (data.count / allValidators.length) * 0.15,
      tooltip: `${name} (${data.count} nodes)`,
      type: data.count > 1000 ? 'hub' : 'node',
      validator_count: data.count,
    }));
    
    return {
      markers,
      total_validators: allValidators.length,
    };
  }
}

// ============ Chain Stats API ============

export interface ChainStats {
  height: number;
  total_transactions: number;
  total_supply: number;
  total_staked: number;
  active_accounts: number;
  data_entries: number;
  difficulty: number;
  last_block_time: number;
  // PoIE Network Metrics
  network_entropy: number;
  avg_tx_per_block: number;
  data_throughput: number;
  tps: number;
  validator_power: number;
}

/**
 * 获取区块链状态信息
 */
export async function fetchChainStats(): Promise<ChainStats> {
  try {
    const response = await apiCall<ChainStats>('/chain');
    
    if (response.success && response.data) {
      return response.data;
    }
    
    throw new Error(response.error || "Failed to fetch chain stats");
  } catch (error) {
    console.warn("Failed to fetch chain stats:", error);
    throw error;
  }
}

// ============ Network Stats API ============

export interface NetworkStats {
  height: number;
  difficulty: number;
  avg_block_time: number;
  tps: number;
  total_transactions: number;
  total_validators: number;
  active_validators: number;
  sectors: Record<string, number>;
}

/**
 * 获取增强版网络统计信息
 */
export async function fetchNetworkStats(): Promise<NetworkStats> {
  try {
    const response = await apiCall<NetworkStats>('/network/stats');
    
    if (response.success && response.data) {
      return response.data;
    }
    
    throw new Error(response.error || "Failed to fetch network stats");
  } catch (error) {
    console.warn("Falling back to simulated network stats:", error);
    
    // 回退到模拟数据
    return {
      height: 12045,
      difficulty: 2.14,
      avg_block_time: 10.2,
      tps: 15.4,
      total_transactions: 589201,
      total_validators: 30000,
      active_validators: 29100,
      sectors: {
        smart_city: 15234,
        industrial: 12456,
        agriculture: 8765,
        healthcare: 6543,
        logistics: 5432,
        energy: 4321,
      },
    };
  }
}
