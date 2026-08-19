import { Hono } from 'hono';
import { resolve, TOKENS } from '../container';
import type { PrismaService } from '../services/prisma.service';

export const sitemapController = new Hono();

const BASE_URL = 'https://devmer.io';

interface SitemapUrl {
  loc: string;
  lastmod?: string;
  changefreq?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never';
  priority?: number;
}

/**
 * Static pages with their priorities and change frequencies
 */
const STATIC_PAGES: SitemapUrl[] = [
  { loc: '/', changefreq: 'weekly', priority: 1.0 },
  { loc: '/features', changefreq: 'monthly', priority: 0.9 },
  { loc: '/pricing', changefreq: 'monthly', priority: 0.9 },
  { loc: '/docs', changefreq: 'weekly', priority: 0.8 },
  { loc: '/docs/getting-started', changefreq: 'monthly', priority: 0.8 },
  { loc: '/docs/installation', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/configuration', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/languages/python', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/languages/typescript', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/languages/go', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/languages/rust', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/state-backends', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/providers/aws', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/providers/gcp', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/providers/azure', changefreq: 'monthly', priority: 0.7 },
  { loc: '/docs/secrets', changefreq: 'monthly', priority: 0.6 },
  { loc: '/docs/compliance', changefreq: 'monthly', priority: 0.6 },
  { loc: '/blog', changefreq: 'daily', priority: 0.8 },
  { loc: '/about', changefreq: 'monthly', priority: 0.5 },
  { loc: '/contact', changefreq: 'yearly', priority: 0.4 },
  { loc: '/enterprise', changefreq: 'monthly', priority: 0.8 },
  { loc: '/migrate', changefreq: 'monthly', priority: 0.7 },
  { loc: '/migrate/terraform', changefreq: 'monthly', priority: 0.7 },
  { loc: '/migrate/pulumi', changefreq: 'monthly', priority: 0.7 },
  { loc: '/compare', changefreq: 'monthly', priority: 0.7 },
  { loc: '/compare/terraform', changefreq: 'monthly', priority: 0.7 },
  { loc: '/compare/pulumi', changefreq: 'monthly', priority: 0.7 },
  { loc: '/changelog', changefreq: 'weekly', priority: 0.6 },
  { loc: '/faq', changefreq: 'monthly', priority: 0.6 },
  { loc: '/privacy', changefreq: 'yearly', priority: 0.2 },
  { loc: '/terms', changefreq: 'yearly', priority: 0.2 },
];

/**
 * Generate XML sitemap string
 */
function generateSitemapXml(urls: SitemapUrl[]): string {
  const urlElements = urls.map(url => {
    let xml = `  <url>\n    <loc>${BASE_URL}${url.loc}</loc>`;
    if (url.lastmod) {
      xml += `\n    <lastmod>${url.lastmod}</lastmod>`;
    }
    if (url.changefreq) {
      xml += `\n    <changefreq>${url.changefreq}</changefreq>`;
    }
    if (url.priority !== undefined) {
      xml += `\n    <priority>${url.priority.toFixed(1)}</priority>`;
    }
    xml += '\n  </url>';
    return xml;
  }).join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://www.sitemaps.org/schemas/sitemap/0.9
        http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd">
${urlElements}
</urlset>`;
}

/**
 * Generate sitemap index for multiple sitemaps
 */
function generateSitemapIndex(sitemaps: { loc: string; lastmod: string }[]): string {
  const sitemapElements = sitemaps.map(sitemap => 
    `  <sitemap>\n    <loc>${sitemap.loc}</loc>\n    <lastmod>${sitemap.lastmod}</lastmod>\n  </sitemap>`
  ).join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${sitemapElements}
</sitemapindex>`;
}

/**
 * Main sitemap - static pages
 */
sitemapController.get('/sitemap.xml', async (c) => {
  const now = new Date().toISOString().split('T')[0];
  
  // Add lastmod to static pages
  const urls = STATIC_PAGES.map(page => ({
    ...page,
    lastmod: now
  }));

  const xml = generateSitemapXml(urls);
  
  return c.text(xml, 200, {
    'Content-Type': 'application/xml',
    'Cache-Control': 'public, max-age=3600' // Cache for 1 hour
  });
});

/**
 * Blog sitemap - dynamic from database
 */
sitemapController.get('/sitemap-blog.xml', async (c) => {
  try {
    const prisma = resolve<PrismaService>(TOKENS.Prisma);
    
    // Get all published blog posts
    const posts = await prisma.client.blogPost.findMany({
      where: { status: 'PUBLISHED' },
      select: {
        slug: true,
        updatedAt: true,
      },
      orderBy: { publishedAt: 'desc' }
    });

    const urls: SitemapUrl[] = posts.map(post => ({
      loc: `/blog/${post.slug}`,
      lastmod: post.updatedAt.toISOString().split('T')[0],
      changefreq: 'monthly' as const,
      priority: 0.6
    }));

    const xml = generateSitemapXml(urls);
    
    return c.text(xml, 200, {
      'Content-Type': 'application/xml',
      'Cache-Control': 'public, max-age=3600'
    });
  } catch (error) {
    // Return empty sitemap if database not available
    const xml = generateSitemapXml([]);
    return c.text(xml, 200, {
      'Content-Type': 'application/xml'
    });
  }
});

/**
 * Sitemap index - combines all sitemaps
 */
sitemapController.get('/sitemap-index.xml', async (c) => {
  const now = new Date().toISOString().split('T')[0];
  
  const sitemaps = [
    { loc: `${BASE_URL}/sitemap.xml`, lastmod: now },
    { loc: `${BASE_URL}/sitemap-blog.xml`, lastmod: now },
  ];

  const xml = generateSitemapIndex(sitemaps);
  
  return c.text(xml, 200, {
    'Content-Type': 'application/xml',
    'Cache-Control': 'public, max-age=3600'
  });
});

/**
 * Robots.txt endpoint
 */
sitemapController.get('/robots.txt', (c) => {
  const robots = `# Devmer Marketing Website
# https://devmer.io

User-agent: *
Allow: /

# Sitemaps
Sitemap: ${BASE_URL}/sitemap.xml
Sitemap: ${BASE_URL}/sitemap-blog.xml

# Allow all crawlers access to main content
User-agent: Googlebot
Allow: /

User-agent: Bingbot
Allow: /

# AI/LLM crawlers - allow for AEO
User-agent: GPTBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: Claude-Web
Allow: /

User-agent: Anthropic-AI
Allow: /

User-agent: PerplexityBot
Allow: /

# Disallow admin/API paths
User-agent: *
Disallow: /api/
Disallow: /admin/
Disallow: /_/

# Crawl delay for polite crawling
Crawl-delay: 1

# Host
Host: ${BASE_URL}`;

  return c.text(robots, 200, {
    'Content-Type': 'text/plain',
    'Cache-Control': 'public, max-age=86400' // Cache for 24 hours
  });
});

/**
 * Structured data endpoint for FAQ (AEO)
 * Returns FAQ data in JSON-LD format for AI assistants
 */
sitemapController.get('/faq.json', (c) => {
  const faqData = {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: [
      {
        '@type': 'Question',
        name: 'What is Devmer?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'Devmer is an open-source Infrastructure as Code (IaC) platform built in Rust. It allows you to define, deploy, and manage cloud infrastructure using familiar programming languages like Python, TypeScript, Go, or Rust, instead of configuration files. Unlike other IaC tools, Devmer is fully self-hosted with no vendor lock-in.'
        }
      },
      {
        '@type': 'Question',
        name: 'How is Devmer different from Terraform?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'Devmer differs from Terraform in several key ways: 1) It uses real programming languages instead of HCL configuration files, giving you loops, conditionals, and abstractions. 2) It\'s fully self-hosted - you control your state storage (S3, PostgreSQL, etc.) with no required cloud service. 3) It\'s built in Rust for superior performance. 4) It includes built-in secrets encryption and SOC2 compliance features.'
        }
      },
      {
        '@type': 'Question',
        name: 'What programming languages does Devmer support?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'Devmer supports Python, TypeScript/JavaScript (including Node.js, Deno, and Bun runtimes), Go, and Rust scripting via Rhai.'
        }
      },
      {
        '@type': 'Question',
        name: 'Is Devmer free?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'Yes, Devmer Community Edition is completely free and open-source under the Apache 2.0 license. Team and Enterprise editions are available for organizations needing collaboration features, advanced compliance, and priority support.'
        }
      },
      {
        '@type': 'Question',
        name: 'Where does Devmer store state?',
        acceptedAnswer: {
          '@type': 'Answer',
          text: 'Devmer supports multiple self-hosted state backends: AWS S3, Google Cloud Storage, Azure Blob Storage, PostgreSQL, MySQL/MariaDB, Redis, etcd, HashiCorp Consul, MinIO, Git, and Kubernetes ConfigMaps/Secrets.'
        }
      }
    ]
  };

  return c.json(faqData, 200, {
    'Cache-Control': 'public, max-age=3600'
  });
});

