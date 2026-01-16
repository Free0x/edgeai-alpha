/**
 * Swap Trading Panel Component
 * Supports real BSC trading via PancakeSwap
 */

import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Slider } from '@/components/ui/slider';
import { ArrowDownUp, Settings, Info, Loader2, ExternalLink, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useWallet } from '@/contexts/WalletContext';
import { 
  getSwapQuote, 
  executeSwap, 
  getTokenBalance,
  CONTRACTS,
  formatTokenAmount,
  formatUSD,
  SwapQuote 
} from '@/lib/pancakeswap';
import { ethers } from 'ethers';
import { toast } from 'sonner';

interface Token {
  symbol: string;
  name: string;
  address: string;
  decimals: number;
  logo?: string;
  balance?: string;
}

interface SwapPanelProps {
  currentPrice: number;
  priceUSD: number;
  className?: string;
}

const TOKENS: Token[] = [
  {
    symbol: 'EDGEAI',
    name: 'EdgeAI',
    address: CONTRACTS.mainnet.EDGEAI,
    decimals: 18,
    logo: '/edgeai-logo.png',
  },
  {
    symbol: 'BNB',
    name: 'BNB',
    address: CONTRACTS.mainnet.WBNB,
    decimals: 18,
    logo: '/bnb-logo.png',
  },
];

