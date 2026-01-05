import { PrismaClient, UserRole, PostStatus } from '@prisma/client';

const prisma = new PrismaClient();

async function main() {
  console.log('🌱 Seeding database...');

  // Create admin user
  const admin = await prisma.user.upsert({
    where: { email: 'admin@devmer.io' },
    update: {},
    create: {
      email: 'admin@devmer.io',
      name: 'Admin User',
      role: UserRole.ADMIN,
      bio: 'Devmer team member',
    },
  });
  console.log(`✅ Created admin user: ${admin.email}`);

  // Create sample blog posts
  const posts = [
    {
      slug: 'introducing-devmer',
      title: 'Introducing Devmer: Infrastructure as Code, Reimagined',
      excerpt: 'Today we are excited to announce Devmer, a new open-source Infrastructure as Code tool built with Rust.',
      content: `
# Introducing Devmer

We're thrilled to announce **Devmer**, a new approach to Infrastructure as Code that combines the power of modern programming languages with the performance and safety of Rust.

## Why Devmer?

- **No Cloud Lock-in**: Unlike other tools, Devmer doesn't require a proprietary cloud service. Store your state anywhere.
- **Multi-Language Support**: Write your infrastructure in Python, TypeScript, Go, or Rust.
- **Blazing Fast**: Built with Rust for maximum performance.
- **Self-Hosted First**: Your infrastructure, your rules.

## Getting Started

\`\`\`bash
# Install Devmer
curl -fsSL https://get.devmer.io | sh

# Create a new project
devmer new my-infra --template aws-typescript
\`\`\`

Stay tuned for more updates!
      `.trim(),
      tags: ['announcement', 'release', 'iac'],
      category: 'announcements',
      status: PostStatus.PUBLISHED,
      featured: true,
      publishedAt: new Date(),
    },
    {
      slug: 'migrating-from-terraform',
      title: 'Migrating from Terraform to Devmer: A Complete Guide',
      excerpt: 'Learn how to migrate your existing Terraform infrastructure to Devmer with our step-by-step guide.',
      content: `
# Migrating from Terraform to Devmer

If you're looking to migrate from Terraform to Devmer, this guide will walk you through the process.

## Why Migrate?

- **Modern Languages**: Use Python, TypeScript, or Go instead of HCL
- **Better Testing**: Unit test your infrastructure like regular code
- **No State Lock-in**: Bring your own state backend

## Migration Steps

1. **Export your Terraform state**
2. **Run the Devmer migration tool**
3. **Review and customize the generated code**
4. **Deploy with Devmer**

\`\`\`bash
devmer migrate terraform --state-file terraform.tfstate
\`\`\`

The migration tool will generate equivalent Devmer code in your language of choice.
      `.trim(),
      tags: ['tutorial', 'migration', 'terraform'],
      category: 'tutorials',
      status: PostStatus.PUBLISHED,
      featured: false,
      publishedAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000), // 1 week ago
    },
    {
      slug: 'devmer-vs-pulumi',
      title: 'Devmer vs Pulumi: An Honest Comparison',
      excerpt: 'A detailed comparison between Devmer and Pulumi to help you choose the right tool.',
      content: `
# Devmer vs Pulumi: An Honest Comparison

Both Devmer and Pulumi offer modern Infrastructure as Code with real programming languages. Here's how they compare.

## Key Differences

| Feature | Devmer | Pulumi |
|---------|--------|--------|
| State Management | Self-hosted (15+ backends) | Pulumi Cloud (or self-managed) |
| Pricing | Free & Open Source | Free tier + paid plans |
| Core Language | Rust | Go |
| Languages | Python, TS, Go, Rust | Python, TS, Go, C#, Java |

## When to Choose Devmer

- You need full control over your state
- You want a truly open-source solution
- You're in a regulated industry requiring data sovereignty

## When to Choose Pulumi

- You want a managed service
- You need .NET or Java support
- You prefer their ecosystem and support
      `.trim(),
      tags: ['comparison', 'pulumi'],
      category: 'comparisons',
      status: PostStatus.DRAFT,
      featured: false,
    },
  ];

  for (const post of posts) {
    const created = await prisma.blogPost.upsert({
      where: { slug: post.slug },
      update: {},
      create: {
        ...post,
        authorId: admin.id,
      },
    });
    console.log(`✅ Created blog post: ${created.title}`);
  }

  // Create sample waitlist entries
  const waitlistEmails = [
    'early-adopter@example.com',
    'devops-lead@startup.io',
    'platform-eng@enterprise.com',
  ];

  for (const email of waitlistEmails) {
    await prisma.waitlistEntry.upsert({
      where: { email },
      update: {},
      create: {
        email,
        useCase: 'Evaluating for team adoption',
      },
    });
  }
  console.log(`✅ Created ${waitlistEmails.length} waitlist entries`);

  // Create sample feature requests
  const features = [
    {
      email: 'user@example.com',
      title: 'Support for Cloudflare Workers',
      description: 'It would be great to have a provider for managing Cloudflare Workers and related resources.',
      category: 'providers',
      votes: 42,
    },
    {
      email: 'devops@company.com',
      title: 'GitOps integration',
      description: 'Native integration with ArgoCD and Flux for GitOps workflows.',
      category: 'integrations',
      votes: 28,
    },
  ];

  for (const feature of features) {
    await prisma.featureRequest.upsert({
      where: { id: feature.title.toLowerCase().replace(/\s+/g, '-') },
      update: {},
      create: feature,
    });
  }
  console.log(`✅ Created ${features.length} feature requests`);

  console.log('🎉 Seeding complete!');
}

main()
  .catch((e) => {
    console.error('❌ Seeding failed:', e);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
