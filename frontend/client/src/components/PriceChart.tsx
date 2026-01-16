/**
 * Enhanced Price Chart Component
 * Uses Lightweight Charts for professional trading view
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { createChart, IChartApi, ISeriesApi, CandlestickData, Time, LineData, HistogramData, ColorType } from 'lightweight-charts';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

interface PriceChartProps {
  data: CandlestickData<Time>[];
  volumeData?: HistogramData<Time>[];
  currentPrice?: number;
  priceChange24h?: number;
  onTimeframeChange?: (tf: string) => void;
  className?: string;
  height?: number;
}

const TIMEFRAMES = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '1H', value: '1h' },
  { label: '4H', value: '4h' },
  { label: '1D', value: '1d' },
];

export function PriceChart({
  data,
  volumeData,
  currentPrice,
  priceChange24h = 0,
  onTimeframeChange,
  className,
  height = 400,
}: PriceChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candlestickSeriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null);
  const volumeSeriesRef = useRef<ISeriesApi<'Histogram'> | null>(null);
  const [selectedTimeframe, setSelectedTimeframe] = useState('1h');
  const [chartType, setChartType] = useState<'candle' | 'line'>('candle');

  // Initialize chart
  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: '#9ca3af',
      },
      grid: {
        vertLines: { color: 'rgba(42, 46, 57, 0.3)' },
        horzLines: { color: 'rgba(42, 46, 57, 0.3)' },
      },
      crosshair: {
        mode: 1,
        vertLine: {
          width: 1,
          color: 'rgba(224, 227, 235, 0.3)',
          style: 0,
        },
        horzLine: {
          width: 1,
          color: 'rgba(224, 227, 235, 0.3)',
          style: 0,
        },
      },
      rightPriceScale: {
        borderColor: 'rgba(42, 46, 57, 0.5)',
        scaleMargins: {
          top: 0.1,
          bottom: 0.2,
        },
      },
      timeScale: {
        borderColor: 'rgba(42, 46, 57, 0.5)',
        timeVisible: true,
        secondsVisible: false,
      },
      width: chartContainerRef.current.clientWidth,
      height: height,
    });

    // Add candlestick series
    const candlestickSeries = chart.addCandlestickSeries({
      upColor: '#22c55e',
      downColor: '#ef4444',
      borderDownColor: '#ef4444',
      borderUpColor: '#22c55e',
      wickDownColor: '#ef4444',
      wickUpColor: '#22c55e',
    });

    // Add volume series
    const volumeSeries = chart.addHistogramSeries({
      color: '#26a69a',
      priceFormat: {
        type: 'volume',
      },
      priceScaleId: '',
    });

    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.8,
        bottom: 0,
      },
    });

    chartRef.current = chart;
    candlestickSeriesRef.current = candlestickSeries;
    volumeSeriesRef.current = volumeSeries;

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({ width: chartContainerRef.current.clientWidth });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, [height]);

  // Update data
  useEffect(() => {
    if (candlestickSeriesRef.current && data.length > 0) {
      candlestickSeriesRef.current.setData(data);
    }
  }, [data]);

  // Update volume data
  useEffect(() => {
    if (volumeSeriesRef.current && volumeData && volumeData.length > 0) {
      volumeSeriesRef.current.setData(volumeData);
    }
  }, [volumeData]);

  // Handle timeframe change
  const handleTimeframeChange = useCallback((tf: string) => {
    setSelectedTimeframe(tf);
    onTimeframeChange?.(tf);
  }, [onTimeframeChange]);

  return (
    <div className={cn('relative', className)}>
      {/* Header with price info */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-4">
          <div>
            <span className="text-2xl font-bold text-white">
              ${currentPrice?.toFixed(4) || '0.00'}
            </span>
            <span className={cn(
              'ml-2 text-sm font-medium',
              priceChange24h >= 0 ? 'text-green-500' : 'text-red-500'
            )}>
              {priceChange24h >= 0 ? '+' : ''}{priceChange24h.toFixed(2)}%
            </span>
          </div>
        </div>

        {/* Timeframe selector */}
        <div className="flex items-center gap-1">
          {TIMEFRAMES.map((tf) => (
            <Button
              key={tf.value}
              variant={selectedTimeframe === tf.value ? 'default' : 'ghost'}
              size="sm"
              className={cn(
                'h-7 px-2 text-xs',
                selectedTimeframe === tf.value
                  ? 'bg-cyan-600 hover:bg-cyan-700'
                  : 'text-gray-400 hover:text-white'
              )}
              onClick={() => handleTimeframeChange(tf.value)}
            >
              {tf.label}
            </Button>
          ))}
          <div className="w-px h-5 bg-gray-700 mx-2" />
          <Button
            variant={chartType === 'candle' ? 'default' : 'ghost'}
            size="sm"
            className="h-7 px-2"
            onClick={() => setChartType('candle')}
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
              <rect x="3" y="8" width="4" height="8" rx="1" />
              <rect x="10" y="4" width="4" height="16" rx="1" />
              <rect x="17" y="6" width="4" height="12" rx="1" />
            </svg>
          </Button>
          <Button
            variant={chartType === 'line' ? 'default' : 'ghost'}
            size="sm"
            className="h-7 px-2"
            onClick={() => setChartType('line')}
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="22 12 18 8 12 14 8 10 2 16" />
            </svg>
          </Button>
        </div>
      </div>

      {/* Chart container */}
      <div 
        ref={chartContainerRef} 
        className="w-full rounded-lg overflow-hidden"
        style={{ height: `${height}px` }}
      />

      {/* Legend */}
      <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
        <span className="flex items-center gap-1">
          <span className="w-3 h-3 bg-green-500 rounded-sm" />
          Bullish
        </span>
        <span className="flex items-center gap-1">
          <span className="w-3 h-3 bg-red-500 rounded-sm" />
          Bearish
        </span>
        <span className="flex items-center gap-1">
          <span className="w-3 h-3 bg-cyan-500/50 rounded-sm" />
          Volume
        </span>
      </div>
    </div>
  );
}

// Mini chart for overview
export function MiniPriceChart({
  data,
  positive = true,
  className,
}: {
  data: { time: Time; value: number }[];
  positive?: boolean;
  className?: string;
}) {
  const chartContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!chartContainerRef.current || data.length === 0) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: 'transparent',
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { visible: false },
      },
      rightPriceScale: { visible: false },
      timeScale: { visible: false },
      crosshair: { mode: 0 },
      handleScroll: false,
      handleScale: false,
      width: chartContainerRef.current.clientWidth,
      height: 40,
    });

    const lineSeries = chart.addAreaSeries({
      lineColor: positive ? '#22c55e' : '#ef4444',
      topColor: positive ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)',
      bottomColor: positive ? 'rgba(34, 197, 94, 0)' : 'rgba(239, 68, 68, 0)',
      lineWidth: 2,
    });

    lineSeries.setData(data);
    chart.timeScale().fitContent();

    return () => chart.remove();
  }, [data, positive]);

  return <div ref={chartContainerRef} className={cn('w-full h-10', className)} />;
}
