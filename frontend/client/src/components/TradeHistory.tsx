/**
 * Trade History and Statistics Components
 * Shows real-time trades and market statistics
 */

import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { 
  ArrowUpRight, 
  ArrowDownRight, 
  ExternalLink, 
  Users, 
  TrendingUp,
  Clock,
  DollarSign,
  Activity
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { TradeHistory as Trade, shortenAddress, formatUSD, formatTokenAmount } from '@/lib/pancakeswap';

interface TradeHistoryProps {
  trades: Trade[];
  className?: string;
}

export function TradeHistoryTable({ trades, className }: TradeHistoryProps) {
  const [filter, setFilter] = useState<'all' | 'buy' | 'sell'>('all');

  const filteredTrades = trades.filter(t => 
    filter === 'all' || t.type === filter
  );

  return (
    <Card className={cn('bg-gray-900/50 border-gray-800', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg font-semibold text-white">Recent Trades</CardTitle>
          <div className="flex gap-1">
            {(['all', 'buy', 'sell'] as const).map((f) => (
              <Button
                key={f}
                variant={filter === f ? 'default' : 'ghost'}
                size="sm"
                className={cn(
                  'h-7 px-2 text-xs capitalize',
                  filter === f ? 'bg-cyan-600' : 'text-gray-400'
                )}
                onClick={() => setFilter(f)}
              >
                {f}
              </Button>
            ))}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-1 max-h-[400px] overflow-y-auto">
          {/* Header */}
          <div className="grid grid-cols-5 gap-2 text-xs text-gray-500 pb-2 border-b border-gray-800">
            <span>Type</span>
            <span>Price</span>
            <span>Amount</span>
            <span>Total</span>
            <span>Time</span>
          </div>
          
          {/* Trades */}
          {filteredTrades.length === 0 ? (
            <div className="text-center text-gray-500 py-8">
              No trades found
            </div>
          ) : (
            filteredTrades.map((trade, i) => (
              <a
                key={i}
                href={`https://bscscan.com/tx/${trade.txHash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="grid grid-cols-5 gap-2 text-sm py-2 hover:bg-gray-800/50 rounded transition-colors"
              >
                <span className={cn(
                  'flex items-center gap-1',
                  trade.type === 'buy' ? 'text-green-400' : 'text-red-400'
                )}>
                  {trade.type === 'buy' ? (
                    <ArrowUpRight className="w-3 h-3" />
                  ) : (
                    <ArrowDownRight className="w-3 h-3" />
                  )}
                  {trade.type.toUpperCase()}
                </span>
                <span className="text-white">
                  ${trade.price.toFixed(4)}
                </span>
                <span className="text-gray-300">
                  {formatTokenAmount(trade.amountOut)}
                </span>
                <span className="text-gray-300">
                  {formatTokenAmount(trade.amountIn)} {trade.tokenIn}
                </span>
                <span className="text-gray-500 flex items-center gap-1">
                  {formatTimeAgo(trade.timestamp)}
                  <ExternalLink className="w-3 h-3 opacity-50" />
                </span>
              </a>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// Market Statistics Card
interface MarketStatsProps {
  priceUSD: number;
  priceChange24h: number;
  volume24h: number;
  liquidity: number;
  marketCap: number;
  fdv: number;
  txns24h: { buys: number; sells: number };
  holders: number;
  className?: string;
}

export function MarketStats({
  priceUSD,
  priceChange24h,
  volume24h,
  liquidity,
  marketCap,
  fdv,
  txns24h,
  holders,
  className,
}: MarketStatsProps) {
  return (
    <Card className={cn('bg-gray-900/50 border-gray-800', className)}>
      <CardContent className="p-4">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {/* Price */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500">Price USD</span>
            <div className="flex items-center gap-2">
              <span className="text-xl font-bold text-white">
                ${priceUSD.toFixed(4)}
              </span>
              <Badge 
                variant="outline" 
                className={cn(
                  'text-xs',
                  priceChange24h >= 0 
                    ? 'border-green-500/50 text-green-400' 
                    : 'border-red-500/50 text-red-400'
                )}
              >
                {priceChange24h >= 0 ? '+' : ''}{priceChange24h.toFixed(2)}%
              </Badge>
            </div>
          </div>

          {/* Volume */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <Activity className="w-3 h-3" />
              24h Volume
            </span>
            <span className="text-lg font-semibold text-white">
              {formatUSD(volume24h)}
            </span>
          </div>

          {/* Liquidity */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <DollarSign className="w-3 h-3" />
              Liquidity
            </span>
            <span className="text-lg font-semibold text-white">
              {formatUSD(liquidity)}
            </span>
          </div>

          {/* Market Cap */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <TrendingUp className="w-3 h-3" />
              Market Cap
            </span>
            <span className="text-lg font-semibold text-white">
              {formatUSD(marketCap)}
            </span>
          </div>

          {/* FDV */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500">FDV</span>
            <span className="text-lg font-semibold text-white">
              {formatUSD(fdv)}
            </span>
          </div>

          {/* Transactions */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500">24h Txns</span>
            <div className="flex items-center gap-2">
              <span className="text-green-400">{txns24h.buys} buys</span>
              <span className="text-gray-600">/</span>
              <span className="text-red-400">{txns24h.sells} sells</span>
            </div>
          </div>

          {/* Holders */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <Users className="w-3 h-3" />
              Holders
            </span>
            <span className="text-lg font-semibold text-white">
              {holders.toLocaleString()}
            </span>
          </div>

          {/* Buy/Sell Ratio */}
          <div className="space-y-1">
            <span className="text-xs text-gray-500">Buy/Sell Ratio</span>
            <div className="flex items-center gap-2">
              <div className="flex-1 h-2 bg-gray-800 rounded-full overflow-hidden">
                <div 
                  className="h-full bg-green-500"
                  style={{ 
                    width: `${(txns24h.buys / (txns24h.buys + txns24h.sells)) * 100}%` 
                  }}
                />
              </div>
              <span className="text-xs text-gray-400">
                {((txns24h.buys / (txns24h.buys + txns24h.sells)) * 100).toFixed(0)}%
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// Top Traders Leaderboard
interface TopTrader {
  address: string;
  volume: number;
  trades: number;
  pnl: number;
  pnlPercent: number;
}

export function TopTraders({ 
  traders, 
  className 
}: { 
  traders: TopTrader[]; 
  className?: string;
}) {
  return (
    <Card className={cn('bg-gray-900/50 border-gray-800', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg font-semibold text-white flex items-center gap-2">
          <Users className="w-4 h-4 text-cyan-400" />
          Top Traders
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {traders.map((trader, i) => (
            <a
              key={i}
              href={`https://bscscan.com/address/${trader.address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center justify-between p-2 hover:bg-gray-800/50 rounded transition-colors"
            >
              <div className="flex items-center gap-3">
                <span className={cn(
                  'w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold',
                  i === 0 ? 'bg-yellow-500/20 text-yellow-400' :
                  i === 1 ? 'bg-gray-400/20 text-gray-300' :
                  i === 2 ? 'bg-orange-500/20 text-orange-400' :
                  'bg-gray-800 text-gray-500'
                )}>
                  {i + 1}
                </span>
                <div>
                  <span className="text-sm text-white font-mono">
                    {shortenAddress(trader.address)}
                  </span>
                  <span className="text-xs text-gray-500 ml-2">
                    {trader.trades} trades
                  </span>
                </div>
              </div>
              <div className="text-right">
                <div className="text-sm text-white">
                  {formatUSD(trader.volume)}
                </div>
                <div className={cn(
                  'text-xs',
                  trader.pnl >= 0 ? 'text-green-400' : 'text-red-400'
                )}>
                  {trader.pnl >= 0 ? '+' : ''}{formatUSD(trader.pnl)} ({trader.pnlPercent.toFixed(1)}%)
                </div>
              </div>
            </a>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// Token Holders List
interface Holder {
  address: string;
  balance: string;
  percentage: number;
}

export function HoldersList({ 
  holders, 
  className 
}: { 
  holders: Holder[]; 
  className?: string;
}) {
  return (
    <Card className={cn('bg-gray-900/50 border-gray-800', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg font-semibold text-white">
          Top Holders
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {holders.map((holder, i) => (
            <a
              key={i}
              href={`https://bscscan.com/address/${holder.address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center justify-between p-2 hover:bg-gray-800/50 rounded transition-colors"
            >
              <div className="flex items-center gap-3">
                <span className="text-sm text-gray-500">#{i + 1}</span>
                <span className="text-sm text-white font-mono">
                  {shortenAddress(holder.address)}
                </span>
              </div>
              <div className="text-right">
                <div className="text-sm text-white">
                  {holder.balance}
                </div>
                <div className="text-xs text-gray-500">
                  {holder.percentage.toFixed(2)}%
                </div>
              </div>
            </a>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

// Price Change Badges
export function PriceChangeBadges({
  changes,
}: {
  changes: { m5: number; h1: number; h6: number; h24: number };
}) {
  const items = [
    { label: '5M', value: changes.m5 },
    { label: '1H', value: changes.h1 },
    { label: '6H', value: changes.h6 },
    { label: '24H', value: changes.h24 },
  ];

  return (
    <div className="flex items-center gap-2">
      {items.map((item) => (
        <div key={item.label} className="text-center">
          <span className="text-xs text-gray-500 block">{item.label}</span>
          <span className={cn(
            'text-sm font-medium',
            item.value >= 0 ? 'text-green-400' : 'text-red-400'
          )}>
            {item.value >= 0 ? '+' : ''}{item.value.toFixed(2)}%
          </span>
        </div>
      ))}
    </div>
  );
}

// Helper function
function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - timestamp);
  
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}
