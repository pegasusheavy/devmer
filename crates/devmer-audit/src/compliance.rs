//! Compliance checking and reporting

use crate::event::{AuditEvent, EventType};
use crate::query::TimeRange;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported compliance frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceFramework {
    /// SOC 2 Type II
    SOC2,
    /// HIPAA
    HIPAA,
    /// PCI-DSS
    PCIDSS,
    /// GDPR
    GDPR,
    /// ISO 27001
    ISO27001,
    /// NIST Cybersecurity Framework
    NIST,
    /// CIS Controls
    CIS,
    /// Custom framework
    Custom,
}

impl ComplianceFramework {
    /// Get framework display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SOC2 => "SOC 2 Type II",
            Self::HIPAA => "HIPAA",
            Self::PCIDSS => "PCI-DSS",
            Self::GDPR => "GDPR",
            Self::ISO27001 => "ISO 27001",
            Self::NIST => "NIST Cybersecurity Framework",
            Self::CIS => "CIS Controls",
            Self::Custom => "Custom",
        }
    }

    /// Get all controls for this framework
    pub fn controls(&self) -> Vec<ComplianceControl> {
        match self {
            Self::SOC2 => soc2_controls(),
            Self::HIPAA => hipaa_controls(),
            Self::PCIDSS => pci_dss_controls(),
            Self::GDPR => gdpr_controls(),
            Self::ISO27001 => iso27001_controls(),
            Self::NIST => nist_controls(),
            Self::CIS => cis_controls(),
            Self::Custom => vec![],
        }
    }
}

/// A compliance control requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    /// Control ID (e.g., "CC6.1" for SOC2)
    pub id: String,
    /// Control name
    pub name: String,
    /// Control description
    pub description: String,
    /// Control category
    pub category: String,
    /// Event types that demonstrate compliance
    pub required_events: Vec<EventType>,
    /// Minimum frequency for events (events per period)
    pub min_frequency: Option<FrequencyRequirement>,
    /// Whether this control requires approval workflows
    pub requires_approval: bool,
    /// Whether this control requires access reviews
    pub requires_access_review: bool,
    /// Custom check function
    #[serde(skip)]
    pub custom_check: Option<fn(&[AuditEvent]) -> ControlCheckResult>,
}

impl ComplianceControl {
    /// Create a new control
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category: "General".to_string(),
            required_events: vec![],
            min_frequency: None,
            requires_approval: false,
            requires_access_review: false,
            custom_check: None,
        }
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Set required events
    pub fn with_required_events(mut self, events: Vec<EventType>) -> Self {
        self.required_events = events;
        self
    }

    /// Set minimum frequency
    pub fn with_min_frequency(mut self, frequency: FrequencyRequirement) -> Self {
        self.min_frequency = Some(frequency);
        self
    }

    /// Require approval workflows
    pub fn require_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }

    /// Require access reviews
    pub fn require_access_review(mut self) -> Self {
        self.requires_access_review = true;
        self
    }
}

/// Frequency requirement for a control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyRequirement {
    /// Minimum number of events
    pub min_count: usize,
    /// Time period
    pub period: FrequencyPeriod,
}

/// Time period for frequency
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FrequencyPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl FrequencyPeriod {
    /// Get the number of days in this period
    pub fn days(&self) -> i64 {
        match self {
            Self::Daily => 1,
            Self::Weekly => 7,
            Self::Monthly => 30,
            Self::Quarterly => 90,
            Self::Yearly => 365,
        }
    }
}

/// Compliance checker
pub struct ComplianceChecker {
    /// Frameworks to check
    frameworks: Vec<ComplianceFramework>,
}

impl ComplianceChecker {
    /// Create a new compliance checker
    pub fn new(frameworks: Vec<ComplianceFramework>) -> Self {
        Self { frameworks }
    }

