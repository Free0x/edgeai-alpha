/**
 * PancakeSwap Integration for EdgeAI DEX
 * Supports BSC Mainnet and Testnet
 */

import { ethers } from 'ethers';

// Contract Addresses
export const CONTRACTS = {
  // BSC Mainnet
  mainnet: {
    EDGEAI: '0x276b792D11B9e3712FE6A78A460a0DEb416baB0A',
    WBNB: '0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c',
    PAIR: '0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06',
    ROUTER: '0x10ED43C718714eb63d5aA57B78B54704E256024E', // PancakeSwap V2 Router
    FACTORY: '0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73',
  },
  // BSC Testnet
  testnet: {
    EDGEAI: '0xEe3131549D8727bBCd6e628D90D6b57cf99F5794', // wEDGE on testnet
    WBNB: '0xae13d989daC2f0dEbFf460aC112a837C89BAa7cd',
    ROUTER: '0xD99D1c33F9fC3444f8101754aBC46c52416550D1', // PancakeSwap Testnet Router
    FACTORY: '0x6725F303b657a9451d8BA641348b6761A6CC7a17',
  }
};

// ABIs
export const ROUTER_ABI = [
  'function getAmountsOut(uint amountIn, address[] memory path) public view returns (uint[] memory amounts)',
  'function getAmountsIn(uint amountOut, address[] memory path) public view returns (uint[] memory amounts)',
  'function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)',
  'function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)',
  'function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)',
  'function addLiquidity(address tokenA, address tokenB, uint amountADesired, uint amountBDesired, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB, uint liquidity)',
  'function addLiquidityETH(address token, uint amountTokenDesired, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external payable returns (uint amountToken, uint amountETH, uint liquidity)',
  'function removeLiquidity(address tokenA, address tokenB, uint liquidity, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB)',
  'function removeLiquidityETH(address token, uint liquidity, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external returns (uint amountToken, uint amountETH)',
];

export const PAIR_ABI = [
  'function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)',
  'function token0() external view returns (address)',
  'function token1() external view returns (address)',
  'function totalSupply() external view returns (uint)',
  'function balanceOf(address owner) external view returns (uint)',
  'event Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)',
  'event Sync(uint112 reserve0, uint112 reserve1)',
];

export const ERC20_ABI = [
  'function name() view returns (string)',
  'function symbol() view returns (string)',
  'function decimals() view returns (uint8)',
  'function totalSupply() view returns (uint256)',
  'function balanceOf(address) view returns (uint256)',
  'function allowance(address owner, address spender) view returns (uint256)',
  'function approve(address spender, uint256 amount) returns (bool)',
  'function transfer(address to, uint256 amount) returns (bool)',
];

export const FACTORY_ABI = [
  'function getPair(address tokenA, address tokenB) external view returns (address pair)',
  'function allPairs(uint) external view returns (address pair)',
  'function allPairsLength() external view returns (uint)',
];

// Types
export interface TokenInfo {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  balance?: string;
}

export interface PairInfo {
  address: string;
  token0: TokenInfo;
  token1: TokenInfo;
  reserve0: string;
  reserve1: string;
  totalSupply: string;
  price: number;
  priceUSD?: number;
}

export interface SwapQuote {
  amountIn: string;
  amountOut: string;
  priceImpact: number;
  path: string[];
  minimumReceived: string;
}

export interface TradeHistory {
  txHash: string;
  type: 'buy' | 'sell';
  amountIn: string;
  amountOut: string;
  tokenIn: string;
  tokenOut: string;
  price: number;
  timestamp: number;
  maker: string;
}

// Helper functions
export function getProvider(isTestnet: boolean = false): ethers.JsonRpcProvider {
  const rpcUrl = isTestnet 
    ? 'https://data-seed-prebsc-1-s1.binance.org:8545'
    : 'https://bsc-dataseed1.binance.org';
  return new ethers.JsonRpcProvider(rpcUrl);
}

export function getContracts(isTestnet: boolean = false) {
  return isTestnet ? CONTRACTS.testnet : CONTRACTS.mainnet;
}

