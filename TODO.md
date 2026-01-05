# Devmer - Infrastructure as Code in Rust

A Rust-based Infrastructure as Code (IaC) tool similar to Pulumi/Terraform, with flexible self-hosted state management, multi-language SDK support, and a modular provider architecture.

## Vision

Devmer aims to be a fully open-source, self-hostable IaC solution that:
- Requires **no proprietary cloud service** (unlike Pulumi Cloud)
- Stores state in **user-controlled backends** (15+ options)
- Supports **imperative programming** in multiple languages
- Is built with **Rust** for performance and safety
- Uses a **modular crate architecture** for cloud providers

---

## Architecture

### Devmer Core (100% Standalone)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Devmer CLI (devmer-cli)                     │
├─────────────────────────────────────────────────────────────────────┤
│                      Core Engine (devmer-core)                      │
│      Planner │ Executor │ Diff Engine │ Resource Graph              │
├─────────────────────────────────────────────────────────────────────┤
│  devmer-state  │  devmer-secrets  │  devmer-concurrency  │ devmer-audit │
│   S3/GCS/Azure │  Passphrase/KMS  │  Distributed Locking │ SOC2/HIPAA   │
├─────────────────────────────────────────────────────────────────────┤
│                    Language SDKs (devmer-runtime)                   │
│  Python │ TypeScript │ Go │ Rust Script (Rhai) │ Deno │ Bun        │
├─────────────────────────────────────────────────────────────────────┤
│                    Provider Crates (devmer-providers)               │
│  AWS │ GCP │ Azure │ Kubernetes │ Docker │ Cloudflare              │
└─────────────────────────────────────────────────────────────────────┘
```

### Cloudmer Integration (Optional)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    DEVMER (Works 100% Standalone)                   │
├─────────────────────────────────────────────────────────────────────┤
│  devmer-cloudmer (optional client library)                          │
│  Hooks into core when CLOUDMER_TOKEN is set                         │
└────────────────────────────────┬────────────────────────────────────┘
                                 │ HTTPS API (only metadata, no secrets)
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    CLOUDMER SERVICE (cloudmer.app)                  │
│  Optional web platform for teams and enterprises                    │
├─────────────────────────────────────────────────────────────────────┤
│  📊 Visualization │ 💰 Cost Insights │ 👥 Team Collaboration       │
│  🔒 Distributed Locks │ 📋 Compliance Dashboards │ 🔔 Alerts       │
└─────────────────────────────────────────────────────────────────────┘
```

### What Works Without Cloudmer (Everything!)

| Feature | Standalone | With Cloudmer |
|---------|-----------|---------------|
| Deploy infrastructure | ✅ Full | ✅ + Visualization |
| State management | ✅ 15+ backends | ✅ + Dashboard |
| Secrets encryption | ✅ Passphrase/KMS/Vault | ✅ Same |
| Multi-user locking | ✅ devmer-concurrency | ✅ + Distributed |
| Audit logging | ✅ File/Syslog/SIEM | ✅ + Dashboard |
| Compliance reports | ✅ Local generation | ✅ + Dashboard |
| Cost tracking | ❌ | ✅ Real-time |
| Team collaboration | ❌ | ✅ Comments/approvals |

---

## Progress Overview

| Crate | Status | Description |
|-------|--------|-------------|
| `devmer-core` | ✅ Done | Resource graph, diff engine, planner, executor |
| `devmer-config` | ✅ Done | TOML config, env interpolation, .env loading |
| `devmer-di` | ✅ Done | Dependency injection with shaku |
| `devmer-state` | ✅ Done | State backend trait + S3/GCS/Azure/Local mocks |
| `devmer-secrets` | ✅ Done | Secrets encryption, providers (passphrase, KMS, Vault, Age) |
| `devmer-providers` | ✅ Done | Provider trait + Mock + AWS (30+ resource schemas) |
| `devmer-runtime` | ✅ Done | Language host trait + Rhai runtime + registry |
| `devmer-org` | ✅ Done | Organizations, teams, RBAC, resource policies, approvals |
| `devmer-rpc` | ✅ Done | gRPC protocol, Engine/Host/Provider services |
| `devmer-convert` | ✅ Done | HCL to scripting language conversion |
| `devmer-cli` | ✅ Done | Complete CLI with all commands |
| `devmer-audit` | ✅ Done | Audit logging, SOC2/HIPAA/PCI-DSS compliance |
| `devmer-concurrency` | ✅ Done | Distributed locking, multi-user coordination, conflict detection |
| `devmer-cloudmer` | ✅ Done | Optional Cloudmer integration (visualization, costs, collaboration) |
| `devmer-tui` | ⏳ Planned | Terminal UI with ratatui |
| `devmer-migrate` | ⏳ Planned | Terraform/Pulumi state import |