    /// Check compliance against events
    pub fn check(&self, events: &[AuditEvent], time_range: &TimeRange) -> ComplianceReport {
        let mut report = ComplianceReport {
            generated_at: Utc::now(),
            time_range: time_range.clone(),
            total_events: events.len(),
            frameworks: HashMap::new(),
            overall_score: 0.0,
            findings: vec![],
        };

        let mut total_controls = 0;
        let mut compliant_controls = 0;

        for framework in &self.frameworks {
            let mut framework_report = FrameworkReport {
                framework: *framework,
                controls: vec![],
                compliant_count: 0,
                total_count: 0,
                score: 0.0,
            };

            for control in framework.controls() {
                let result = self.check_control(&control, events, time_range);
                
                framework_report.total_count += 1;
                total_controls += 1;

                if result.compliant {
                    framework_report.compliant_count += 1;
                    compliant_controls += 1;
                } else {
                    report.findings.push(ComplianceFinding {
                        framework: *framework,
                        control_id: control.id.clone(),
                        control_name: control.name.clone(),
                        severity: result.severity,
                        description: result.details.clone(),
                        recommendation: result.recommendation.clone(),
                        evidence_count: result.evidence_count,
                    });
                }

                framework_report.controls.push(ControlReport {
                    control,
                    result,
                });
            }

            framework_report.score = if framework_report.total_count > 0 {
                (framework_report.compliant_count as f64 / framework_report.total_count as f64) * 100.0
            } else {
                100.0
            };

            report.frameworks.insert(*framework, framework_report);
        }

        report.overall_score = if total_controls > 0 {
            (compliant_controls as f64 / total_controls as f64) * 100.0
        } else {
            100.0
        };

        report
    }

    /// Check a single control
    fn check_control(
        &self,
        control: &ComplianceControl,
        events: &[AuditEvent],
        time_range: &TimeRange,
    ) -> ControlCheckResult {
        // Filter events to time range
        let filtered_events: Vec<AuditEvent> = events
            .iter()
            .filter(|e| {
                if let Some(ref start) = time_range.start {
                    if e.timestamp < *start {
                        return false;
                    }
                }
                if let Some(ref end) = time_range.end {
                    if e.timestamp > *end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Check required events
        let matching_events: Vec<&AuditEvent> = filtered_events
            .iter()
            .filter(|e| control.required_events.contains(&e.event_type))
            .collect();

        // Capture evidence info before any potential moves
        let evidence_count = matching_events.len();
        let evidence_ids: Vec<uuid::Uuid> = matching_events.iter().map(|e| e.id).collect();

        let mut compliant = !control.required_events.is_empty() || matching_events.is_empty();
        let mut details = String::new();
        let mut recommendation = String::new();

        // Check if required events exist
        if !control.required_events.is_empty() && matching_events.is_empty() {
            compliant = false;
            details = format!(
                "No events of required types found: {:?}",
                control.required_events
            );
            recommendation = "Ensure the required audit events are being captured.".to_string();
        }

        // Check frequency requirement
        if let Some(ref freq) = control.min_frequency {
            let period_start = time_range.start.unwrap_or(Utc::now() - chrono::Duration::days(freq.period.days()));
            let events_in_period: usize = matching_events
                .iter()
                .filter(|e| e.timestamp >= period_start)
                .count();

            if events_in_period < freq.min_count {
                compliant = false;
                details = format!(
                    "Insufficient events: found {} of required {} per {:?}",
                    events_in_period, freq.min_count, freq.period
                );
                recommendation = format!(
                    "Increase frequency of {} events to meet compliance requirements.",
                    control.required_events.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join(", ")
                );
            }
        }

        // Check approval requirements
        if control.requires_approval {
            let has_approvals = filtered_events.iter().any(|e| {
                matches!(
                    e.event_type,
                    EventType::ApprovalRequested | EventType::ApprovalGranted
                )
            });
            if !has_approvals {
                compliant = false;
                details = "No approval workflow events found.".to_string();
                recommendation = "Implement approval workflows for sensitive operations.".to_string();
            }
        }

        // Check access review requirements
        if control.requires_access_review {
            let has_access_reviews = filtered_events.iter().any(|e| {
                matches!(
                    e.event_type,
                    EventType::RoleAssigned | EventType::RoleRevoked | EventType::PermissionGranted | EventType::PermissionRevoked
                )
            });
            if !has_access_reviews {
                compliant = false;
                details = "No access review events found.".to_string();
                recommendation = "Implement regular access reviews.".to_string();
            }
        }

        // Run custom check if provided
        if let Some(custom_check) = control.custom_check {
            let custom_result = custom_check(&filtered_events);
            if !custom_result.compliant {
                compliant = false;
                details = custom_result.details;
                recommendation = custom_result.recommendation;
            }
        }

        ControlCheckResult {
            compliant,
            severity: if compliant {
                FindingSeverity::None
            } else {
                FindingSeverity::Medium
            },
            details,
            recommendation,
            evidence_count,
            evidence_ids,
        }
    }
}

/// Result of checking a control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCheckResult {
    /// Whether the control is compliant
    pub compliant: bool,
    /// Severity if non-compliant
    pub severity: FindingSeverity,
    /// Details about the check
    pub details: String,
    /// Recommendation for remediation
    pub recommendation: String,
    /// Number of evidence events
    pub evidence_count: usize,
    /// IDs of evidence events
    pub evidence_ids: Vec<uuid::Uuid>,
}

/// Severity of a compliance finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Complete compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// When the report was generated
    pub generated_at: DateTime<Utc>,
    /// Time range covered
    pub time_range: TimeRange,
    /// Total events analyzed
    pub total_events: usize,
    /// Reports by framework
    pub frameworks: HashMap<ComplianceFramework, FrameworkReport>,
    /// Overall compliance score (0-100)
    pub overall_score: f64,
    /// All findings
    pub findings: Vec<ComplianceFinding>,
}

impl ComplianceReport {
    /// Get critical findings
    pub fn critical_findings(&self) -> Vec<&ComplianceFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .collect()
    }