// Get pair info
export async function getPairInfo(isTestnet: boolean = false): Promise<PairInfo | null> {
  try {
    const provider = getProvider(isTestnet);
    const contracts = getContracts(isTestnet);
    
    if (!contracts.PAIR) {
      // For testnet, we might need to get pair from factory
      return null;
    }
    
    const pairContract = new ethers.Contract(contracts.PAIR, PAIR_ABI, provider);
    const edgeaiContract = new ethers.Contract(contracts.EDGEAI, ERC20_ABI, provider);
    const wbnbContract = new ethers.Contract(contracts.WBNB, ERC20_ABI, provider);
    
    const [reserves, token0Addr, edgeaiSymbol, edgeaiDecimals, wbnbSymbol, wbnbDecimals, totalSupply] = await Promise.all([
      pairContract.getReserves(),
      pairContract.token0(),
      edgeaiContract.symbol(),
      edgeaiContract.decimals(),
      wbnbContract.symbol(),
      wbnbContract.decimals(),
      pairContract.totalSupply(),
    ]);
    
    const isToken0EDGEAI = token0Addr.toLowerCase() === contracts.EDGEAI.toLowerCase();
    
    const reserve0 = ethers.formatUnits(reserves[0], isToken0EDGEAI ? edgeaiDecimals : wbnbDecimals);
    const reserve1 = ethers.formatUnits(reserves[1], isToken0EDGEAI ? wbnbDecimals : edgeaiDecimals);
    
    const edgeaiReserve = isToken0EDGEAI ? reserve0 : reserve1;
    const wbnbReserve = isToken0EDGEAI ? reserve1 : reserve0;
    
    // Price in WBNB
    const price = parseFloat(wbnbReserve) / parseFloat(edgeaiReserve);
    
    return {
      address: contracts.PAIR,
      token0: {
        address: isToken0EDGEAI ? contracts.EDGEAI : contracts.WBNB,
        symbol: isToken0EDGEAI ? edgeaiSymbol : wbnbSymbol,
        name: isToken0EDGEAI ? 'EdgeAI' : 'Wrapped BNB',
        decimals: isToken0EDGEAI ? Number(edgeaiDecimals) : Number(wbnbDecimals),
      },
      token1: {
        address: isToken0EDGEAI ? contracts.WBNB : contracts.EDGEAI,
        symbol: isToken0EDGEAI ? wbnbSymbol : edgeaiSymbol,
        name: isToken0EDGEAI ? 'Wrapped BNB' : 'EdgeAI',
        decimals: isToken0EDGEAI ? Number(wbnbDecimals) : Number(edgeaiDecimals),
      },
      reserve0: edgeaiReserve,
      reserve1: wbnbReserve,
      totalSupply: ethers.formatUnits(totalSupply, 18),
      price,
    };
  } catch (error) {
    console.error('Failed to get pair info:', error);
    return null;
  }
}

// Get swap quote
export async function getSwapQuote(
  amountIn: string,
  tokenIn: string,
  tokenOut: string,
  slippage: number = 0.5,
  isTestnet: boolean = false
): Promise<SwapQuote | null> {
  try {
    const provider = getProvider(isTestnet);
    const contracts = getContracts(isTestnet);
    
    const router = new ethers.Contract(contracts.ROUTER, ROUTER_ABI, provider);
    
    const tokenInContract = new ethers.Contract(tokenIn, ERC20_ABI, provider);
    const tokenOutContract = new ethers.Contract(tokenOut, ERC20_ABI, provider);
    
    const [decimalsIn, decimalsOut] = await Promise.all([
      tokenInContract.decimals(),
      tokenOutContract.decimals(),
    ]);
    
    const amountInWei = ethers.parseUnits(amountIn, decimalsIn);
    const path = [tokenIn, tokenOut];
    
    const amounts = await router.getAmountsOut(amountInWei, path);
    const amountOut = ethers.formatUnits(amounts[1], decimalsOut);
    
    // Calculate price impact
    const pairInfo = await getPairInfo(isTestnet);
    let priceImpact = 0;
    if (pairInfo) {
      const reserveIn = tokenIn.toLowerCase() === contracts.EDGEAI.toLowerCase() 
        ? parseFloat(pairInfo.reserve0) 
        : parseFloat(pairInfo.reserve1);
      priceImpact = (parseFloat(amountIn) / reserveIn) * 100;
    }
    
    // Calculate minimum received with slippage
    const minReceived = parseFloat(amountOut) * (1 - slippage / 100);
    
    return {
      amountIn,
      amountOut,
      priceImpact,
      path,
      minimumReceived: minReceived.toFixed(6),
    };
  } catch (error) {
    console.error('Failed to get swap quote:', error);
    return null;
  }
}

