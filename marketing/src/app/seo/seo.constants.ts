import type { PageMeta, FAQItem } from './seo.service';

/**
 * Default page metadata
 */
export const DEFAULT_META: PageMeta = {
  title: 'Devmer - Infrastructure as Code in Rust',
  description: 'Open-source, self-hosted Infrastructure as Code platform. Define cloud infrastructure using Python, TypeScript, Go, or Rust with no vendor lock-in.',
  keywords: [
    'infrastructure as code',
    'IaC',
    'Rust',
    'DevOps',
    'cloud infrastructure',
    'Terraform alternative',
    'Pulumi alternative',
    'self-hosted',
    'AWS',
    'GCP',
    'Azure',
    'TypeScript',
    'Python',
    'Go'
  ]
};

/**
 * Page-specific metadata
 */
export const PAGE_META: Record<string, PageMeta> = {
  home: {
    title: 'Devmer - Infrastructure as Code in Rust | Self-Hosted IaC Platform',
    description: 'Devmer is an open-source, self-hosted Infrastructure as Code platform built in Rust. Define cloud infrastructure using Python, TypeScript, Go, or Rust. No vendor lock-in.',
    keywords: DEFAULT_META.keywords
  },
  
  features: {
    title: 'Features - Multi-Language IaC with Self-Hosted State',
    description: 'Explore Devmer features: multi-language SDKs (Python, TypeScript, Go, Rust), self-hosted state backends, secrets encryption, SOC2 compliance, and Terminal UI.',
    keywords: [
      'IaC features',
      'multi-language SDK',
      'state management',
      'secrets encryption',
      'SOC2 compliance',
      'Terminal UI'
    ]
  },
  
  pricing: {
    title: 'Pricing - Free Open Source IaC with Team & Enterprise Options',
    description: 'Devmer Community Edition is free and open-source. Team and Enterprise plans add collaboration, compliance, and support features for growing organizations.',
    keywords: [
      'IaC pricing',
      'open source',
      'free infrastructure as code',
      'enterprise IaC',
      'team collaboration'
    ]
  },
  
  docs: {
    title: 'Documentation - Getting Started with Devmer',
    description: 'Learn how to install Devmer, configure state backends, write infrastructure code in your preferred language, and deploy to AWS, GCP, or Azure.',
    keywords: [
      'Devmer documentation',
      'IaC tutorial',
      'infrastructure code examples',
      'getting started'
    ]
  },
  
  blog: {
    title: 'Blog - Infrastructure as Code Insights & Updates',
    description: 'Latest news, tutorials, and insights about Infrastructure as Code, cloud infrastructure, DevOps best practices, and Devmer updates.',
    keywords: [
      'IaC blog',
      'DevOps blog',
      'cloud infrastructure',
      'tutorials'
    ],
    ogType: 'website'
  },
  
  about: {
    title: 'About Devmer - Built by Joseph R. Quinn',
    description: 'Devmer is built by Joseph R. Quinn. Learn about our mission to make Infrastructure as Code accessible, self-hosted, and developer-friendly.',
    keywords: [
      'about Devmer',
      'Joseph R. Quinn',
      'IaC company'
    ]
  },
  
  contact: {
    title: 'Contact Us - Get in Touch with Devmer Team',
    description: 'Have questions about Devmer? Contact our team for sales inquiries, technical support, or partnership opportunities.',
    keywords: [
      'contact Devmer',
      'IaC support',
      'sales'
    ]
  },
  
  enterprise: {
    title: 'Enterprise - Devmer for Large Organizations',
    description: 'Devmer Enterprise offers SSO/SAML, audit logging, compliance reporting, custom providers, and dedicated support for large organizations.',
    keywords: [
      'enterprise IaC',
      'SSO',
      'audit logging',
      'compliance',
      'SOC2',
      'HIPAA'
    ]
  },
  
  migrate: {
    title: 'Migrate from Terraform or Pulumi to Devmer',
    description: 'Step-by-step guide to migrate your existing Terraform, OpenTofu, or Pulumi infrastructure to Devmer. Includes HCL converter and state importer.',
    keywords: [
      'migrate from Terraform',
      'Terraform to Devmer',
      'Pulumi migration',
      'HCL converter'
    ]
  },
  
  compare: {
    title: 'Devmer vs Terraform vs Pulumi - IaC Platform Comparison',
    description: 'Compare Devmer with Terraform and Pulumi. See how self-hosted state, multi-language support, and pricing stack up across IaC platforms.',
    keywords: [
      'Devmer vs Terraform',
      'Devmer vs Pulumi',
      'IaC comparison',
      'Terraform alternative'
    ]
  }
};

