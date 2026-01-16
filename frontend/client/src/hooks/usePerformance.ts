import { useEffect, useRef, useCallback } from 'react';

interface PerformanceMetrics {
  // Core Web Vitals
  lcp?: number; // Largest Contentful Paint
  fid?: number; // First Input Delay
  cls?: number; // Cumulative Layout Shift
  
  // Additional metrics
  fcp?: number; // First Contentful Paint
  ttfb?: number; // Time to First Byte
  
  // Custom metrics
  componentRenderTime?: number;
  apiResponseTime?: number;
}

// Global metrics store
const metrics: PerformanceMetrics = {};

/**
 * Hook to measure component render performance
 */
export function useRenderTime(componentName: string) {
  const startTime = useRef(performance.now());

  useEffect(() => {
    const renderTime = performance.now() - startTime.current;
    
    // Log slow renders (> 16ms = 60fps threshold)
    if (renderTime > 16) {
      console.warn(`[Performance] Slow render in ${componentName}: ${renderTime.toFixed(2)}ms`);
    }

    // Report to analytics (if available)
    if (typeof window !== 'undefined' && 'gtag' in window) {
      (window as any).gtag('event', 'render_time', {
        component: componentName,
        value: Math.round(renderTime),
      });
    }

    return () => {
      startTime.current = performance.now();
    };
  });
}

/**
 * Hook to measure API call performance
 */
export function useApiTiming() {
  const measure = useCallback(async <T>(
    name: string,
    apiCall: () => Promise<T>
  ): Promise<T> => {
    const start = performance.now();
    
    try {
      const result = await apiCall();
      const duration = performance.now() - start;
      
      // Log slow API calls (> 1 second)
      if (duration > 1000) {
        console.warn(`[Performance] Slow API call ${name}: ${duration.toFixed(2)}ms`);
      }

      // Store metric
      metrics.apiResponseTime = duration;

      return result;
    } catch (error) {
      const duration = performance.now() - start;
      console.error(`[Performance] API call ${name} failed after ${duration.toFixed(2)}ms`);
      throw error;
    }
  }, []);

  return { measure };
}

/**
 * Hook to observe Core Web Vitals
 */
export function useWebVitals(onReport?: (metrics: PerformanceMetrics) => void) {
  useEffect(() => {
    if (typeof window === 'undefined') return;

    // Observe LCP
    const lcpObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      const lastEntry = entries[entries.length - 1] as PerformanceEntry;
      metrics.lcp = lastEntry.startTime;
      onReport?.({ ...metrics });
    });

    try {
      lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });
    } catch (e) {
      // LCP not supported
    }

    // Observe FID
    const fidObserver = new PerformanceObserver((list) => {
      const entries = list.getEntries();
      const firstEntry = entries[0] as PerformanceEventTiming;
      metrics.fid = firstEntry.processingStart - firstEntry.startTime;
      onReport?.({ ...metrics });
    });

    try {
      fidObserver.observe({ type: 'first-input', buffered: true });
    } catch (e) {
      // FID not supported
    }

    // Observe CLS
    let clsValue = 0;
    const clsObserver = new PerformanceObserver((list) => {
      for (const entry of list.getEntries() as any[]) {
        if (!entry.hadRecentInput) {
          clsValue += entry.value;
        }
      }
      metrics.cls = clsValue;
      onReport?.({ ...metrics });
    });

    try {
      clsObserver.observe({ type: 'layout-shift', buffered: true });
    } catch (e) {
      // CLS not supported
    }

    // Get FCP from performance timeline
    const paintEntries = performance.getEntriesByType('paint');
    const fcpEntry = paintEntries.find(entry => entry.name === 'first-contentful-paint');
    if (fcpEntry) {
      metrics.fcp = fcpEntry.startTime;
    }

    // Get TTFB from navigation timing
    const navEntries = performance.getEntriesByType('navigation') as PerformanceNavigationTiming[];
    if (navEntries.length > 0) {
      metrics.ttfb = navEntries[0].responseStart - navEntries[0].requestStart;
    }

    return () => {
      lcpObserver.disconnect();
      fidObserver.disconnect();
      clsObserver.disconnect();
    };
  }, [onReport]);

  return metrics;
}

/**
 * Hook for intersection observer (lazy loading)
 */
export function useIntersectionObserver(
  options: IntersectionObserverInit = {}
) {
  const elementRef = useRef<HTMLElement | null>(null);
  const callbackRef = useRef<((isIntersecting: boolean) => void) | null>(null);

  const setElement = useCallback((element: HTMLElement | null) => {
    elementRef.current = element;
  }, []);

  const setCallback = useCallback((callback: (isIntersecting: boolean) => void) => {
    callbackRef.current = callback;
  }, []);

  useEffect(() => {
    const element = elementRef.current;
    if (!element) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          callbackRef.current?.(entry.isIntersecting);
        });
      },
      {
        threshold: 0.1,
        ...options,
      }
    );

    observer.observe(element);

    return () => {
      observer.disconnect();
    };
  }, [options]);

  return { setElement, setCallback };
}

/**
 * Debounce hook for performance optimization
 */
export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

// Need to import useState for useDebounce
import { useState } from 'react';

/**
 * Throttle hook for performance optimization
 */
export function useThrottle<T>(value: T, interval: number): T {
  const [throttledValue, setThrottledValue] = useState(value);
  const lastUpdated = useRef(Date.now());

  useEffect(() => {
    const now = Date.now();
    
    if (now - lastUpdated.current >= interval) {
      lastUpdated.current = now;
      setThrottledValue(value);
    } else {
      const timer = setTimeout(() => {
        lastUpdated.current = Date.now();
        setThrottledValue(value);
      }, interval - (now - lastUpdated.current));

      return () => clearTimeout(timer);
    }
  }, [value, interval]);

  return throttledValue;
}

/**
 * Get current performance metrics
 */
export function getPerformanceMetrics(): PerformanceMetrics {
  return { ...metrics };
}

export default {
  useRenderTime,
  useApiTiming,
  useWebVitals,
  useIntersectionObserver,
  useDebounce,
  useThrottle,
  getPerformanceMetrics,
};