// Execute swap (requires connected wallet)
export async function executeSwap(
  signer: ethers.Signer,
  amountIn: string,
  amountOutMin: string,
  path: string[],
  deadline: number = 20, // minutes
  isTestnet: boolean = false
): Promise<ethers.TransactionResponse | null> {
  try {
    const contracts = getContracts(isTestnet);
    const router = new ethers.Contract(contracts.ROUTER, ROUTER_ABI, signer);
    
    const tokenIn = new ethers.Contract(path[0], ERC20_ABI, signer);
    const decimalsIn = await tokenIn.decimals();
    const decimalsOut = await new ethers.Contract(path[path.length - 1], ERC20_ABI, signer).decimals();
    
    const amountInWei = ethers.parseUnits(amountIn, decimalsIn);
    const amountOutMinWei = ethers.parseUnits(amountOutMin, decimalsOut);
    const deadlineTimestamp = Math.floor(Date.now() / 1000) + deadline * 60;
    const to = await signer.getAddress();
    
    // Check and approve if needed
    const allowance = await tokenIn.allowance(to, contracts.ROUTER);
    if (allowance < amountInWei) {
      const approveTx = await tokenIn.approve(contracts.ROUTER, ethers.MaxUint256);
      await approveTx.wait();
    }
    
    // Execute swap
    const isETHIn = path[0].toLowerCase() === contracts.WBNB.toLowerCase();
    const isETHOut = path[path.length - 1].toLowerCase() === contracts.WBNB.toLowerCase();
    
    let tx: ethers.TransactionResponse;
    
    if (isETHIn) {
      tx = await router.swapExactETHForTokens(
        amountOutMinWei,
        path,
        to,
        deadlineTimestamp,
        { value: amountInWei }
      );
    } else if (isETHOut) {
      tx = await router.swapExactTokensForETH(
        amountInWei,
        amountOutMinWei,
        path,
        to,
        deadlineTimestamp
      );
    } else {
      tx = await router.swapExactTokensForTokens(
        amountInWei,
        amountOutMinWei,
        path,
        to,
        deadlineTimestamp
      );
    }
    
    return tx;
  } catch (error) {
    console.error('Failed to execute swap:', error);
    return null;
  }
}

// Get token balance
export async function getTokenBalance(
  tokenAddress: string,
  walletAddress: string,
  isTestnet: boolean = false
): Promise<string> {
  try {
    const provider = getProvider(isTestnet);
    const token = new ethers.Contract(tokenAddress, ERC20_ABI, provider);
    const [balance, decimals] = await Promise.all([
      token.balanceOf(walletAddress),
      token.decimals(),
    ]);
    return ethers.formatUnits(balance, decimals);
  } catch (error) {
    console.error('Failed to get token balance:', error);
    return '0';
  }
}