    /// Get high severity findings
    pub fn high_findings(&self) -> Vec<&ComplianceFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::High)
            .collect()
    }

    /// Check if fully compliant
    pub fn is_compliant(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Report for a single framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkReport {
    /// Framework
    pub framework: ComplianceFramework,
    /// Control reports
    pub controls: Vec<ControlReport>,
    /// Number of compliant controls
    pub compliant_count: usize,
    /// Total number of controls
    pub total_count: usize,
    /// Compliance score (0-100)
    pub score: f64,
}

/// Report for a single control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlReport {
    /// The control
    pub control: ComplianceControl,
    /// Check result
    pub result: ControlCheckResult,
}

/// A compliance finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    /// Framework
    pub framework: ComplianceFramework,
    /// Control ID
    pub control_id: String,
    /// Control name
    pub control_name: String,
    /// Severity
    pub severity: FindingSeverity,
    /// Description
    pub description: String,
    /// Recommendation
    pub recommendation: String,
    /// Number of evidence events
    pub evidence_count: usize,
}

// =============================================================================
// Framework-specific controls
// =============================================================================

/// SOC 2 controls
fn soc2_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "CC6.1",
            "Logical Access Security",
            "The entity implements logical access security to protect against threats.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthenticationFailure,
            EventType::AuthorizationDenied,
        ])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Daily,
        }),

        ComplianceControl::new(
            "CC6.2",
            "System Credentials",
            "System credentials are managed and reviewed.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::TokenGenerated,
            EventType::TokenRevoked,
        ])
        .require_access_review(),

        ComplianceControl::new(
            "CC6.3",
            "Access Removal",
            "Access to systems is removed when no longer required.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::RoleRevoked,
            EventType::PermissionRevoked,
            EventType::MemberRemoved,
        ]),

        ComplianceControl::new(
            "CC7.1",
            "System Operations",
            "System operations are monitored to detect potential problems.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::DeploymentStarted,
            EventType::DeploymentCompleted,
            EventType::DeploymentFailed,
        ])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Weekly,
        }),

        ComplianceControl::new(
            "CC7.2",
            "Incident Management",
            "Security incidents are identified and responded to.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::PolicyViolation,
            EventType::AuthenticationFailure,
        ]),

        ComplianceControl::new(
            "CC8.1",
            "Change Management",
            "Changes to infrastructure are authorized and documented.",
        )
        .with_category("Common Criteria")
        .with_required_events(vec![
            EventType::ResourceCreated,
            EventType::ResourceUpdated,
            EventType::ResourceDeleted,
        ])
        .require_approval(),
    ]
}