---

## Completed Features

### Core Engine (`devmer-core`)
- [x] Resource model with URN, inputs, outputs, dependencies
- [x] PropertyValue type system (string, int, bool, array, object, secret, output)
- [x] Dependency graph construction with petgraph
- [x] Topological sorting for deployment order
- [x] Diff engine (create, update, delete, replace detection)
- [x] Deployment planner with parallel operation support
- [x] Execution engine with event streaming
- [x] Provider trait and registry

### Configuration (`devmer-config`)
- [x] TOML configuration parsing
- [x] Layered config (file + env + CLI)
- [x] Environment variable interpolation (`${VAR}`, `${VAR:-default}`)
- [x] .env file loading with dotenvy
- [x] Stack configuration support

### Dependency Injection (`devmer-di`)
- [x] shaku-based DI container
- [x] Service interfaces (Config, State, Secrets, Runtime, Execution)
- [x] Provider registry integration
- [x] Runtime context for scripting languages

### State Management (`devmer-state`)
- [x] StateBackend trait
- [x] State locking with LockInfo/LockStatus
- [x] State versioning and history
- [x] Mock S3 backend
- [x] Mock GCS backend
- [x] Mock Azure Blob backend
- [x] Local file backend

### Secrets (`devmer-secrets`)
- [x] SecretsProvider trait
- [x] Passphrase-based encryption (Argon2 + AES-GCM)
- [x] AWS KMS provider (mock)
- [x] GCP KMS provider (mock)
- [x] Azure Key Vault provider (mock)
- [x] HashiCorp Vault provider (mock)
- [x] Age encryption provider (mock)
- [x] Secure memory handling (zeroize)

### Providers (`devmer-providers`)
- [x] Provider trait with full lifecycle (check, diff, create, read, update, delete)
- [x] MockProvider for testing
- [x] AWS Provider with 30+ resource schemas:
  - S3 Bucket, Lambda Function, IAM Role/Policy/User
  - EC2 Instance/SecurityGroup/VPC/Subnet
  - DynamoDB Table, RDS Instance, ECS Cluster/Service
  - API Gateway, CloudWatch, SNS/SQS, Route53, and more
- [x] Resource schema validation
- [x] Diff computation with replacement detection

### Runtime (`devmer-runtime`)
- [x] LanguageRuntime trait
- [x] Rhai embedded scripting runtime
- [x] Resource registration and tracking
- [x] ComponentResource support
- [x] StackReference support
- [x] Runtime context (ConfigProvider, SecretsProvider, ResourceProvider)
- [x] Output handling

### Organization Admin (`devmer-org`)
- [x] Organization/Team/User hierarchy
- [x] Role-based access control (RBAC)
- [x] Built-in roles (Owner, Admin, Member, Viewer, etc.)
- [x] Custom role creation
- [x] Resource policies with glob patterns
- [x] Stack/project scoping per team
- [x] Resource type restrictions
- [x] Environment-based approval requirements
- [x] Approval workflows

### Audit & Compliance (`devmer-audit`)
- [x] Comprehensive audit event model (50+ event types)
- [x] Event categories (deployment, resource, state, secret, auth, policy, approval, org, system, compliance)
- [x] Event severity levels (Debug, Info, Warning, Error, Critical)
- [x] Actor tracking (User, Service, System, Anonymous, ApiKey)
- [x] Resource tracking with URN support
- [x] Hash chain for tamper-evidence
- [x] Chain verification and integrity checks
- [x] File-based audit backend with daily rotation
- [x] Memory backend for testing
- [x] Multi-backend support (write to multiple backends)
- [x] Event querying with time ranges, filters, and full-text search
- [x] SOC2 Type II compliance controls (CC6.1, CC6.2, CC6.3, CC7.1, CC7.2, CC8.1)
- [x] HIPAA compliance controls (164.308, 164.312)
- [x] PCI-DSS compliance controls (7.1, 8.1, 10.1, 10.2, 10.7)
- [x] GDPR compliance controls (Art.30, Art.32, Art.33)
- [x] ISO 27001 compliance controls (A.9.2.1, A.9.2.3, A.12.4.1)
- [x] NIST Cybersecurity Framework controls
- [x] CIS Controls
- [x] Compliance checking engine
- [x] Report generation (Markdown, HTML, Text, CSV, JSON)
- [x] SIEM integration formats:
  - CEF (Common Event Format)
  - LEEF (Log Event Extended Format for IBM QRadar)
  - Syslog RFC 5424
  - Splunk HEC
  - Elasticsearch Bulk
  - CSV
  - JSON Lines