// Get recent trades from pair events
export async function getRecentTrades(
  limit: number = 50,
  isTestnet: boolean = false
): Promise<TradeHistory[]> {
  try {
    const provider = getProvider(isTestnet);
    const contracts = getContracts(isTestnet);
    
    if (!contracts.PAIR) return [];
    
    const pair = new ethers.Contract(contracts.PAIR, PAIR_ABI, provider);
    
    // Get recent Swap events
    const filter = pair.filters.Swap();
    const currentBlock = await provider.getBlockNumber();
    const fromBlock = currentBlock - 5000; // ~4 hours of blocks
    
    const events = await pair.queryFilter(filter, fromBlock, currentBlock);
    
    const trades: TradeHistory[] = [];
    
    for (const event of events.slice(-limit)) {
      const block = await event.getBlock();
      const args = event.args;
      
      if (!args) continue;
      
      const isBuy = args.amount0In > 0; // EDGEAI in = sell, BNB in = buy
      
      trades.push({
        txHash: event.transactionHash,
        type: isBuy ? 'buy' : 'sell',
        amountIn: ethers.formatUnits(isBuy ? args.amount1In : args.amount0In, 18),
        amountOut: ethers.formatUnits(isBuy ? args.amount0Out : args.amount1Out, 18),
        tokenIn: isBuy ? 'WBNB' : 'EDGEAI',
        tokenOut: isBuy ? 'EDGEAI' : 'WBNB',
        price: 0, // Calculate from amounts
        timestamp: block?.timestamp || 0,
        maker: args.sender,
      });
    }
    
    return trades.reverse();
  } catch (error) {
    console.error('Failed to get recent trades:', error);
    return [];
  }
}

// Fetch data from DEX Screener API
export async function fetchDexScreenerData() {
  try {
    const res = await fetch(
      'https://api.dexscreener.com/latest/dex/pairs/bsc/0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06',
      { cache: 'no-store' }
    );
    
    if (!res.ok) throw new Error('Failed to fetch');
    
    const data = await res.json();
    
    if (data.pairs && data.pairs[0]) {
      const pair = data.pairs[0];
      return {
        priceUsd: parseFloat(pair.priceUsd),
        priceNative: parseFloat(pair.priceNative),
        volume24h: pair.volume?.h24 || 0,
        priceChange: {
          m5: pair.priceChange?.m5 || 0,
          h1: pair.priceChange?.h1 || 0,
          h6: pair.priceChange?.h6 || 0,
          h24: pair.priceChange?.h24 || 0,
        },
        liquidity: {
          usd: pair.liquidity?.usd || 0,
          base: pair.liquidity?.base || 0,
          quote: pair.liquidity?.quote || 0,
        },
        txns: {
          buys: pair.txns?.h24?.buys || 0,
          sells: pair.txns?.h24?.sells || 0,
        },
        fdv: pair.fdv || 0,
        marketCap: pair.marketCap || 0,
        pairCreatedAt: pair.pairCreatedAt,
      };
    }
    
    return null;
  } catch (error) {
    console.error('Failed to fetch DEX Screener data:', error);
    return null;
  }
}

// Get top holders from BSCScan (requires API key in production)
export async function getTopHolders(limit: number = 10): Promise<{ address: string; balance: string; percentage: number }[]> {
  // In production, use BSCScan API
  // For now, return mock data
  return [
    { address: '0x1234...5678', balance: '50,000,000', percentage: 5.0 },
    { address: '0x2345...6789', balance: '35,000,000', percentage: 3.5 },
    { address: '0x3456...7890', balance: '25,000,000', percentage: 2.5 },
    { address: '0x4567...8901', balance: '20,000,000', percentage: 2.0 },
    { address: '0x5678...9012', balance: '15,000,000', percentage: 1.5 },
  ];
}

// Format helpers
export function formatUSD(value: number): string {
  if (value >= 1_000_000_000) return `$${(value / 1_000_000_000).toFixed(2)}B`;
  if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(2)}M`;
  if (value >= 1_000) return `$${(value / 1_000).toFixed(2)}K`;
  return `$${value.toFixed(2)}`;
}

export function formatTokenAmount(value: number | string, decimals: number = 4): string {
  const num = typeof value === 'string' ? parseFloat(value) : value;
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(2)}M`;
  if (num >= 1_000) return `${(num / 1_000).toFixed(2)}K`;
  return num.toFixed(decimals);
}

export function shortenAddress(address: string, chars: number = 4): string {
  return `${address.slice(0, chars + 2)}...${address.slice(-chars)}`;
}
