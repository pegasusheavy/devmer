import { injectable, inject } from 'tsyringe';
import { PrismaClient } from '@prisma/client';
import { PrismaPg } from '@prisma/adapter-pg';
import { Pool, PoolClient } from 'pg';
import type { IConfigService } from '../types';
import { TOKENS } from '../container';

/**
 * Prisma service for database access with full DI integration
 * 
 * @example
 * ```typescript
 * @injectable()
 * class MyService {
 *   constructor(@inject(TOKENS.Prisma) private prisma: PrismaService) {}
 *   
 *   async getUsers() {
 *     return this.prisma.client.user.findMany();
 *   }
 * }
 * ```
 */
@injectable()
export class PrismaService {
  private _client: PrismaClient | null = null;
  private _pool: Pool | null = null;
  private _connected = false;

  constructor(
    @inject(TOKENS.Config) private config: IConfigService
  ) {}

  /**
   * Get the Prisma client instance
   * @throws Error if not connected
   */
  get client(): PrismaClient {
    if (!this._client) {
      throw new Error('Prisma not connected. Call connect() first.');
    }
    return this._client;
  }

  /**
   * Get the underlying PostgreSQL pool
   * Useful for raw queries or transactions
   */
  get pool(): Pool {
    if (!this._pool) {
      throw new Error('Prisma not connected. Call connect() first.');
    }
    return this._pool;
  }

  /**
   * Check if connected to database
   */
  get isConnected(): boolean {
    return this._connected;
  }

  /**
   * Connect to the database
   */
  async connect(): Promise<void> {
    if (this._connected) {
      return;
    }

    const databaseUrl = this.config.get<string>('database.url');
    if (!databaseUrl) {
      throw new Error('DATABASE_URL is not configured');
    }

    // Create PostgreSQL connection pool
    this._pool = new Pool({
      connectionString: databaseUrl,
      max: this.config.get<number>('database.poolMax') || 10,
      idleTimeoutMillis: this.config.get<number>('database.idleTimeout') || 30000,
      connectionTimeoutMillis: this.config.get<number>('database.connectionTimeout') || 5000,
    });

    // Create Prisma adapter
    const adapter = new PrismaPg(this._pool);

    // Create Prisma client with adapter
    this._client = new PrismaClient({
      adapter,
      log: this.config.isDevelopment()
        ? ['query', 'info', 'warn', 'error']
        : ['error'],
    });

    // Test connection
    await this._client.$connect();
    this._connected = true;

    console.log('✅ Prisma connected to database');
  }

  /**
   * Disconnect from the database
   */
  async disconnect(): Promise<void> {
    if (!this._connected) {
      return;
    }

    await this._client?.$disconnect();
    await this._pool?.end();

    this._client = null;
    this._pool = null;
    this._connected = false;

    console.log('📤 Prisma disconnected from database');
  }

  /**
   * Execute a function within a transaction
   * 
   * @example
   * ```typescript
   * await prisma.transaction(async (tx) => {
   *   await tx.user.create({ data: { ... } });
   *   await tx.post.create({ data: { ... } });
   * });
   * ```
   */
  async transaction<T>(
    fn: (tx: Omit<PrismaClient, '$connect' | '$disconnect' | '$on' | '$transaction' | '$use' | '$extends'>) => Promise<T>,
    options?: { maxWait?: number; timeout?: number; isolationLevel?: 'ReadUncommitted' | 'ReadCommitted' | 'RepeatableRead' | 'Serializable' }
  ): Promise<T> {
    return this.client.$transaction(fn, options);
  }

  /**
   * Execute a raw SQL query
   * 
   * @example
   * ```typescript
   * const users = await prisma.rawQuery<User[]>`SELECT * FROM users WHERE active = true`;
   * ```
   */
  async rawQuery<T>(query: TemplateStringsArray, ...values: unknown[]): Promise<T> {
    return this.client.$queryRaw<T>(query, ...values);
  }

  /**
   * Execute a raw SQL command (INSERT, UPDATE, DELETE)
   * 
   * @example
   * ```typescript
   * const count = await prisma.rawExecute`UPDATE users SET active = false WHERE last_login < ${date}`;
   * ```
   */
  async rawExecute(query: TemplateStringsArray, ...values: unknown[]): Promise<number> {
    return this.client.$executeRaw(query, ...values);
  }

  /**
   * Get a client from the pool for raw operations
   * Remember to release the client when done!
   */
  async getPoolClient(): Promise<PoolClient> {
    return this.pool.connect();
  }

  /**
   * Health check - verify database connectivity
   */
  async healthCheck(): Promise<{ healthy: boolean; latencyMs: number; error?: string }> {
    const start = Date.now();
    try {
      await this.client.$queryRaw`SELECT 1`;
      return {
        healthy: true,
        latencyMs: Date.now() - start,
      };
    } catch (error) {
      return {
        healthy: false,
        latencyMs: Date.now() - start,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }
}

// Type exports for convenience
export type { PrismaClient };
