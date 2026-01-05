import { injectable, inject } from 'tsyringe';
import type { ICacheService, IConfigService } from '../types';
import { TOKENS } from '../container';

interface CacheEntry<T> {
  value: T;
  expiresAt: number;
}

@injectable()
export class CacheService implements ICacheService {
  private cache = new Map<string, CacheEntry<unknown>>();
  private defaultTtl: number;

  constructor(
    @inject(TOKENS.Config) private configService: IConfigService
  ) {
    this.defaultTtl = this.configService.get<number>('cache.ttl') || 3600;

    // Cleanup expired entries periodically
    setInterval(() => this.cleanup(), 60000);
  }

  async get<T>(key: string): Promise<T | null> {
    const entry = this.cache.get(key) as CacheEntry<T> | undefined;

    if (!entry) {
      return null;
    }

    if (Date.now() > entry.expiresAt) {
      this.cache.delete(key);
      return null;
    }

    return entry.value;
  }

  async set<T>(key: string, value: T, ttlSeconds?: number): Promise<void> {
    const ttl = ttlSeconds ?? this.defaultTtl;
    const expiresAt = Date.now() + ttl * 1000;

    this.cache.set(key, { value, expiresAt });
  }

  async delete(key: string): Promise<void> {
    this.cache.delete(key);
  }

  async clear(): Promise<void> {
    this.cache.clear();
  }

  private cleanup(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now > entry.expiresAt) {
        this.cache.delete(key);
      }
    }
  }
}
