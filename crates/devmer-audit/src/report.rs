//! Report generation

use crate::compliance::{ComplianceReport, FindingSeverity};
use crate::event::AuditEvent;
use crate::query::TimeRange;
use crate::{AuditError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::{Context, Tera};

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Plain text
    Text,
    /// Markdown
    Markdown,
    /// HTML
    Html,
    /// JSON
    Json,
    /// CSV
    Csv,
}

impl ReportFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Text => "text/plain",
            Self::Markdown => "text/markdown",
            Self::Html => "text/html",
            Self::Json => "application/json",
            Self::Csv => "text/csv",
        }
    }
}

/// Report configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Report title
    pub title: String,
    /// Organization name
    pub organization: Option<String>,
    /// Report author
    pub author: Option<String>,
    /// Include detailed evidence
    pub include_evidence: bool,
    /// Include recommendations
    pub include_recommendations: bool,
    /// Include executive summary
    pub include_executive_summary: bool,
    /// Maximum evidence items per finding
    pub max_evidence_per_finding: usize,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            title: "Compliance Report".to_string(),
            organization: None,
            author: None,
            include_evidence: true,
            include_recommendations: true,
            include_executive_summary: true,
            max_evidence_per_finding: 10,
        }
    }
}

/// Report generator
pub struct ReportGenerator {
    templates: Tera,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new() -> Result<Self> {
        let mut templates = Tera::default();

        // Add built-in templates
        templates
            .add_raw_template("compliance_md", COMPLIANCE_MARKDOWN_TEMPLATE)
            .map_err(|e| AuditError::TemplateError(e.to_string()))?;

        templates
            .add_raw_template("compliance_html", COMPLIANCE_HTML_TEMPLATE)
            .map_err(|e| AuditError::TemplateError(e.to_string()))?;

        templates
            .add_raw_template("audit_summary_md", AUDIT_SUMMARY_MARKDOWN_TEMPLATE)
            .map_err(|e| AuditError::TemplateError(e.to_string()))?;

        Ok(Self { templates })
    }

    /// Generate a compliance report
    pub fn generate_compliance_report(
        &self,
        report: &ComplianceReport,
        format: ReportFormat,
        config: &ReportConfig,
    ) -> Result<String> {
        match format {
            ReportFormat::Json => {
                serde_json::to_string_pretty(report).map_err(|e| AuditError::SerializationError(e))
            }
            ReportFormat::Markdown => self.render_compliance_markdown(report, config),
            ReportFormat::Html => self.render_compliance_html(report, config),
            ReportFormat::Text => self.render_compliance_text(report, config),
            ReportFormat::Csv => self.render_compliance_csv(report),
        }
    }

    /// Render compliance report as Markdown
    fn render_compliance_markdown(
        &self,
        report: &ComplianceReport,
        config: &ReportConfig,
    ) -> Result<String> {
        let mut context = Context::new();
        context.insert("report", report);
        context.insert("config", config);
        context.insert("generated_at", &report.generated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string());

        self.templates
            .render("compliance_md", &context)
            .map_err(|e| AuditError::TemplateError(e.to_string()))
    }

    /// Render compliance report as HTML
    fn render_compliance_html(
        &self,
        report: &ComplianceReport,
        config: &ReportConfig,
    ) -> Result<String> {
        let mut context = Context::new();
        context.insert("report", report);
        context.insert("config", config);
        context.insert("generated_at", &report.generated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string());

        self.templates
            .render("compliance_html", &context)
            .map_err(|e| AuditError::TemplateError(e.to_string()))
    }

    /// Render compliance report as plain text
    fn render_compliance_text(
        &self,
        report: &ComplianceReport,
        config: &ReportConfig,
    ) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str(&format!("{}\n", "=".repeat(60)));
        output.push_str(&format!("{}\n", config.title));
        output.push_str(&format!("{}\n\n", "=".repeat(60)));

