import { injectable, inject } from 'tsyringe';
import { PrismaClient } from '@prisma/client';
import { PrismaPg } from '@prisma/adapter-pg';
import { Pool } from 'pg';
import type { IDatabaseService, IConfigService } from '../types';
import { TOKENS } from '../container';

@injectable()
export class DatabaseService implements IDatabaseService {
  private client: PrismaClient | null = null;
  private pool: Pool | null = null;
  private connected = false;

  constructor(
    @inject(TOKENS.Config) private configService: IConfigService
  ) {}

  async connect(): Promise<void> {
    if (this.connected) return;

    const databaseUrl = this.configService.get<string>('database.url');

    if (!databaseUrl) {
      throw new Error('DATABASE_URL is not configured');
    }

    this.pool = new Pool({ connectionString: databaseUrl });
    const adapter = new PrismaPg(this.pool);

    this.client = new PrismaClient({
      adapter,
      log: this.configService.isDevelopment()
        ? ['query', 'error', 'warn']
        : ['error'],
    });

    // Test connection
    await this.client.$connect();
    this.connected = true;

    console.log('✅ Database connected');
  }

  async disconnect(): Promise<void> {
    if (!this.connected) return;

    await this.client?.$disconnect();
    await this.pool?.end();

    this.client = null;
    this.pool = null;
    this.connected = false;

    console.log('📤 Database disconnected');
  }

  isConnected(): boolean {
    return this.connected;
  }

  getClient(): PrismaClient {
    if (!this.client) {
      throw new Error('Database not connected. Call connect() first.');
    }
    return this.client;
  }
}