/**
 * FAQ items for AEO (Answer Engine Optimization)
 * These are structured to provide clear, direct answers that AI assistants can use
 */
export const MAIN_FAQS: FAQItem[] = [
  {
    question: 'What is Devmer?',
    answer: 'Devmer is an open-source Infrastructure as Code (IaC) platform built in Rust. It allows you to define, deploy, and manage cloud infrastructure using familiar programming languages like Python, TypeScript, Go, or Rust, instead of configuration files. Unlike other IaC tools, Devmer is fully self-hosted with no vendor lock-in - you control your state storage using S3, PostgreSQL, or other backends.'
  },
  {
    question: 'How is Devmer different from Terraform?',
    answer: 'Devmer differs from Terraform in several key ways: 1) It uses real programming languages instead of HCL configuration files, giving you loops, conditionals, functions, and abstractions. 2) It\'s fully self-hosted - you control your state storage (S3, PostgreSQL, etc.) with no required cloud service. 3) It\'s built in Rust for superior performance. 4) It includes built-in secrets encryption and SOC2 compliance features. 5) It has a built-in HCL converter to help migrate existing Terraform code.'
  },
  {
    question: 'How is Devmer different from Pulumi?',
    answer: 'While both Devmer and Pulumi use programming languages for IaC, Devmer is fully self-hosted with no required cloud service. Your state stays in your infrastructure (S3, PostgreSQL, etc.), not in a vendor\'s cloud. Devmer also offers a Terminal UI for deployment visualization, built-in HCL migration tools, is built in Rust for performance, and is completely open-source with an Apache 2.0 license.'
  },
  {
    question: 'What programming languages does Devmer support?',
    answer: 'Devmer supports Python, TypeScript/JavaScript (including Node.js, Deno, and Bun runtimes), Go, and Rust scripting via Rhai. This allows teams to use their preferred language and existing skills to define infrastructure, with full IDE support, type checking, and testing capabilities.'
  },
  {
    question: 'Where does Devmer store infrastructure state?',
    answer: 'Devmer supports multiple self-hosted state backends: AWS S3 with DynamoDB locking, Google Cloud Storage with Firestore locking, Azure Blob Storage with Cosmos DB locking, PostgreSQL, MySQL/MariaDB, Redis, etcd, HashiCorp Consul, MinIO, Git repositories, and Kubernetes ConfigMaps/Secrets. You maintain full control over your infrastructure state with no vendor dependencies.'
  },
  {
    question: 'Is Devmer free to use?',
    answer: 'Yes, Devmer Community Edition is completely free and open-source under the Apache 2.0 license. It includes the full IaC engine, all language SDKs (Python, TypeScript, Go, Rust), all state backends, and the Terminal UI. Team ($49/user/month) and Enterprise (custom pricing) editions add collaboration features, advanced compliance, SSO, and priority support.'
  },
  {
    question: 'Can I migrate from Terraform to Devmer?',
    answer: 'Yes, Devmer includes comprehensive migration tools. The HCL converter transforms Terraform configurations into Devmer code in your preferred programming language (Python, TypeScript, Go, or Rust). The state importer can migrate your existing Terraform or OpenTofu state files. The migration wizard guides you through the process step by step.'
  },
  {
    question: 'What cloud providers does Devmer support?',
    answer: 'Devmer supports major cloud providers including AWS (with 30+ resource types), Google Cloud Platform (GCP), and Microsoft Azure. It also supports Kubernetes for container orchestration. The provider system is extensible, so you can create custom providers for any API or service using Devmer\'s provider SDK.'
  },
  {
    question: 'Does Devmer support secrets management?',
    answer: 'Yes, Devmer has built-in secrets encryption and management. It supports multiple encryption providers: passphrase-based (PBKDF2/Argon2), AWS KMS, GCP KMS, Azure Key Vault, HashiCorp Vault, Age encryption, and SOPS. Secrets are encrypted at rest and never stored in plain text in state files.'
  },
  {
    question: 'Is Devmer SOC2 compliant?',
    answer: 'Devmer Enterprise includes SOC2 compliance features: comprehensive audit logging with tamper-evident hash chaining, compliance report generation, SIEM integration (CloudWatch, S3 archival), and HIPAA/PCI-DSS templates. The self-hosted architecture means you maintain data sovereignty and can meet regulatory requirements.'
  },
  {
    question: 'How do I install Devmer?',
    answer: 'Install Devmer using your package manager: `brew install devmer` (macOS), `cargo install devmer` (Rust), or download binaries from GitHub releases. Then run `devmer init` to create a new project, configure your state backend, and start defining infrastructure in your preferred language.'
  },
  {
    question: 'Does Devmer have a GUI or dashboard?',
    answer: 'Devmer includes a Terminal UI (TUI) built with Ratatui that provides real-time deployment visualization, resource graphs, and interactive state management. For teams needing a web dashboard, Devmer Enterprise includes a web-based administration interface.'
  }
];