export function SwapPanel({ currentPrice, priceUSD, className }: SwapPanelProps) {
  const { metamaskAddress, metamaskConnected, connectMetaMask } = useWallet();
  
  const [tokenIn, setTokenIn] = useState<Token>(TOKENS[1]); // BNB
  const [tokenOut, setTokenOut] = useState<Token>(TOKENS[0]); // EDGEAI
  const [amountIn, setAmountIn] = useState('');
  const [amountOut, setAmountOut] = useState('');
  const [slippage, setSlippage] = useState(0.5);
  const [showSettings, setShowSettings] = useState(false);
  const [quote, setQuote] = useState<SwapQuote | null>(null);
  const [loading, setLoading] = useState(false);
  const [swapping, setSwapping] = useState(false);
  const [balanceIn, setBalanceIn] = useState('0');
  const [balanceOut, setBalanceOut] = useState('0');

  // Fetch balances when wallet connected
  useEffect(() => {
    const fetchBalances = async () => {
      if (!metamaskAddress) return;
      
      try {
        const [balIn, balOut] = await Promise.all([
          getTokenBalance(tokenIn.address, metamaskAddress),
          getTokenBalance(tokenOut.address, metamaskAddress),
        ]);
        setBalanceIn(balIn);
        setBalanceOut(balOut);
      } catch (e) {
        console.error('Failed to fetch balances:', e);
      }
    };
    
    fetchBalances();
    const interval = setInterval(fetchBalances, 30000);
    return () => clearInterval(interval);
  }, [metamaskAddress, tokenIn, tokenOut]);

  // Get quote when amount changes
  useEffect(() => {
    const getQuote = async () => {
      if (!amountIn || parseFloat(amountIn) <= 0) {
        setQuote(null);
        setAmountOut('');
        return;
      }

      setLoading(true);
      try {
        const result = await getSwapQuote(
          amountIn,
          tokenIn.address,
          tokenOut.address,
          slippage
        );
        
        if (result) {
          setQuote(result);
          setAmountOut(result.amountOut);
        }
      } catch (e) {
        console.error('Failed to get quote:', e);
      } finally {
        setLoading(false);
      }
    };

    const debounce = setTimeout(getQuote, 500);
    return () => clearTimeout(debounce);
  }, [amountIn, tokenIn, tokenOut, slippage]);

  // Swap tokens
  const handleSwapTokens = useCallback(() => {
    setTokenIn(tokenOut);
    setTokenOut(tokenIn);
    setAmountIn(amountOut);
    setAmountOut(amountIn);
    setBalanceIn(balanceOut);
    setBalanceOut(balanceIn);
  }, [tokenIn, tokenOut, amountIn, amountOut, balanceIn, balanceOut]);

  // Execute swap
  const handleSwap = async () => {
    if (!metamaskConnected || !quote) {
      toast.error('Please connect your wallet first');
      return;
    }

    if (!window.ethereum) {
      toast.error('MetaMask not found');
      return;
    }

    setSwapping(true);
    try {
      const provider = new ethers.BrowserProvider(window.ethereum);
      const signer = await provider.getSigner();
      
      const tx = await executeSwap(
        signer,
        amountIn,
        quote.minimumReceived,
        quote.path,
        20 // 20 minutes deadline
      );

      if (tx) {
        toast.success(
          <div className="flex flex-col gap-1">
            <span>Swap submitted!</span>
            <a 
              href={`https://bscscan.com/tx/${tx.hash}`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-cyan-400 text-sm flex items-center gap-1"
            >
              View on BSCScan <ExternalLink className="w-3 h-3" />
            </a>
          </div>
        );

        // Wait for confirmation
        await tx.wait();
        toast.success('Swap confirmed!');
        
        // Reset form
        setAmountIn('');
        setAmountOut('');
        setQuote(null);
        
        // Refresh balances
        const [balIn, balOut] = await Promise.all([
          getTokenBalance(tokenIn.address, metamaskAddress!),
          getTokenBalance(tokenOut.address, metamaskAddress!),
        ]);
        setBalanceIn(balIn);
        setBalanceOut(balOut);
      }
    } catch (e: any) {
      console.error('Swap failed:', e);
      toast.error(e.message || 'Swap failed');
    } finally {
      setSwapping(false);
    }
  };

  // Set max amount
  const handleSetMax = () => {
    setAmountIn(balanceIn);
  };

  // Percentage buttons
  const handlePercentage = (pct: number) => {
    const amount = (parseFloat(balanceIn) * pct / 100).toFixed(6);
    setAmountIn(amount);
  };

  return (
    <Card className={cn('bg-gray-900/50 border-gray-800', className)}>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-lg font-semibold text-white">Swap</CardTitle>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-gray-400 hover:text-white"
          onClick={() => setShowSettings(!showSettings)}
        >
          <Settings className="h-4 w-4" />
        </Button>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Settings panel */}
        {showSettings && (
          <div className="p-3 bg-gray-800/50 rounded-lg space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-400">Slippage Tolerance</span>
              <span className="text-sm text-white">{slippage}%</span>
            </div>
            <div className="flex gap-2">
              {[0.1, 0.5, 1.0].map((s) => (
                <Button
                  key={s}
                  variant={slippage === s ? 'default' : 'outline'}
                  size="sm"
                  className={cn(
                    'flex-1',
                    slippage === s ? 'bg-cyan-600' : 'border-gray-700'
                  )}
                  onClick={() => setSlippage(s)}
                >
                  {s}%
                </Button>
              ))}
              <Input
                type="number"
                value={slippage}
                onChange={(e) => setSlippage(parseFloat(e.target.value) || 0.5)}
                className="w-20 h-8 bg-gray-900 border-gray-700 text-center"
              />
            </div>
          </div>
        )}

        {/* Token In */}
        <div className="p-4 bg-gray-800/50 rounded-lg space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">You Pay</span>
            <span className="text-xs text-gray-500">
              Balance: {formatTokenAmount(balanceIn)} {tokenIn.symbol}
            </span>
          </div>
          <div className="flex items-center gap-3">
            <Input
              type="number"
              placeholder="0.0"
              value={amountIn}
              onChange={(e) => setAmountIn(e.target.value)}
              className="flex-1 text-2xl font-medium bg-transparent border-none focus-visible:ring-0 p-0 h-auto"
            />
            <Button
              variant="outline"
              className="h-10 px-3 bg-gray-700/50 border-gray-600 hover:bg-gray-700"
            >
              <span className="font-medium">{tokenIn.symbol}</span>
            </Button>
          </div>
          <div className="flex items-center gap-2">
            {[25, 50, 75, 100].map((pct) => (
              <Button
                key={pct}
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs text-gray-400 hover:text-white"
                onClick={() => handlePercentage(pct)}
              >
                {pct}%
              </Button>
            ))}
          </div>
        </div>

        {/* Swap button */}
        <div className="flex justify-center -my-2">
          <Button
            variant="ghost"
            size="icon"
            className="h-10 w-10 rounded-full bg-gray-800 border border-gray-700 hover:bg-gray-700"
            onClick={handleSwapTokens}
          >
            <ArrowDownUp className="h-4 w-4" />
          </Button>
        </div>

        {/* Token Out */}
        <div className="p-4 bg-gray-800/50 rounded-lg space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">You Receive</span>
            <span className="text-xs text-gray-500">
              Balance: {formatTokenAmount(balanceOut)} {tokenOut.symbol}
            </span>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex-1">
              {loading ? (
                <Loader2 className="h-6 w-6 animate-spin text-gray-400" />
              ) : (
                <span className="text-2xl font-medium text-white">
                  {amountOut || '0.0'}
                </span>
              )}
            </div>
            <Button
              variant="outline"
              className="h-10 px-3 bg-gray-700/50 border-gray-600 hover:bg-gray-700"
            >
              <span className="font-medium">{tokenOut.symbol}</span>
            </Button>
          </div>
          {amountOut && priceUSD > 0 && (
            <span className="text-xs text-gray-500">
              ≈ {formatUSD(parseFloat(amountOut) * priceUSD)}
            </span>
          )}
        </div>

        {/* Quote details */}
        {quote && (
          <div className="p-3 bg-gray-800/30 rounded-lg space-y-2 text-sm">
            <div className="flex items-center justify-between text-gray-400">
              <span>Rate</span>
              <span>
                1 {tokenIn.symbol} = {(parseFloat(quote.amountOut) / parseFloat(quote.amountIn)).toFixed(4)} {tokenOut.symbol}
              </span>
            </div>
            <div className="flex items-center justify-between text-gray-400">
              <span className="flex items-center gap-1">
                Price Impact
                <Info className="h-3 w-3" />
              </span>
              <span className={cn(
                quote.priceImpact > 3 ? 'text-red-400' : 
                quote.priceImpact > 1 ? 'text-yellow-400' : 'text-green-400'
              )}>
                {quote.priceImpact.toFixed(2)}%
              </span>
            </div>
            <div className="flex items-center justify-between text-gray-400">
              <span>Minimum Received</span>
              <span>{quote.minimumReceived} {tokenOut.symbol}</span>
            </div>
            <div className="flex items-center justify-between text-gray-400">
              <span>Slippage</span>
              <span>{slippage}%</span>
            </div>
          </div>
        )}

        {/* Price impact warning */}
        {quote && quote.priceImpact > 5 && (
          <div className="flex items-center gap-2 p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
            <AlertTriangle className="h-4 w-4 text-red-400" />
            <span className="text-sm text-red-400">
              High price impact! Your trade will move the market significantly.
            </span>
          </div>
        )}

        {/* Action button */}
        {!metamaskConnected ? (
          <Button
            className="w-full h-12 bg-cyan-600 hover:bg-cyan-700 text-white font-medium"
            onClick={connectMetaMask}
          >
            Connect Wallet
          </Button>
        ) : !amountIn || parseFloat(amountIn) <= 0 ? (
          <Button
            className="w-full h-12 bg-gray-700 text-gray-400 cursor-not-allowed"
            disabled
          >
            Enter Amount
          </Button>
        ) : parseFloat(amountIn) > parseFloat(balanceIn) ? (
          <Button
            className="w-full h-12 bg-red-600/20 text-red-400 cursor-not-allowed"
            disabled
          >
            Insufficient {tokenIn.symbol} Balance
          </Button>
        ) : (
          <Button
            className="w-full h-12 bg-cyan-600 hover:bg-cyan-700 text-white font-medium"
            onClick={handleSwap}
            disabled={swapping || !quote}
          >
            {swapping ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Swapping...
              </>
            ) : (
              `Swap ${tokenIn.symbol} for ${tokenOut.symbol}`
            )}
          </Button>
        )}

        {/* Powered by */}
        <div className="flex items-center justify-center gap-2 text-xs text-gray-500">
          <span>Powered by</span>
          <a 
            href="https://pancakeswap.finance" 
            target="_blank" 
            rel="noopener noreferrer"
            className="text-cyan-400 hover:underline"
          >
            PancakeSwap
          </a>
        </div>
      </CardContent>
    </Card>
  );
}

// Quick buy/sell buttons
export function QuickTradeButtons({ 
  onBuy, 
  onSell,
  disabled = false 
}: { 
  onBuy: () => void; 
  onSell: () => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex gap-2">
      <Button
        className="flex-1 h-12 bg-green-600 hover:bg-green-700 text-white font-medium"
        onClick={onBuy}
        disabled={disabled}
      >
        Buy EDGEAI
      </Button>
      <Button
        className="flex-1 h-12 bg-red-600 hover:bg-red-700 text-white font-medium"
        onClick={onSell}
        disabled={disabled}
      >
        Sell EDGEAI
      </Button>
    </div>
  );
}
