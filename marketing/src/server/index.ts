import 'reflect-metadata';
import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { secureHeaders } from 'hono/secure-headers';

import { configureContainer, resolve, TOKENS } from './container';
import type { IConfigService } from './types';
import type { PrismaService } from './services/prisma.service';

// Controllers
import {
  newsletterController,
  contactController,
  blogController,
  analyticsController,
  sitemapController,
} from './controllers';

// Configure DI container
configureContainer();

// Create Hono app
const app = new Hono();

// Middleware
app.use('*', logger());
app.use('*', secureHeaders());
app.use(
  '*',
  cors({
    origin: (origin) => {
      const config = resolve<IConfigService>(TOKENS.Config);
      const appUrl = config.get<string>('app.url') || 'http://localhost:3000';
      // Allow same origin and configured app URL
      if (!origin || origin === appUrl) {
        return origin || appUrl;
      }
      // Allow localhost in development
      if (config.isDevelopment() && origin?.includes('localhost')) {
        return origin;
      }
      return appUrl;
    },
    credentials: true,
  })
);

// Health check
app.get('/health', (c) => {
  return c.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// SEO/AEO routes (at root level)
app.route('/', sitemapController);

// API routes
app.route('/api/newsletter', newsletterController);
app.route('/api/contact', contactController);
app.route('/api/blog', blogController);
app.route('/api/analytics', analyticsController);

// 404 handler
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404);
});

// Error handler
app.onError((err, c) => {
  console.error('Server error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

// Start server
async function main() {
  const config = resolve<IConfigService>(TOKENS.Config);
  const prisma = resolve<PrismaService>(TOKENS.Prisma);

  // Connect to database
  await prisma.connect();

  const port = config.get<number>('server.port') || 4000;
  const host = config.get<string>('server.host') || '0.0.0.0';

  console.log(`
╔═══════════════════════════════════════════════════╗
║         Devmer Marketing API Server               ║
╠═══════════════════════════════════════════════════╣
║  🚀 Server:     http://${host}:${port}              
║  📊 Environment: ${config.get<string>('env')}
║  🗄️  Database:   Connected
╚═══════════════════════════════════════════════════╝
  `);

  serve({
    fetch: app.fetch,
    port,
    hostname: host,
  });

  // Graceful shutdown
  process.on('SIGINT', async () => {
    console.log('\n📤 Shutting down...');
    await prisma.disconnect();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    console.log('\n📤 Shutting down...');
    await prisma.disconnect();
    process.exit(0);
  });
}

main().catch((error) => {
  console.error('Failed to start server:', error);
  process.exit(1);
});

export default app;