/// HIPAA controls
fn hipaa_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "164.312(a)(1)",
            "Access Control",
            "Implement access controls to ePHI systems.",
        )
        .with_category("Technical Safeguards")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthorizationGranted,
            EventType::AuthorizationDenied,
        ]),

        ComplianceControl::new(
            "164.312(b)",
            "Audit Controls",
            "Implement audit controls to record and examine system activity.",
        )
        .with_category("Technical Safeguards")
        .with_required_events(vec![
            EventType::ResourceCreated,
            EventType::ResourceUpdated,
            EventType::ResourceDeleted,
            EventType::SecretAccessed,
        ])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Daily,
        }),

        ComplianceControl::new(
            "164.312(c)(1)",
            "Data Integrity",
            "Implement mechanisms to ensure ePHI integrity.",
        )
        .with_category("Technical Safeguards")
        .with_required_events(vec![
            EventType::StateWritten,
            EventType::StateRead,
        ]),

        ComplianceControl::new(
            "164.312(d)",
            "Person Authentication",
            "Implement procedures to verify person or entity identity.",
        )
        .with_category("Technical Safeguards")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthenticationFailure,
        ]),

        ComplianceControl::new(
            "164.308(a)(1)(ii)(D)",
            "Information System Activity Review",
            "Implement procedures for regular review of audit logs.",
        )
        .with_category("Administrative Safeguards")
        .with_required_events(vec![EventType::AuditLogExported])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Weekly,
        }),
    ]
}

/// PCI-DSS controls
fn pci_dss_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "7.1",
            "Limit Access",
            "Limit access to system components to only those needed.",
        )
        .with_category("Access Control")
        .with_required_events(vec![
            EventType::AuthorizationGranted,
            EventType::AuthorizationDenied,
        ])
        .require_access_review(),

        ComplianceControl::new(
            "8.1",
            "User Identification",
            "Assign unique IDs to each person with access.",
        )
        .with_category("Authentication")
        .with_required_events(vec![EventType::AuthenticationSuccess]),

        ComplianceControl::new(
            "10.1",
            "Audit Trails",
            "Implement audit trails to link all access to individual users.",
        )
        .with_category("Logging")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::ResourceCreated,
            EventType::ResourceUpdated,
            EventType::ResourceDeleted,
        ])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Daily,
        }),

        ComplianceControl::new(
            "10.2",
            "Automated Audit Trails",
            "Implement automated audit trails for reconstructing events.",
        )
        .with_category("Logging")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthenticationFailure,
            EventType::SecretAccessed,
        ]),

        ComplianceControl::new(
            "10.7",
            "Audit Trail Retention",
            "Retain audit trail history for at least one year.",
        )
        .with_category("Logging")
        .with_min_frequency(FrequencyRequirement {
            min_count: 365,
            period: FrequencyPeriod::Yearly,
        }),
    ]
}

/// GDPR controls
fn gdpr_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "Art.30",
            "Records of Processing",
            "Maintain records of processing activities.",
        )
        .with_category("Documentation")
        .with_required_events(vec![
            EventType::ResourceCreated,
            EventType::ResourceUpdated,
            EventType::ResourceDeleted,
        ]),

        ComplianceControl::new(
            "Art.32",
            "Security of Processing",
            "Implement appropriate security measures.",
        )
        .with_category("Security")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthorizationDenied,
            EventType::SecretRotated,
        ]),

        ComplianceControl::new(
            "Art.33",
            "Breach Notification",
            "Notify supervisory authority of data breaches.",
        )
        .with_category("Incident Response")
        .with_required_events(vec![
            EventType::PolicyViolation,
            EventType::AuthenticationFailure,
        ]),
    ]
}

