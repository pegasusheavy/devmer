import { injectable } from 'tsyringe';
import type { IConfigService } from '../types';

@injectable()
export class ConfigService implements IConfigService {
  private config: Map<string, unknown> = new Map();

  constructor() {
    this.loadConfig();
  }

  private loadConfig(): void {
    // Database
    this.config.set('database.url', process.env['DATABASE_URL']);

    // Server
    this.config.set('server.port', parseInt(process.env['PORT'] || '4000', 10));
    this.config.set('server.host', process.env['HOST'] || '0.0.0.0');

    // Environment
    this.config.set('env', process.env['NODE_ENV'] || 'development');

    // App
    this.config.set('app.url', process.env['APP_URL'] || 'http://localhost:3000');
    this.config.set('app.name', process.env['APP_NAME'] || 'Devmer');

    // Email
    this.config.set('email.host', process.env['SMTP_HOST']);
    this.config.set('email.port', parseInt(process.env['SMTP_PORT'] || '587', 10));
    this.config.set('email.user', process.env['SMTP_USER']);
    this.config.set('email.pass', process.env['SMTP_PASS']);
    this.config.set('email.from', process.env['EMAIL_FROM'] || 'hello@devmer.io');

    // Auth
    this.config.set('auth.jwtSecret', process.env['JWT_SECRET']);
    this.config.set('auth.sessionSecret', process.env['SESSION_SECRET']);

    // Features
    this.config.set('features.waitlist', process.env['ENABLE_WAITLIST'] === 'true');
    this.config.set('features.blog', process.env['ENABLE_BLOG'] !== 'false');
    this.config.set('features.analytics', process.env['ENABLE_ANALYTICS'] !== 'false');

    // Cache
    this.config.set('cache.ttl', parseInt(process.env['CACHE_TTL'] || '3600', 10));
  }

  get<T>(key: string): T | undefined {
    return this.config.get(key) as T | undefined;
  }

  getOrThrow<T>(key: string): T {
    const value = this.config.get(key);
    if (value === undefined || value === null) {
      throw new Error(`Configuration key "${key}" is not set`);
    }
    return value as T;
  }

  isDevelopment(): boolean {
    return this.get<string>('env') === 'development';
  }

  isProduction(): boolean {
    return this.get<string>('env') === 'production';
  }
}