        if let Some(ref org) = config.organization {
            output.push_str(&format!("Organization: {}\n", org));
        }
        output.push_str(&format!("Generated: {}\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
        output.push_str(&format!("Total Events Analyzed: {}\n", report.total_events));
        output.push_str(&format!("Overall Compliance Score: {:.1}%\n\n", report.overall_score));

        // Executive Summary
        if config.include_executive_summary {
            output.push_str("EXECUTIVE SUMMARY\n");
            output.push_str(&format!("{}\n\n", "-".repeat(40)));

            let critical = report.findings.iter().filter(|f| f.severity == FindingSeverity::Critical).count();
            let high = report.findings.iter().filter(|f| f.severity == FindingSeverity::High).count();
            let medium = report.findings.iter().filter(|f| f.severity == FindingSeverity::Medium).count();

            output.push_str(&format!("Critical Findings: {}\n", critical));
            output.push_str(&format!("High Findings: {}\n", high));
            output.push_str(&format!("Medium Findings: {}\n\n", medium));
        }

        // Framework Reports
        for (framework, framework_report) in &report.frameworks {
            output.push_str(&format!("\n{}\n", framework.display_name()));
            output.push_str(&format!("{}\n", "-".repeat(40)));
            output.push_str(&format!("Score: {:.1}%\n", framework_report.score));
            output.push_str(&format!("Controls: {}/{}\n\n", framework_report.compliant_count, framework_report.total_count));

            for control_report in &framework_report.controls {
                let status = if control_report.result.compliant { "✓" } else { "✗" };
                output.push_str(&format!("[{}] {} - {}\n", status, control_report.control.id, control_report.control.name));
                
                if !control_report.result.compliant && config.include_recommendations {
                    output.push_str(&format!("    Issue: {}\n", control_report.result.details));
                    output.push_str(&format!("    Recommendation: {}\n", control_report.result.recommendation));
                }
            }
        }

        Ok(output)
    }

    /// Render compliance report as CSV
    fn render_compliance_csv(&self, report: &ComplianceReport) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("Framework,Control ID,Control Name,Compliant,Severity,Details,Recommendation,Evidence Count\n");

        for (framework, framework_report) in &report.frameworks {
            for control_report in &framework_report.controls {
                let row = format!(
                    "{},{},{},{},{:?},{},{},{}\n",
                    framework.display_name(),
                    control_report.control.id,
                    escape_csv(&control_report.control.name),
                    control_report.result.compliant,
                    control_report.result.severity,
                    escape_csv(&control_report.result.details),
                    escape_csv(&control_report.result.recommendation),
                    control_report.result.evidence_count,
                );
                output.push_str(&row);
            }
        }

        Ok(output)
    }

    /// Generate an audit summary report
    pub fn generate_audit_summary(
        &self,
        events: &[AuditEvent],
        time_range: &TimeRange,
        format: ReportFormat,
        config: &ReportConfig,
    ) -> Result<String> {
        let summary = AuditSummary::from_events(events, time_range);

        match format {
            ReportFormat::Json => {
                serde_json::to_string_pretty(&summary).map_err(|e| AuditError::SerializationError(e))
            }
            ReportFormat::Markdown => self.render_audit_summary_markdown(&summary, config),
            _ => self.render_audit_summary_text(&summary, config),
        }
    }

    /// Render audit summary as Markdown
    fn render_audit_summary_markdown(
        &self,
        summary: &AuditSummary,
        config: &ReportConfig,
    ) -> Result<String> {
        let mut context = Context::new();
        context.insert("summary", summary);
        context.insert("config", config);

        self.templates
            .render("audit_summary_md", &context)
            .map_err(|e| AuditError::TemplateError(e.to_string()))
    }

    /// Render audit summary as text
    fn render_audit_summary_text(
        &self,
        summary: &AuditSummary,
        config: &ReportConfig,
    ) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format!("{}\n", config.title));
        output.push_str(&format!("{}\n\n", "=".repeat(config.title.len())));

        output.push_str(&format!("Period: {} - {}\n", 
            summary.start_time.map(|t| t.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "N/A".to_string()),
            summary.end_time.map(|t| t.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "N/A".to_string()),
        ));
        output.push_str(&format!("Total Events: {}\n\n", summary.total_events));

        output.push_str("Events by Category:\n");
        for (category, count) in &summary.events_by_category {
            output.push_str(&format!("  {}: {}\n", category, count));
        }

        output.push_str("\nEvents by Severity:\n");
        for (severity, count) in &summary.events_by_severity {
            output.push_str(&format!("  {}: {}\n", severity, count));
        }

        Ok(output)
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create report generator")
    }
}

/// Audit summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    /// Start of the period
    pub start_time: Option<DateTime<Utc>>,
    /// End of the period
    pub end_time: Option<DateTime<Utc>>,
    /// Total number of events
    pub total_events: usize,
    /// Events by category
    pub events_by_category: HashMap<String, usize>,
    /// Events by severity
    pub events_by_severity: HashMap<String, usize>,
    /// Events by outcome
    pub events_by_outcome: HashMap<String, usize>,
    /// Top actors by event count
    pub top_actors: Vec<(String, usize)>,
    /// Top stacks by event count
    pub top_stacks: Vec<(String, usize)>,
    /// Security events
    pub security_events: usize,
    /// Failed events
    pub failed_events: usize,
}