/// ISO 27001 controls
fn iso27001_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "A.9.2.1",
            "User Registration",
            "Formal user registration and de-registration process.",
        )
        .with_category("Access Control")
        .with_required_events(vec![
            EventType::MemberAdded,
            EventType::MemberRemoved,
        ]),

        ComplianceControl::new(
            "A.9.2.3",
            "Management of Privileged Access",
            "Restrict and control allocation of privileged access.",
        )
        .with_category("Access Control")
        .with_required_events(vec![
            EventType::RoleAssigned,
            EventType::RoleRevoked,
        ])
        .require_approval(),

        ComplianceControl::new(
            "A.12.4.1",
            "Event Logging",
            "Event logs recording user activities and security events.",
        )
        .with_category("Logging")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthenticationFailure,
            EventType::ResourceCreated,
            EventType::ResourceUpdated,
        ]),
    ]
}

/// NIST Cybersecurity Framework controls
fn nist_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "PR.AC-1",
            "Identity Management",
            "Identities and credentials are issued and managed.",
        )
        .with_category("Protect")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::TokenGenerated,
            EventType::TokenRevoked,
        ]),

        ComplianceControl::new(
            "PR.PT-1",
            "Audit/Log Records",
            "Audit/log records are determined, documented, and reviewed.",
        )
        .with_category("Protect")
        .with_required_events(vec![
            EventType::AuditLogExported,
            EventType::AuditLogRotated,
        ]),

        ComplianceControl::new(
            "DE.AE-1",
            "Anomalies and Events",
            "Baseline of operations is established and managed.",
        )
        .with_category("Detect")
        .with_required_events(vec![
            EventType::ResourceDriftDetected,
            EventType::PolicyViolation,
        ]),

        ComplianceControl::new(
            "RS.AN-1",
            "Investigation Notifications",
            "Notifications from detection systems are investigated.",
        )
        .with_category("Respond")
        .with_required_events(vec![
            EventType::PolicyViolation,
            EventType::AuthorizationDenied,
        ]),
    ]
}

/// CIS Controls
fn cis_controls() -> Vec<ComplianceControl> {
    vec![
        ComplianceControl::new(
            "CIS.4",
            "Controlled Use of Admin Privileges",
            "Control use of administrative privileges.",
        )
        .with_category("Access Control")
        .with_required_events(vec![
            EventType::RoleAssigned,
            EventType::RoleRevoked,
            EventType::AuthorizationGranted,
        ])
        .require_access_review(),

        ComplianceControl::new(
            "CIS.6",
            "Maintenance, Monitoring and Analysis of Audit Logs",
            "Collect, manage, and analyze audit logs.",
        )
        .with_category("Logging")
        .with_required_events(vec![
            EventType::AuthenticationSuccess,
            EventType::AuthenticationFailure,
            EventType::ResourceCreated,
        ])
        .with_min_frequency(FrequencyRequirement {
            min_count: 1,
            period: FrequencyPeriod::Daily,
        }),

        ComplianceControl::new(
            "CIS.16",
            "Account Monitoring and Control",
            "Actively manage lifecycle of accounts.",
        )
        .with_category("Access Control")
        .with_required_events(vec![
            EventType::MemberAdded,
            EventType::MemberRemoved,
            EventType::SessionStarted,
            EventType::SessionEnded,
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Actor;

    #[test]
    fn test_compliance_checker() {
        let checker = ComplianceChecker::new(vec![ComplianceFramework::SOC2]);
        
        let events = vec![
            AuditEvent::new(
                EventType::AuthenticationSuccess,
                Actor::user("user1"),
                "User logged in",
            ),
            AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                "Deployment started",
            ),
            AuditEvent::new(
                EventType::ResourceCreated,
                Actor::user("user1"),
                "Created resource",
            ),
        ];

        let time_range = TimeRange::last_30_days();
        let report = checker.check(&events, &time_range);

        assert!(report.frameworks.contains_key(&ComplianceFramework::SOC2));
        assert!(report.overall_score > 0.0);
    }

    #[test]
    fn test_framework_controls() {
        let controls = ComplianceFramework::SOC2.controls();
        assert!(!controls.is_empty());

        let controls = ComplianceFramework::HIPAA.controls();
        assert!(!controls.is_empty());

        let controls = ComplianceFramework::PCIDSS.controls();
        assert!(!controls.is_empty());
    }
}