/**
 * Getting Started FAQ for documentation pages
 */
export const GETTING_STARTED_FAQS: FAQItem[] = [
  {
    question: 'How do I create a new Devmer project?',
    answer: 'Run `devmer init` in your terminal. This creates a new project with a Devmer.toml configuration file and example infrastructure code in your chosen language. You\'ll be prompted to select your preferred language (Python, TypeScript, Go, or Rust) and state backend.'
  },
  {
    question: 'How do I preview changes before deploying?',
    answer: 'Run `devmer preview` to see a detailed diff of what will change. This shows resources to be created, updated, or deleted without making any actual changes. Review the preview output, then run `devmer up` to apply the changes.'
  },
  {
    question: 'How do I destroy infrastructure created by Devmer?',
    answer: 'Run `devmer destroy` to remove all resources managed by the current stack. You\'ll see a preview of resources to be deleted and must confirm before Devmer removes them. Use `devmer destroy --target <resource>` to destroy specific resources.'
  },
  {
    question: 'How do I manage multiple environments (dev, staging, prod)?',
    answer: 'Use Devmer stacks to manage multiple environments. Each stack has its own state and configuration. Run `devmer stack init <name>` to create a stack, then `devmer stack select <name>` to switch between them. Use stack configuration files to customize settings per environment.'
  }
];

/**
 * Comparison FAQs for landing pages
 */
export const COMPARISON_FAQS: FAQItem[] = [
  {
    question: 'Should I use Devmer, Terraform, or Pulumi?',
    answer: 'Choose Devmer if you want: 1) Full control over your state with no vendor cloud dependency, 2) Real programming languages instead of config files, 3) A Rust-based tool with excellent performance, 4) Built-in compliance features. Choose Terraform if you prefer HCL syntax and have existing HCL investments. Choose Pulumi if you\'re comfortable with their cloud service managing your state.'
  },
  {
    question: 'Can Devmer replace Terraform?',
    answer: 'Yes, Devmer can fully replace Terraform for infrastructure management. It supports the same cloud providers (AWS, GCP, Azure) and resource types. The HCL converter helps migrate existing Terraform code, and the state importer transfers your infrastructure state. Many teams find programming languages more maintainable than HCL for complex infrastructure.'
  },
  {
    question: 'Is Devmer compatible with Terraform providers?',
    answer: 'Devmer has its own provider system designed for better type safety and performance. However, Devmer providers cover the same AWS, GCP, and Azure resources as Terraform. The migration tooling helps convert Terraform configurations to equivalent Devmer code.'
  }
];
