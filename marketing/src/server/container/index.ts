import 'reflect-metadata';
import { container, DependencyContainer, InjectionToken } from 'tsyringe';

// Import services for registration
import { PrismaService } from '../services/prisma.service';
import { ConfigService } from '../services/config.service';
import { CacheService } from '../services/cache.service';
import { EmailService } from '../services/email.service';
import { AnalyticsService } from '../services/analytics.service';
import { LeadService } from '../services/lead.service';
import { NewsletterService } from '../services/newsletter.service';
import { BlogService } from '../services/blog.service';
import { ContactService } from '../services/contact.service';

// Legacy DatabaseService for backwards compatibility
import { DatabaseService } from '../services/database.service';

/**
 * Injection tokens for DI container
 * 
 * @example
 * ```typescript
 * @injectable()
 * class MyService {
 *   constructor(
 *     @inject(TOKENS.Prisma) private prisma: PrismaService,
 *     @inject(TOKENS.Config) private config: IConfigService,
 *   ) {}
 * }
 * ```
 */
export const TOKENS = {
  // Infrastructure
  Config: Symbol('Config'),
  Prisma: Symbol('Prisma'),
  Database: Symbol('Database'), // Legacy alias for Prisma
  Cache: Symbol('Cache'),
  Email: Symbol('Email'),

  // Analytics
  Analytics: Symbol('Analytics'),

  // Domain Services
  Lead: Symbol('Lead'),
  Newsletter: Symbol('Newsletter'),
  Blog: Symbol('Blog'),
  Contact: Symbol('Contact'),
} as const;

// Type-safe token type
export type TokenType = typeof TOKENS;
export type TokenKey = keyof TokenType;

/**
 * Service type mapping for type-safe resolution
 */
export interface ServiceMap {
  [TOKENS.Config]: ConfigService;
  [TOKENS.Prisma]: PrismaService;
  [TOKENS.Database]: DatabaseService;
  [TOKENS.Cache]: CacheService;
  [TOKENS.Email]: EmailService;
  [TOKENS.Analytics]: AnalyticsService;
  [TOKENS.Lead]: LeadService;
  [TOKENS.Newsletter]: NewsletterService;
  [TOKENS.Blog]: BlogService;
  [TOKENS.Contact]: ContactService;
}

let configured = false;

/**
 * Configure the DI container with all services
 * Safe to call multiple times - only configures once
 */
export function configureContainer(): DependencyContainer {
  if (configured) {
    return container;
  }

  // ============================================
  // Layer 1: Configuration (no dependencies)
  // ============================================
  container.registerSingleton(TOKENS.Config, ConfigService);

  // ============================================
  // Layer 2: Infrastructure (depends on Config)
  // ============================================
  
  // Prisma database service
  container.registerSingleton(TOKENS.Prisma, PrismaService);
  
  // Legacy Database service (wraps Prisma for backwards compat)
  container.registerSingleton(TOKENS.Database, DatabaseService);
  
  // In-memory cache
  container.registerSingleton(TOKENS.Cache, CacheService);
  
  // Email service
  container.registerSingleton(TOKENS.Email, EmailService);

  // ============================================
  // Layer 3: Analytics (depends on Prisma)
  // ============================================
  container.registerSingleton(TOKENS.Analytics, AnalyticsService);

  // ============================================
  // Layer 4: Domain Services (depends on above)
  // ============================================
  container.registerSingleton(TOKENS.Lead, LeadService);
  container.registerSingleton(TOKENS.Newsletter, NewsletterService);
  container.registerSingleton(TOKENS.Blog, BlogService);
  container.registerSingleton(TOKENS.Contact, ContactService);

  configured = true;
  return container;
}

/**
 * Get the configured container
 */
export function getContainer(): DependencyContainer {
  if (!configured) {
    configureContainer();
  }
  return container;
}

/**
 * Resolve a service from the container
 * 
 * @example
 * ```typescript
 * const prisma = resolve<PrismaService>(TOKENS.Prisma);
 * const config = resolve<IConfigService>(TOKENS.Config);
 * ```
 */
export function resolve<T>(token: symbol): T {
  return container.resolve<T>(token as InjectionToken<T>);
}

/**
 * Check if container is configured
 */
export function isConfigured(): boolean {
  return configured;
}

/**
 * Reset container (useful for testing)
 */
export function resetContainer(): void {
  container.clearInstances();
  configured = false;
}

// Export the container instance
export { container };