### Concurrency Control (`devmer-concurrency`)
- [x] Distributed lock manager
  - [x] Exclusive resource locking
  - [x] TTL-based lock expiration (default 30 minutes)
  - [x] Lock queuing with fair ordering (FIFO)
  - [x] Heartbeat/lease renewal
  - [x] Force-acquire for admin operations
  - [x] In-memory backend (for single-instance)
  - [x] Pluggable backend trait for distributed deployments
- [x] User session tracking
  - [x] Track active user sessions
  - [x] Monitor who's accessing which resources
  - [x] Session expiration and cleanup
  - [x] Client info tracking (hostname, IP, user agent)
- [x] Conflict detection
  - [x] Pre-operation conflict checks
  - [x] Detect concurrent modifications
  - [x] Detect locked resources
  - [x] Detect dependency conflicts
  - [x] State version mismatch detection
  - [x] Severity levels (Warning, Error, Critical)
  - [x] Recommendations for conflict resolution
- [x] Operation journal
  - [x] Full audit trail of lock operations
  - [x] Query by resource, actor, event type
  - [x] Time-range filtering
  - [x] In-memory storage with max entries

### Cloudmer Integration (`devmer-cloudmer`)
*Note: All features below are optional - Devmer works 100% without Cloudmer*
- [x] API client with device auth flow
- [x] State sync to Cloudmer for visualization
- [x] Deployment notifications
- [x] Cost insights retrieval
- [x] Integration hooks (`CloudmerHooks`)
  - [x] Environment-based configuration (`CLOUDMER_TOKEN`)
  - [x] Auto-detection of git info (commit, branch)
  - [x] CI/CD system detection
- [x] Distributed locking via Cloudmer
  - [x] Acquire/release locks via API
  - [x] Queue position tracking
  - [x] Heartbeat renewal
- [x] Clear documentation of standalone vs. enhanced features
- [x] Privacy-focused (only metadata, no secrets transmitted)

---

## Completed Recently

### CLI (`devmer-cli`)
- [x] `devmer new` - Create new project
- [x] `devmer init` - Initialize existing directory
- [x] `devmer preview` - Show planned changes
- [x] `devmer up` - Deploy infrastructure
- [x] `devmer down` - Destroy deployed infrastructure
- [x] `devmer refresh` - Refresh state from cloud
- [x] `devmer stack` - Stack management (ls, new, select, rm, history, output)
- [x] `devmer config` - Configuration management (get, set, rm)
- [x] `devmer secrets` - Secrets management (set, get, ls, rotate)
- [x] `devmer state` - State inspection/manipulation (export, import, unlock, delete)
- [x] `devmer login` - Cloud provider authentication
- [x] `devmer convert` - HCL to scripting language conversion (from, analyze, formats)
- [x] `devmer version` - Version information
- [x] Interactive prompts with dialoguer
- [x] Colored terminal output
- [x] Progress indicators
- [x] Workspace state persistence (.devmer/workspace.json)
- [x] Stack-specific config files (Devmer.{stack}.toml)
- [x] Passphrase-based secrets encryption

### HCL Conversion (`devmer-convert`)
- [x] HCL parsing with hcl-rs
- [x] Resource mapping to Devmer types
- [x] Code generation (TypeScript, Python, Go, Rhai)
- [x] IR (Intermediate Representation) for resources, variables, outputs, data sources
- [x] Expression conversion (references, function calls, conditionals, for expressions)
- [x] Provider configuration conversion
- [x] Lifecycle settings conversion
- [x] Devmer.toml generation
- [x] Package scaffolding (package.json, requirements.txt, go.mod)
- [ ] Module conversion (module calls -> components)