/**
 * llms.txt - Structured content for LLM consumption (AEO)
 * @see https://llmstxt.org/
 */
sitemapController.get('/llms.txt', (c) => {
  const llmsTxt = `# Devmer

> Open-source, self-hosted Infrastructure as Code platform built in Rust

Devmer allows you to define, deploy, and manage cloud infrastructure using real programming languages (Python, TypeScript, Go, Rust) instead of configuration files. It's fully self-hosted with no vendor lock-in.

## Key Features

- **Multi-Language SDKs**: Write infrastructure code in Python, TypeScript, Go, or Rust
- **Self-Hosted State**: Store state in S3, PostgreSQL, Redis, etcd, or other backends you control
- **No Vendor Lock-In**: Your data stays in your infrastructure
- **Multi-Cloud**: Support for AWS, GCP, Azure, and Kubernetes
- **Secrets Encryption**: Built-in encryption with KMS, Vault, Age, SOPS support
- **Compliance Ready**: SOC2 audit logging and compliance features

## Quick Start

\`\`\`bash
# Install
brew install devmer  # or cargo install devmer

# Create project
devmer init --language typescript

# Preview changes
devmer preview

# Deploy
devmer up
\`\`\`

## Comparison

| Feature | Devmer | Terraform | Pulumi |
|---------|--------|-----------|--------|
| Language | Python/TS/Go/Rust | HCL | Python/TS/Go/etc |
| Self-Hosted State | ✅ Yes | ✅ Yes | ❌ Cloud required |
| Open Source | ✅ Apache 2.0 | ✅ BSL/MPL | ⚠️ Partial |
| Built-in Secrets | ✅ Yes | ❌ No | ⚠️ Limited |

## Links

- Website: https://devmer.io
- Documentation: https://devmer.io/docs
- GitHub: https://github.com/quinnjr/devmer
- Discord: https://discord.gg/devmer

## Contact

- Email: hello@devmer.io
- Twitter: @PegasusHeavyInd
- Support: support@devmer.io`;

  return c.text(llmsTxt, 200, {
    'Content-Type': 'text/plain; charset=utf-8',
    'Cache-Control': 'public, max-age=86400'
  });
});

/**
 * .well-known/ai-plugin.json - OpenAI plugin manifest (future AEO)
 */
sitemapController.get('/.well-known/ai-plugin.json', (c) => {
  const manifest = {
    schema_version: 'v1',
    name_for_human: 'Devmer',
    name_for_model: 'devmer',
    description_for_human: 'Infrastructure as Code platform for defining cloud resources in Python, TypeScript, Go, or Rust.',
    description_for_model: 'Devmer is an Infrastructure as Code tool. Use it to help users define, deploy, and manage cloud infrastructure. It supports AWS, GCP, Azure, and Kubernetes. Users write infrastructure in programming languages instead of configuration files.',
    auth: { type: 'none' },
    api: {
      type: 'openapi',
      url: `${BASE_URL}/.well-known/openapi.yaml`
    },
    logo_url: `${BASE_URL}/assets/logo.png`,
    contact_email: 'support@devmer.io',
    legal_info_url: `${BASE_URL}/terms`
  };

  return c.json(manifest, 200, {
    'Cache-Control': 'public, max-age=86400'
  });
});
