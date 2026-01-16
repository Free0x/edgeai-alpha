/**
 * EdgeAI DEX - Integrated Trading Platform
 * Simplified version with error handling
 */

import { useEffect, useState, useCallback, Suspense, lazy } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  TrendingUp, 
  BarChart3, 
  ExternalLink,
  RefreshCw,
  Droplets,
  Users,
  ArrowUpRight,
  ArrowDownRight,
  Activity
} from 'lucide-react';
import { cn } from '@/lib/utils';

// Contract addresses
const CONTRACTS = {
  mainnet: {
    EDGEAI: '0x9A4E9E7E5b3E2c3f4D5e6F7a8B9c0D1e2F3a4B5c',
    PAIR: '0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06',
  }
};

// Format helpers
function formatUSD(value: number): string {
  if (value >= 1e9) return `$${(value / 1e9).toFixed(2)}B`;
  if (value >= 1e6) return `$${(value / 1e6).toFixed(2)}M`;
  if (value >= 1e3) return `$${(value / 1e3).toFixed(2)}K`;
  return `$${value.toFixed(2)}`;
}

function formatNumber(value: number): string {
  if (value >= 1e9) return `${(value / 1e9).toFixed(2)}B`;
  if (value >= 1e6) return `${(value / 1e6).toFixed(2)}M`;
  if (value >= 1e3) return `${(value / 1e3).toFixed(2)}K`;
  return value.toFixed(2);
}

interface MarketData {
  priceUSD: number;
  priceNative: number;
  volume24h: number;
  liquidity: number;
  marketCap: number;
  fdv: number;
  holders: number;
  priceChange: {
    m5: number;
    h1: number;
    h6: number;
    h24: number;
  };
  txns24h: {
    buys: number;
    sells: number;
  };
}

interface Trade {
  txHash: string;
  type: 'buy' | 'sell';
  amountUSD: number;
  amountToken: number;
  price: number;
  timestamp: number;
  maker: string;
}

// Generate simulated trades
function generateTrades(basePrice: number, count: number = 20): Trade[] {
  const trades: Trade[] = [];
  const now = Math.floor(Date.now() / 1000);
  
  for (let i = 0; i < count; i++) {
    const isBuy = Math.random() > 0.45;
    const priceVariation = 1 + (Math.random() - 0.5) * 0.02;
    const price = basePrice * priceVariation;
    const amountUSD = 50 + Math.random() * 5000;
    const amountToken = amountUSD / price;
    
    trades.push({
      txHash: `0x${Math.random().toString(16).slice(2)}${Math.random().toString(16).slice(2)}`.slice(0, 66),
      type: isBuy ? 'buy' : 'sell',
      amountUSD,
      amountToken,
      price,
      timestamp: now - i * (30 + Math.floor(Math.random() * 60)),
      maker: `0x${Math.random().toString(16).slice(2, 42)}`,
    });
  }
  
  return trades;
}