impl AuditSummary {
    /// Create a summary from events
    pub fn from_events(events: &[AuditEvent], _time_range: &TimeRange) -> Self {
        let mut events_by_category: HashMap<String, usize> = HashMap::new();
        let mut events_by_severity: HashMap<String, usize> = HashMap::new();
        let mut events_by_outcome: HashMap<String, usize> = HashMap::new();
        let mut actors: HashMap<String, usize> = HashMap::new();
        let mut stacks: HashMap<String, usize> = HashMap::new();
        let mut security_events = 0;
        let mut failed_events = 0;

        let mut start_time: Option<DateTime<Utc>> = None;
        let mut end_time: Option<DateTime<Utc>> = None;

        for event in events {
            // Track time range
            if start_time.is_none() || event.timestamp < start_time.unwrap() {
                start_time = Some(event.timestamp);
            }
            if end_time.is_none() || event.timestamp > end_time.unwrap() {
                end_time = Some(event.timestamp);
            }

            // Count by category
            let category = event.event_type.category().to_string();
            *events_by_category.entry(category).or_insert(0) += 1;

            // Count by severity
            let severity = format!("{}", event.severity);
            *events_by_severity.entry(severity).or_insert(0) += 1;

            // Count by outcome
            let outcome = format!("{:?}", event.outcome);
            *events_by_outcome.entry(outcome).or_insert(0) += 1;

            // Count actors
            *actors.entry(event.actor.id.clone()).or_insert(0) += 1;

            // Count stacks
            if let Some(ref stack) = event.stack {
                *stacks.entry(stack.clone()).or_insert(0) += 1;
            }

            // Count security events
            if event.event_type.is_security_event() {
                security_events += 1;
            }

            // Count failed events
            if event.outcome == crate::event::EventOutcome::Failure {
                failed_events += 1;
            }
        }

        // Sort and take top 10
        let mut top_actors: Vec<_> = actors.into_iter().collect();
        top_actors.sort_by(|a, b| b.1.cmp(&a.1));
        top_actors.truncate(10);

        let mut top_stacks: Vec<_> = stacks.into_iter().collect();
        top_stacks.sort_by(|a, b| b.1.cmp(&a.1));
        top_stacks.truncate(10);

        Self {
            start_time,
            end_time,
            total_events: events.len(),
            events_by_category,
            events_by_severity,
            events_by_outcome,
            top_actors,
            top_stacks,
            security_events,
            failed_events,
        }
    }
}

/// Escape a string for CSV
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// =============================================================================
// Templates
// =============================================================================

const COMPLIANCE_MARKDOWN_TEMPLATE: &str = r#"# {{ config.title }}

{% if config.organization %}**Organization:** {{ config.organization }}{% endif %}
**Generated:** {{ generated_at }}
**Total Events Analyzed:** {{ report.total_events }}
**Overall Compliance Score:** {{ report.overall_score | round(precision=1) }}%

{% if config.include_executive_summary %}
## Executive Summary

| Severity | Count |
|----------|-------|
| Critical | {{ report.findings | filter(attribute="severity", value="Critical") | length }} |
| High | {{ report.findings | filter(attribute="severity", value="High") | length }} |
| Medium | {{ report.findings | filter(attribute="severity", value="Medium") | length }} |
| Low | {{ report.findings | filter(attribute="severity", value="Low") | length }} |
{% endif %}

## Framework Reports

{% for framework, framework_report in report.frameworks %}
### {{ framework_report.framework | title }}

**Score:** {{ framework_report.score | round(precision=1) }}%
**Controls:** {{ framework_report.compliant_count }}/{{ framework_report.total_count }}

| Control | Name | Status |
|---------|------|--------|
{% for control_report in framework_report.controls %}| {{ control_report.control.id }} | {{ control_report.control.name }} | {% if control_report.result.compliant %}✅ Compliant{% else %}❌ Non-Compliant{% endif %} |
{% endfor %}

{% endfor %}

{% if report.findings | length > 0 %}
## Findings

{% for finding in report.findings %}
### {{ finding.control_id }}: {{ finding.control_name }}

- **Framework:** {{ finding.framework | title }}
- **Severity:** {{ finding.severity | title }}
- **Description:** {{ finding.description }}
{% if config.include_recommendations %}- **Recommendation:** {{ finding.recommendation }}{% endif %}
- **Evidence Count:** {{ finding.evidence_count }}

{% endfor %}
{% endif %}
"#;

