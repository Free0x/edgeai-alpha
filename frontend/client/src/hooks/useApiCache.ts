import { useState, useEffect, useCallback, useRef } from 'react';

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  ttl: number;
}

// Global cache store
const cache = new Map<string, CacheEntry<unknown>>();

// Cache configuration
const DEFAULT_TTL = 30000; // 30 seconds
const STALE_WHILE_REVALIDATE = 60000; // 1 minute

/**
 * Custom hook for API requests with caching
 * Implements stale-while-revalidate pattern
 */
export function useApiCache<T>(
  key: string,
  fetcher: () => Promise<T>,
  options: {
    ttl?: number;
    enabled?: boolean;
    refetchInterval?: number;
    onSuccess?: (data: T) => void;
    onError?: (error: Error) => void;
  } = {}
) {
  const {
    ttl = DEFAULT_TTL,
    enabled = true,
    refetchInterval,
    onSuccess,
    onError,
  } = options;

  const [data, setData] = useState<T | undefined>(() => {
    const cached = cache.get(key) as CacheEntry<T> | undefined;
    if (cached && Date.now() - cached.timestamp < STALE_WHILE_REVALIDATE) {
      return cached.data;
    }
    return undefined;
  });
  const [isLoading, setIsLoading] = useState(!data);
  const [error, setError] = useState<Error | null>(null);
  const [isStale, setIsStale] = useState(false);

  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  const fetchData = useCallback(async (isBackground = false) => {
    if (!enabled) return;

    // Check cache first
    const cached = cache.get(key) as CacheEntry<T> | undefined;
    const now = Date.now();

    if (cached) {
      const age = now - cached.timestamp;
      
      // Fresh cache - use it directly
      if (age < cached.ttl) {
        if (!isBackground) {
          setData(cached.data);
          setIsLoading(false);
          setIsStale(false);
        }
        return;
      }
      
      // Stale but usable - show stale data while revalidating
      if (age < STALE_WHILE_REVALIDATE) {
        if (!isBackground) {
          setData(cached.data);
          setIsStale(true);
        }
      }
    }

    if (!isBackground) {
      setIsLoading(true);
    }

    try {
      const result = await fetcherRef.current();
      
      // Update cache
      cache.set(key, {
        data: result,
        timestamp: Date.now(),
        ttl,
      });

      setData(result);
      setError(null);
      setIsStale(false);
      onSuccess?.(result);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      setError(error);
      onError?.(error);
    } finally {
      setIsLoading(false);
    }
  }, [key, ttl, enabled, onSuccess, onError]);

  // Initial fetch
  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Refetch interval
  useEffect(() => {
    if (!refetchInterval || !enabled) return;

    const interval = setInterval(() => {
      fetchData(true); // Background fetch
    }, refetchInterval);

    return () => clearInterval(interval);
  }, [refetchInterval, enabled, fetchData]);

  // Manual refetch
  const refetch = useCallback(() => {
    // Invalidate cache
    cache.delete(key);
    return fetchData();
  }, [key, fetchData]);

  // Mutate cache directly
  const mutate = useCallback((newData: T | ((prev: T | undefined) => T)) => {
    const updatedData = typeof newData === 'function' 
      ? (newData as (prev: T | undefined) => T)(data)
      : newData;
    
    setData(updatedData);
    cache.set(key, {
      data: updatedData,
      timestamp: Date.now(),
      ttl,
    });
  }, [key, data, ttl]);

  return {
    data,
    isLoading,
    error,
    isStale,
    refetch,
    mutate,
  };
}

/**
 * Prefetch data into cache
 */
export async function prefetch<T>(key: string, fetcher: () => Promise<T>, ttl = DEFAULT_TTL) {
  try {
    const data = await fetcher();
    cache.set(key, {
      data,
      timestamp: Date.now(),
      ttl,
    });
    return data;
  } catch (error) {
    console.error(`Prefetch failed for ${key}:`, error);
    throw error;
  }
}

/**
 * Invalidate cache entries
 */
export function invalidateCache(keyOrPattern: string | RegExp) {
  if (typeof keyOrPattern === 'string') {
    cache.delete(keyOrPattern);
  } else {
    for (const key of cache.keys()) {
      if (keyOrPattern.test(key)) {
        cache.delete(key);
      }
    }
  }
}

/**
 * Clear all cache
 */
export function clearCache() {
  cache.clear();
}

/**
 * Get cache statistics
 */
export function getCacheStats() {
  const now = Date.now();
  let fresh = 0;
  let stale = 0;
  let expired = 0;

  for (const entry of cache.values()) {
    const age = now - entry.timestamp;
    if (age < entry.ttl) {
      fresh++;
    } else if (age < STALE_WHILE_REVALIDATE) {
      stale++;
    } else {
      expired++;
    }
  }

  return {
    total: cache.size,
    fresh,
    stale,
    expired,
  };
}

export default useApiCache;