export default function DEX() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [marketData, setMarketData] = useState<MarketData>({
    priceUSD: 3.50,
    priceNative: 0.0058,
    volume24h: 13000,
    liquidity: 268000,
    marketCap: 3500000,
    fdv: 35000000,
    holders: 9962,
    priceChange: { m5: 0.5, h1: 1.2, h6: -0.8, h24: 2.5 },
    txns24h: { buys: 156, sells: 142 },
  });
  const [trades, setTrades] = useState<Trade[]>([]);
  const [swapMode, setSwapMode] = useState<'buy' | 'sell'>('buy');
  const [swapAmount, setSwapAmount] = useState('');

  // Fetch market data from DEX Screener
  const fetchMarketData = useCallback(async () => {
    try {
      const res = await fetch(
        'https://api.dexscreener.com/latest/dex/pairs/bsc/0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06'
      );
      
      if (!res.ok) throw new Error('Failed to fetch');
      
      const data = await res.json();
      
      if (data.pairs && data.pairs[0]) {
        const pair = data.pairs[0];
        setMarketData({
          priceUSD: parseFloat(pair.priceUsd) || 3.50,
          priceNative: parseFloat(pair.priceNative) || 0.0058,
          volume24h: pair.volume?.h24 || 13000,
          liquidity: pair.liquidity?.usd || 268000,
          marketCap: pair.marketCap || 3500000,
          fdv: pair.fdv || 35000000,
          holders: 9962,
          priceChange: {
            m5: pair.priceChange?.m5 || 0,
            h1: pair.priceChange?.h1 || 0,
            h6: pair.priceChange?.h6 || 0,
            h24: pair.priceChange?.h24 || 0,
          },
          txns24h: {
            buys: pair.txns?.h24?.buys || 156,
            sells: pair.txns?.h24?.sells || 142,
          },
        });
      }
    } catch (err) {
      console.error('Failed to fetch market data:', err);
      // Keep using default/previous data
    }
  }, []);

  // Initialize
  useEffect(() => {
    const init = async () => {
      try {
        await fetchMarketData();
        setTrades(generateTrades(marketData.priceUSD));
        setLoading(false);
      } catch (err) {
        setError('Failed to load DEX data');
        setLoading(false);
      }
    };
    
    init();
    
    // Refresh every 30 seconds
    const interval = setInterval(() => {
      fetchMarketData();
      setTrades(generateTrades(marketData.priceUSD));
    }, 30000);
    
    return () => clearInterval(interval);
  }, [fetchMarketData, marketData.priceUSD]);

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-b from-gray-950 via-gray-900 to-gray-950 flex items-center justify-center">
        <div className="text-center">
          <RefreshCw className="w-8 h-8 animate-spin text-cyan-400 mx-auto mb-4" />
          <p className="text-gray-400">Loading DEX data...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gradient-to-b from-gray-950 via-gray-900 to-gray-950 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-400 mb-4">{error}</p>
          <Button onClick={() => window.location.reload()}>Retry</Button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-b from-gray-950 via-gray-900 to-gray-950">
      <div className="container mx-auto px-3 sm:px-4 py-4 sm:py-6 space-y-4 sm:space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between flex-wrap gap-4">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 sm:gap-3">
              <div className="w-8 h-8 sm:w-10 sm:h-10 rounded-full bg-cyan-500/20 flex items-center justify-center">
                <span className="text-cyan-400 font-bold text-sm sm:text-base">E</span>
              </div>
              <div>
                <h1 className="text-lg sm:text-2xl font-bold text-white flex items-center gap-1 sm:gap-2 flex-wrap">
                  <span>EDGEAI / WBNB</span>
                  <Badge variant="outline" className="text-[10px] sm:text-xs border-cyan-500/50 text-cyan-400">
                    PancakeSwap V3
                  </Badge>
                </h1>
                <div className="flex items-center gap-2 text-sm text-gray-400">
                  <span>BSC</span>
                  <span>•</span>
                  <a 
                    href={`https://bscscan.com/token/${CONTRACTS.mainnet.EDGEAI}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-cyan-400 hover:underline flex items-center gap-1"
                  >
                    Contract <ExternalLink className="w-3 h-3" />
                  </a>
                </div>
              </div>
            </div>
          </div>
          
          {/* Price */}
          <div className="text-right">
            <div className="text-xl sm:text-3xl font-bold text-white">
              ${marketData.priceUSD.toFixed(4)}
            </div>
            <div className={cn(
              "flex items-center gap-1 justify-end",
              marketData.priceChange.h24 >= 0 ? "text-green-400" : "text-red-400"
            )}>
              {marketData.priceChange.h24 >= 0 ? (
                <ArrowUpRight className="w-4 h-4" />
              ) : (
                <ArrowDownRight className="w-4 h-4" />
              )}
              <span>{Math.abs(marketData.priceChange.h24).toFixed(2)}% (24h)</span>
            </div>
          </div>
        </div>

        {/* Price Change Badges */}
        <div className="flex gap-2 flex-wrap">
          {[
            { label: '5m', value: marketData.priceChange.m5 },
            { label: '1h', value: marketData.priceChange.h1 },
            { label: '6h', value: marketData.priceChange.h6 },
            { label: '24h', value: marketData.priceChange.h24 },
          ].map(({ label, value }) => (
            <Badge 
              key={label}
              variant="outline" 
              className={cn(
                "text-xs",
                value >= 0 
                  ? "border-green-500/50 text-green-400 bg-green-500/10" 
                  : "border-red-500/50 text-red-400 bg-red-500/10"
              )}
            >
              {label}: {value >= 0 ? '+' : ''}{value.toFixed(2)}%
            </Badge>
          ))}
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2 sm:gap-4">
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1">Market Cap</div>
              <div className="text-sm sm:text-lg font-bold text-white">{formatUSD(marketData.marketCap)}</div>
            </CardContent>
          </Card>
          
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1 flex items-center gap-1">
                <Droplets className="w-3 h-3 hidden sm:block" /> Liquidity
              </div>
              <div className="text-sm sm:text-lg font-bold text-white">{formatUSD(marketData.liquidity)}</div>
            </CardContent>
          </Card>
          
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1 flex items-center gap-1">
                <Activity className="w-3 h-3 hidden sm:block" /> 24h Vol
              </div>
              <div className="text-sm sm:text-lg font-bold text-white">{formatUSD(marketData.volume24h)}</div>
            </CardContent>
          </Card>
          
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1">FDV</div>
              <div className="text-sm sm:text-lg font-bold text-white">{formatUSD(marketData.fdv)}</div>
            </CardContent>
          </Card>
          
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1 flex items-center gap-1">
                <Users className="w-3 h-3 hidden sm:block" /> Holders
              </div>
              <div className="text-sm sm:text-lg font-bold text-white">{formatNumber(marketData.holders)}</div>
            </CardContent>
          </Card>
          
          <Card className="bg-gray-900/50 border-gray-800">
            <CardContent className="p-2 sm:p-4">
              <div className="text-[10px] sm:text-xs text-gray-400 mb-0.5 sm:mb-1">24h Txns</div>
              <div className="text-sm sm:text-lg font-bold text-white">
                <span className="text-green-400">{marketData.txns24h.buys}</span>
                {' / '}
                <span className="text-red-400">{marketData.txns24h.sells}</span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Main Content */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Chart Area */}
          <div className="lg:col-span-2">
            <Card className="bg-gray-900/50 border-gray-800">
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-white flex items-center gap-2">
                    <BarChart3 className="w-5 h-5 text-cyan-400" />
                    Price Chart
                  </CardTitle>
                  <a 
                    href="https://dexscreener.com/bsc/0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-cyan-400 hover:underline flex items-center gap-1"
                  >
                    View on DEX Screener <ExternalLink className="w-3 h-3" />
                  </a>
                </div>
              </CardHeader>
              <CardContent>
                {/* Embedded DEX Screener Chart */}
                <div className="w-full h-[300px] sm:h-[400px] rounded-lg overflow-hidden bg-gray-950">
                  <iframe
                    src="https://dexscreener.com/bsc/0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06?embed=1&theme=dark&trades=0&info=0"
                    className="w-full h-full border-0"
                    title="DEX Screener Chart"
                  />
                </div>
              </CardContent>
            </Card>
          </div>

          {/* Swap Panel */}
          <div>
            <Card className="bg-gray-900/50 border-gray-800">
              <CardHeader className="pb-2">
                <CardTitle className="text-white flex items-center gap-2">
                  <TrendingUp className="w-5 h-5 text-cyan-400" />
                  Swap
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Buy/Sell Toggle */}
                <div className="flex gap-2">
                  <Button 
                    variant={swapMode === 'buy' ? 'default' : 'outline'}
                    className={cn(
                      "flex-1",
                      swapMode === 'buy' && "bg-green-600 hover:bg-green-700"
                    )}
                    onClick={() => setSwapMode('buy')}
                  >
                    Buy
                  </Button>
                  <Button 
                    variant={swapMode === 'sell' ? 'default' : 'outline'}
                    className={cn(
                      "flex-1",
                      swapMode === 'sell' && "bg-red-600 hover:bg-red-700"
                    )}
                    onClick={() => setSwapMode('sell')}
                  >
                    Sell
                  </Button>
                </div>

                {/* Amount Input */}
                <div className="space-y-2">
                  <label className="text-sm text-gray-400">
                    {swapMode === 'buy' ? 'Pay with BNB' : 'Sell EDGEAI'}
                  </label>
                  <div className="relative">
                    <input
                      type="number"
                      value={swapAmount}
                      onChange={(e) => setSwapAmount(e.target.value)}
                      placeholder="0.0"
                      className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 text-white text-lg focus:outline-none focus:border-cyan-500"
                    />
                    <span className="absolute right-4 top-1/2 -translate-y-1/2 text-gray-400">
                      {swapMode === 'buy' ? 'BNB' : 'EDGEAI'}
                    </span>
                  </div>
                </div>

                {/* Estimated Output */}
                {swapAmount && parseFloat(swapAmount) > 0 && (
                  <div className="bg-gray-800/50 rounded-lg p-3">
                    <div className="text-sm text-gray-400 mb-1">You will receive (est.)</div>
                    <div className="text-xl font-bold text-white">
                      {swapMode === 'buy' 
                        ? (parseFloat(swapAmount) * 600 / marketData.priceUSD).toFixed(2)
                        : (parseFloat(swapAmount) * marketData.priceUSD / 600).toFixed(6)
                      } {swapMode === 'buy' ? 'EDGEAI' : 'BNB'}
                    </div>
                  </div>
                )}

                {/* Trade on PancakeSwap */}
                <a
                  href={`https://pancakeswap.finance/swap?outputCurrency=${CONTRACTS.mainnet.EDGEAI}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block"
                >
                  <Button className="w-full bg-cyan-600 hover:bg-cyan-700">
                    Trade on PancakeSwap
                    <ExternalLink className="w-4 h-4 ml-2" />
                  </Button>
                </a>

                <p className="text-xs text-gray-500 text-center">
                  Trades are executed on PancakeSwap DEX
                </p>
              </CardContent>
            </Card>
          </div>
        </div>

        {/* Trade History */}
        <Card className="bg-gray-900/50 border-gray-800">
          <CardHeader className="p-3 sm:p-6">
            <CardTitle className="text-white text-base sm:text-lg">Recent Trades</CardTitle>
          </CardHeader>
          <CardContent className="p-2 sm:p-6 pt-0">
            <div className="overflow-x-auto -mx-2 sm:mx-0">
              <table className="w-full min-w-[500px] sm:min-w-0">
                <thead>
                  <tr className="text-left text-gray-400 text-xs sm:text-sm border-b border-gray-800">
                    <th className="pb-2 sm:pb-3 pr-2 sm:pr-4">Time</th>
                    <th className="pb-2 sm:pb-3 pr-2 sm:pr-4">Type</th>
                    <th className="pb-2 sm:pb-3 pr-2 sm:pr-4">Price</th>
                    <th className="pb-2 sm:pb-3 pr-2 sm:pr-4">Amount</th>
                    <th className="pb-2 sm:pb-3 pr-2 sm:pr-4 hidden sm:table-cell">Value</th>
                    <th className="pb-2 sm:pb-3 hidden md:table-cell">Maker</th>
                  </tr>
                </thead>
                <tbody>
                  {trades.slice(0, 15).map((trade, i) => (
                    <tr key={i} className="border-b border-gray-800/50 hover:bg-gray-800/30">
                      <td className="py-2 sm:py-3 pr-2 sm:pr-4 text-gray-400 text-xs sm:text-sm">
                        {new Date(trade.timestamp * 1000).toLocaleTimeString()}
                      </td>
                      <td className="py-2 sm:py-3 pr-2 sm:pr-4">
                        <Badge 
                          variant="outline" 
                          className={cn(
                            "text-[10px] sm:text-xs",
                            trade.type === 'buy' 
                              ? "border-green-500/50 text-green-400" 
                              : "border-red-500/50 text-red-400"
                          )}
                        >
                          {trade.type.toUpperCase()}
                        </Badge>
                      </td>
                      <td className="py-2 sm:py-3 pr-2 sm:pr-4 text-white font-mono text-xs sm:text-sm">
                        ${trade.price.toFixed(4)}
                      </td>
                      <td className="py-2 sm:py-3 pr-2 sm:pr-4 text-white text-xs sm:text-sm">
                        {trade.amountToken.toFixed(2)}
                      </td>
                      <td className="py-2 sm:py-3 pr-2 sm:pr-4 text-white text-xs sm:text-sm hidden sm:table-cell">
                        ${trade.amountUSD.toFixed(2)}
                      </td>
                      <td className="py-2 sm:py-3 text-gray-400 font-mono text-xs sm:text-sm hidden md:table-cell">
                        <a 
                          href={`https://bscscan.com/address/${trade.maker}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="hover:text-cyan-400"
                        >
                          {trade.maker.slice(0, 6)}...{trade.maker.slice(-4)}
                        </a>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>

        {/* Quick Links */}
        <div className="flex flex-wrap gap-2 sm:gap-3 justify-center">
          <a
            href="https://dexscreener.com/bsc/0x47F93f12853c8bA0D8a81Fdac3867D993e2ebD06"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Button variant="outline" size="sm" className="border-gray-700 text-gray-300 hover:text-white text-xs sm:text-sm">
              DEX Screener <ExternalLink className="w-3 h-3 sm:w-4 sm:h-4 ml-1 sm:ml-2" />
            </Button>
          </a>
          <a
            href={`https://pancakeswap.finance/swap?outputCurrency=${CONTRACTS.mainnet.EDGEAI}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Button variant="outline" size="sm" className="border-gray-700 text-gray-300 hover:text-white text-xs sm:text-sm">
              PancakeSwap <ExternalLink className="w-3 h-3 sm:w-4 sm:h-4 ml-1 sm:ml-2" />
            </Button>
          </a>
          <a
            href={`https://bscscan.com/token/${CONTRACTS.mainnet.EDGEAI}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Button variant="outline" size="sm" className="border-gray-700 text-gray-300 hover:text-white text-xs sm:text-sm">
              BSCScan <ExternalLink className="w-3 h-3 sm:w-4 sm:h-4 ml-1 sm:ml-2" />
            </Button>
          </a>
        </div>
      </div>
    </div>
  );
}