const COMPLIANCE_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ config.title }}</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }
        h1, h2, h3 { color: #333; }
        .summary { background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0; }
        .score { font-size: 48px; font-weight: bold; }
        .score.good { color: #28a745; }
        .score.warn { color: #ffc107; }
        .score.bad { color: #dc3545; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background: #333; color: white; }
        .compliant { color: #28a745; }
        .non-compliant { color: #dc3545; }
        .finding { background: #fff3cd; padding: 15px; margin: 10px 0; border-radius: 4px; border-left: 4px solid #ffc107; }
        .finding.critical { background: #f8d7da; border-color: #dc3545; }
        .finding.high { background: #fff3cd; border-color: #ffc107; }
    </style>
</head>
<body>
    <h1>{{ config.title }}</h1>
    
    <div class="summary">
        <p><strong>Generated:</strong> {{ generated_at }}</p>
        {% if config.organization %}<p><strong>Organization:</strong> {{ config.organization }}</p>{% endif %}
        <p><strong>Total Events:</strong> {{ report.total_events }}</p>
        <p class="score {% if report.overall_score >= 80 %}good{% elif report.overall_score >= 60 %}warn{% else %}bad{% endif %}">
            {{ report.overall_score | round(precision=1) }}%
        </p>
    </div>

    {% for framework, framework_report in report.frameworks %}
    <h2>{{ framework_report.framework | title }}</h2>
    <p>Score: {{ framework_report.score | round(precision=1) }}% ({{ framework_report.compliant_count }}/{{ framework_report.total_count }} controls)</p>
    
    <table>
        <thead>
            <tr><th>Control ID</th><th>Name</th><th>Status</th></tr>
        </thead>
        <tbody>
            {% for control_report in framework_report.controls %}
            <tr>
                <td>{{ control_report.control.id }}</td>
                <td>{{ control_report.control.name }}</td>
                <td class="{% if control_report.result.compliant %}compliant{% else %}non-compliant{% endif %}">
                    {% if control_report.result.compliant %}✅ Compliant{% else %}❌ Non-Compliant{% endif %}
                </td>
            </tr>
            {% endfor %}
        </tbody>
    </table>
    {% endfor %}

    {% if report.findings | length > 0 %}
    <h2>Findings</h2>
    {% for finding in report.findings %}
    <div class="finding {{ finding.severity | lower }}">
        <h3>{{ finding.control_id }}: {{ finding.control_name }}</h3>
        <p><strong>Severity:</strong> {{ finding.severity | title }}</p>
        <p>{{ finding.description }}</p>
        {% if config.include_recommendations %}<p><strong>Recommendation:</strong> {{ finding.recommendation }}</p>{% endif %}
    </div>
    {% endfor %}
    {% endif %}
</body>
</html>
"#;

const AUDIT_SUMMARY_MARKDOWN_TEMPLATE: &str = r#"# {{ config.title }}

**Period:** {{ summary.start_time }} - {{ summary.end_time }}
**Total Events:** {{ summary.total_events }}

## Events by Category

| Category | Count |
|----------|-------|
{% for category, count in summary.events_by_category %}| {{ category }} | {{ count }} |
{% endfor %}

## Events by Severity

| Severity | Count |
|----------|-------|
{% for severity, count in summary.events_by_severity %}| {{ severity }} | {{ count }} |
{% endfor %}

## Top Actors

| Actor | Events |
|-------|--------|
{% for actor in summary.top_actors %}| {{ actor.0 }} | {{ actor.1 }} |
{% endfor %}

## Statistics

- **Security Events:** {{ summary.security_events }}
- **Failed Events:** {{ summary.failed_events }}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compliance::{ComplianceChecker, ComplianceFramework};
    use crate::event::{Actor, EventType};

    #[test]
    fn test_report_generator() {
        let generator = ReportGenerator::new().unwrap();

        let events = vec![
            AuditEvent::new(
                EventType::AuthenticationSuccess,
                Actor::user("user1"),
                "Login",
            ),
            AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                "Deploy",
            ),
        ];

        let checker = ComplianceChecker::new(vec![ComplianceFramework::SOC2]);
        let time_range = TimeRange::last_30_days();
        let report = checker.check(&events, &time_range);

        let config = ReportConfig::default();

        // Test JSON
        let json = generator
            .generate_compliance_report(&report, ReportFormat::Json, &config)
            .unwrap();
        assert!(json.contains("soc2")); // ComplianceFramework serializes as lowercase

        // Test Markdown
        let md = generator
            .generate_compliance_report(&report, ReportFormat::Markdown, &config)
            .unwrap();
        assert!(md.contains("Compliance"));

        // Test Text
        let text = generator
            .generate_compliance_report(&report, ReportFormat::Text, &config)
            .unwrap();
        assert!(text.contains("Compliance"));

        // Test CSV
        let csv = generator
            .generate_compliance_report(&report, ReportFormat::Csv, &config)
            .unwrap();
        assert!(csv.contains("Framework"));
    }

    #[test]
    fn test_audit_summary() {
        let events = vec![
            AuditEvent::new(EventType::AuthenticationSuccess, Actor::user("user1"), "Login")
                .with_stack("dev"),
            AuditEvent::new(EventType::DeploymentStarted, Actor::user("user2"), "Deploy")
                .with_stack("prod"),
        ];

        let summary = AuditSummary::from_events(&events, &TimeRange::last_7_days());

        assert_eq!(summary.total_events, 2);
        assert!(summary.events_by_category.contains_key("authentication"));
        assert!(summary.events_by_category.contains_key("deployment"));
    }
}