---

## Remaining Work

### Phase 1: MVP (v0.1.0)

#### Language SDKs
- [ ] TypeScript SDK via napi-rs
- [ ] Python SDK via pyo3
- [ ] Go SDK via cgo
- [ ] Language host gRPC protocol

#### TUI
- [ ] Main dashboard view
- [ ] Deployment progress view
- [ ] Resource tree browser
- [ ] Change preview with diffs
- [ ] State browser

#### Real Provider Implementations
- [ ] AWS SDK integration (replace mocks)
- [ ] Actual S3 state backend
- [ ] Actual DynamoDB locking

### Phase 2: Extended Backends (v0.2.0)
- [ ] PostgreSQL state backend
- [ ] MySQL/MariaDB state backend  
- [ ] Redis state backend
- [ ] etcd state backend
- [ ] Consul state backend
- [ ] Git state backend
- [ ] Kubernetes ConfigMap/Secret backend

### Phase 3: Migration (v0.3.0)
- [ ] Terraform state parser (v3/v4)
- [ ] OpenTofu state parser
- [ ] Pulumi state parser
- [ ] Migration wizard
- [ ] Code generation from state

### Phase 4: Audit & Compliance (v0.4.0) ✅
- [x] Audit event capture
- [x] File audit backend
- [x] Memory audit backend
- [x] Multi-backend support
- [x] Hash chaining for tamper-evidence
- [x] Chain verification
- [x] SOC2 compliance controls
- [x] HIPAA compliance controls
- [x] PCI-DSS compliance controls
- [x] GDPR compliance controls
- [x] ISO 27001 compliance controls
- [x] NIST CSF compliance controls
- [x] CIS Controls
- [x] Compliance report generation (Markdown, HTML, Text, CSV, JSON)
- [x] SIEM export formats (CEF, LEEF, Syslog, Splunk HEC, Elasticsearch Bulk)
- [x] Event querying and filtering
- [ ] CloudWatch audit backend (optional feature)
- [ ] S3 archival backend (optional feature)
- [ ] PostgreSQL backend (optional feature)
- [ ] Parquet archival format (optional feature)

### Phase 5: Enterprise (v0.5.0+)
- [ ] License key system
- [ ] Usage analytics
- [ ] Premium providers
- [ ] Advanced compliance features
- [ ] Priority support infrastructure

---

## Milestones

### v0.1.0 - Foundation ⏳
- [x] Core engine with resource lifecycle
- [x] Local file state backend
- [x] CLI with all commands
- [ ] Basic TUI with deployment progress
- [x] Passphrase-based secrets
- [x] AWS provider (mock)
- [x] Rust script (Rhai) support

### v0.2.0 - Cloud State & Secrets
- [x] S3/GCS/Azure state backends (mock)
- [x] State locking
- [x] KMS secrets providers (mock)
- [ ] Terraform/Pulumi state import
- [ ] Migration wizard

### v0.3.0 - Language SDKs
- [ ] Python SDK
- [ ] TypeScript SDK (Node.js)
- [ ] Deno & Bun support
- [ ] Go SDK

### v0.4.0 - Full TUI
- [ ] Dashboard view
- [ ] Resource browser
- [ ] Interactive deployment
- [ ] State browser
- [ ] Theming support

### v0.5.0 - Multi-Cloud
- [ ] Real AWS provider
- [ ] GCP provider
- [ ] Azure provider
- [ ] Kubernetes provider

### v1.0.0 - Production Ready
- [ ] All SDKs stable
- [ ] Comprehensive AWS coverage
- [ ] All state backends stable
- [ ] Full audit & compliance
- [ ] Complete documentation

---

## Non-Goals (For Now)

- **SaaS offering** - Focus on self-hosted only
- **GUI-first** - CLI/TUI-first approach
- **Terraform HCL compatibility** - New language approach
- **Automatic provider generation** - Manual quality control

---

## References

- [Pulumi Architecture](https://www.pulumi.com/docs/concepts/)
- [OpenTofu](https://opentofu.org/)
- [Terraform Provider Development](https://developer.hashicorp.com/terraform/plugin)
- [AWS Rust SDK](https://aws.amazon.com/sdk-for-rust/)
- [Rhai Scripting Language](https://rhai.rs/)
