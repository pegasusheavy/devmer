//! Resource policies for fine-grained access control

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Effect of a policy rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    /// Allow the action
    Allow,
    /// Deny the action
    #[default]
    Deny,
    /// Require approval for the action
    RequireApproval,
}

/// A single policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule name/identifier
    pub name: String,

    /// Effect of the rule
    pub effect: PolicyEffect,

    /// Stack patterns this rule applies to
    #[serde(default)]
    pub stacks: Vec<String>,

    /// Project patterns this rule applies to
    #[serde(default)]
    pub projects: Vec<String>,

    /// Resource type patterns this rule applies to
    #[serde(default)]
    pub resource_types: Vec<String>,

    /// Actions this rule applies to
    #[serde(default)]
    pub actions: Vec<String>,

    /// Environments this rule applies to
    #[serde(default)]
    pub environments: Vec<String>,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,

    /// Rule description
    pub description: Option<String>,
}

impl PolicyRule {
    /// Create a new allow rule
    pub fn allow(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            effect: PolicyEffect::Allow,
            stacks: Vec::new(),
            projects: Vec::new(),
            resource_types: Vec::new(),
            actions: Vec::new(),
            environments: Vec::new(),
            priority: 0,
            description: None,
        }
    }

    /// Create a new deny rule
    pub fn deny(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            effect: PolicyEffect::Deny,
            stacks: Vec::new(),
            projects: Vec::new(),
            resource_types: Vec::new(),
            actions: Vec::new(),
            environments: Vec::new(),
            priority: 0,
            description: None,
        }
    }

    /// Create a new require-approval rule
    pub fn require_approval(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            effect: PolicyEffect::RequireApproval,
            stacks: Vec::new(),
            projects: Vec::new(),
            resource_types: Vec::new(),
            actions: Vec::new(),
            environments: Vec::new(),
            priority: 0,
            description: None,
        }
    }

    /// Builder: set stacks
    pub fn for_stacks(mut self, patterns: &[&str]) -> Self {
        self.stacks = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder: set projects
    pub fn for_projects(mut self, patterns: &[&str]) -> Self {
        self.projects = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder: set resource types
    pub fn for_resource_types(mut self, patterns: &[&str]) -> Self {
        self.resource_types = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder: set actions
    pub fn for_actions(mut self, actions: &[&str]) -> Self {
        self.actions = actions.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder: set environments
    pub fn for_environments(mut self, envs: &[&str]) -> Self {
        self.environments = envs.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Builder: set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if this rule matches a request
    pub fn matches(
        &self,
        stack: Option<&str>,
        project: Option<&str>,
        resource_type: Option<&str>,
        action: Option<&str>,
        environment: Option<&str>,
    ) -> bool {
        // Empty list means "all"
        let stack_matches = self.stacks.is_empty()
            || stack.map(|s| self.stacks.iter().any(|p| glob_match(p, s))).unwrap_or(true);

        let project_matches = self.projects.is_empty()
            || project.map(|p| self.projects.iter().any(|pat| glob_match(pat, p))).unwrap_or(true);

        let resource_matches = self.resource_types.is_empty()
            || resource_type.map(|r| self.resource_types.iter().any(|p| glob_match(p, r))).unwrap_or(true);

        let action_matches = self.actions.is_empty()
            || action.map(|a| self.actions.iter().any(|act| act == a || act == "*")).unwrap_or(true);

        let env_matches = self.environments.is_empty()
            || environment.map(|e| self.environments.iter().any(|env| env == e || env == "*")).unwrap_or(true);

        stack_matches && project_matches && resource_matches && action_matches && env_matches
    }
}

/// Resource policy for a team or user
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcePolicy {
    /// Policy name
    pub name: String,

    /// Policy description
    pub description: Option<String>,

    /// Policy rules (evaluated in priority order)
    pub rules: Vec<PolicyRule>,

    /// Default effect when no rules match
    #[serde(default = "default_deny")]
    pub default_effect: PolicyEffect,

    /// Allowed stack patterns (convenience field)
    #[serde(default)]
    pub allowed_stacks: HashSet<String>,

    /// Denied stack patterns (convenience field)
    #[serde(default)]
    pub denied_stacks: HashSet<String>,

    /// Allowed resource type patterns (convenience field)
    #[serde(default)]
    pub allowed_resource_types: HashSet<String>,

    /// Denied resource type patterns (convenience field)
    #[serde(default)]
    pub denied_resource_types: HashSet<String>,

    /// Stacks requiring approval
    #[serde(default)]
    pub approval_required_stacks: HashSet<String>,

    /// Environments requiring approval
    #[serde(default)]
    pub approval_required_environments: HashSet<String>,
}

fn default_deny() -> PolicyEffect {
    PolicyEffect::Deny
}

impl ResourcePolicy {
    /// Create a new empty policy
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            rules: Vec::new(),
            default_effect: PolicyEffect::Deny,
            allowed_stacks: HashSet::new(),
            denied_stacks: HashSet::new(),
            allowed_resource_types: HashSet::new(),
            denied_resource_types: HashSet::new(),
            approval_required_stacks: HashSet::new(),
            approval_required_environments: HashSet::new(),
        }
    }

    /// Create a builder
    pub fn builder() -> ResourcePolicyBuilder {
        ResourcePolicyBuilder::new()
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Evaluate policy for a request
    pub fn evaluate(
        &self,
        stack: Option<&str>,
        project: Option<&str>,
        resource_type: Option<&str>,
        action: Option<&str>,
        environment: Option<&str>,
    ) -> PolicyEffect {
        // Phase 1: Check denials (highest priority)
        
        // Check denied stacks
        if let Some(stack) = stack {
            for pattern in &self.denied_stacks {
                if glob_match(pattern, stack) {
                    return PolicyEffect::Deny;
                }
            }
        }

        // Check denied resource types
        if let Some(resource_type) = resource_type {
            for pattern in &self.denied_resource_types {
                if glob_match(pattern, resource_type) {
                    return PolicyEffect::Deny;
                }
            }
        }

        // Phase 2: Check approval requirements
        
        if let Some(stack) = stack {
            for pattern in &self.approval_required_stacks {
                if glob_match(pattern, stack) {
                    return PolicyEffect::RequireApproval;
                }
            }
        }

        if let Some(env) = environment {
            if self.approval_required_environments.contains(env) {
                return PolicyEffect::RequireApproval;
            }
        }

        // Phase 3: Check allow lists (if defined, must match)
        
        // If there's an allow list for stacks and we have a stack, it must match
        if let Some(stack) = stack {
            if !self.allowed_stacks.is_empty() {
                if !self.allowed_stacks.iter().any(|p| glob_match(p, stack)) {
                    return PolicyEffect::Deny;
                }
            }
        }

        // If there's an allow list for resources and we have a resource, it must match
        if let Some(resource_type) = resource_type {
            if !self.allowed_resource_types.is_empty() {
                if !self.allowed_resource_types.iter().any(|p| glob_match(p, resource_type)) {
                    return PolicyEffect::Deny;
                }
            }
        }

        // Phase 4: Evaluate rules in priority order
        for rule in &self.rules {
            if rule.matches(stack, project, resource_type, action, environment) {
                return rule.effect;
            }
        }

        // Phase 5: Determine final result
        // If we have any allow lists defined and we passed them, allow
        let has_allow_lists = !self.allowed_stacks.is_empty() || !self.allowed_resource_types.is_empty();
        if has_allow_lists {
            // We passed all allow list checks, so allow
            return PolicyEffect::Allow;
        }

        // Return default effect
        self.default_effect
    }

    /// Check if a stack is allowed
    pub fn is_stack_allowed(&self, stack: &str) -> bool {
        matches!(
            self.evaluate(Some(stack), None, None, None, None),
            PolicyEffect::Allow | PolicyEffect::RequireApproval
        )
    }

    /// Check if a resource type is allowed
    pub fn is_resource_type_allowed(&self, resource_type: &str) -> bool {
        matches!(
            self.evaluate(None, None, Some(resource_type), None, None),
            PolicyEffect::Allow | PolicyEffect::RequireApproval
        )
    }

    /// Check if deployment requires approval
    pub fn requires_approval(
        &self,
        stack: Option<&str>,
        environment: Option<&str>,
    ) -> bool {
        matches!(
            self.evaluate(stack, None, None, Some("deploy"), environment),
            PolicyEffect::RequireApproval
        )
    }
}

/// Builder for ResourcePolicy
pub struct ResourcePolicyBuilder {
    policy: ResourcePolicy,
}

impl ResourcePolicyBuilder {
    pub fn new() -> Self {
        Self {
            policy: ResourcePolicy::new("policy"),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.policy.name = name.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.policy.description = Some(desc.into());
        self
    }

    pub fn allow_stacks(mut self, patterns: &[&str]) -> Self {
        for p in patterns {
            self.policy.allowed_stacks.insert(p.to_string());
        }
        self
    }

    pub fn deny_stacks(mut self, patterns: &[&str]) -> Self {
        for p in patterns {
            self.policy.denied_stacks.insert(p.to_string());
        }
        self
    }

    pub fn allow_resources(mut self, patterns: &[&str]) -> Self {
        for p in patterns {
            self.policy.allowed_resource_types.insert(p.to_string());
        }
        self
    }

    pub fn deny_resources(mut self, patterns: &[&str]) -> Self {
        for p in patterns {
            self.policy.denied_resource_types.insert(p.to_string());
        }
        self
    }

    pub fn require_approval_for_stacks(mut self, patterns: &[&str]) -> Self {
        for p in patterns {
            self.policy.approval_required_stacks.insert(p.to_string());
        }
        self
    }

    pub fn require_approval_for_environments(mut self, envs: &[&str]) -> Self {
        for e in envs {
            self.policy.approval_required_environments.insert(e.to_string());
        }
        self
    }

    pub fn add_rule(mut self, rule: PolicyRule) -> Self {
        self.policy.add_rule(rule);
        self
    }

    pub fn default_allow(mut self) -> Self {
        self.policy.default_effect = PolicyEffect::Allow;
        self
    }

    pub fn default_deny(mut self) -> Self {
        self.policy.default_effect = PolicyEffect::Deny;
        self
    }

    pub fn build(self) -> ResourcePolicy {
        self.policy
    }
}

impl Default for ResourcePolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Handle patterns like "*/production/*" - wildcard at start and/or end
    if pattern.starts_with("*/") && pattern.ends_with("/*") {
        // Pattern like */middle/*
        let middle = &pattern[2..pattern.len() - 2];
        return value.contains(&format!("/{}/", middle)) 
            || value.contains(&format!("/{}", middle)) && value.ends_with(middle);
    }

    if pattern.starts_with("*/") {
        // Pattern like */suffix
        let suffix = &pattern[2..];
        return value.ends_with(suffix) || value.contains(&format!("/{}", suffix));
    }

    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return value.starts_with(prefix) || value.starts_with(&format!("{}/", prefix));
    }

    if pattern.ends_with(":*") {
        let prefix = &pattern[..pattern.len() - 1];
        return value.starts_with(prefix);
    }

    if pattern.contains('*') {
        // Simple wildcard in middle
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return value.starts_with(parts[0]) && value.ends_with(parts[1]);
        }
    }

    pattern == value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_builder() {
        let policy = ResourcePolicy::builder()
            .name("marketing-policy")
            .allow_stacks(&["marketing/*", "shared/cdn"])
            .deny_resources(&["aws:iam:*", "aws:kms:*"])
            .require_approval_for_stacks(&["marketing/production/*"])
            .require_approval_for_environments(&["production"])
            .build();

        assert!(policy.is_stack_allowed("marketing/website"));
        assert!(policy.is_stack_allowed("shared/cdn"));
        assert!(!policy.is_stack_allowed("platform/api"));

        assert!(!policy.is_resource_type_allowed("aws:iam:Role"));
        assert!(policy.is_resource_type_allowed("aws:s3:Bucket"));

        assert!(policy.requires_approval(Some("marketing/production/api"), None));
        assert!(policy.requires_approval(Some("marketing/staging"), Some("production")));
    }

    #[test]
    fn test_policy_rules() {
        let mut policy = ResourcePolicy::new("test");

        policy.add_rule(
            PolicyRule::allow("allow-staging")
                .for_stacks(&["*/staging/*"])
                .for_actions(&["deploy", "preview"])
                .with_priority(10),
        );

        policy.add_rule(
            PolicyRule::require_approval("approve-production")
                .for_stacks(&["*/production/*"])
                .with_priority(20),
        );

        policy.add_rule(
            PolicyRule::deny("deny-iam")
                .for_resource_types(&["aws:iam:*"])
                .with_priority(100),
        );

        // IAM should be denied (highest priority)
        assert_eq!(
            policy.evaluate(
                Some("app/staging/api"),
                None,
                Some("aws:iam:Role"),
                Some("deploy"),
                None
            ),
            PolicyEffect::Deny
        );

        // Production requires approval
        assert_eq!(
            policy.evaluate(
                Some("app/production/api"),
                None,
                Some("aws:s3:Bucket"),
                Some("deploy"),
                None
            ),
            PolicyEffect::RequireApproval
        );

        // Staging is allowed
        assert_eq!(
            policy.evaluate(
                Some("app/staging/api"),
                None,
                Some("aws:s3:Bucket"),
                Some("deploy"),
                None
            ),
            PolicyEffect::Allow
        );
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("marketing/*", "marketing/website"));
        assert!(glob_match("marketing/*", "marketing/campaigns/summer"));
        assert!(glob_match("aws:s3:*", "aws:s3:Bucket"));
        assert!(glob_match("aws:*:Bucket", "aws:s3:Bucket"));
        assert!(!glob_match("marketing/*", "platform/api"));
    }
}
