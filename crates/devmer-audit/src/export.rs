//! Export audit events to various formats

use crate::event::{AuditEvent, EventSeverity};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON Lines (one JSON object per line)
    JsonLines,
    /// Common Event Format (CEF)
    Cef,
    /// Log Event Extended Format (LEEF)
    Leef,
    /// Syslog RFC 5424
    Syslog,
    /// CSV
    Csv,
    /// Splunk HEC format
    SplunkHec,
    /// Elasticsearch bulk format
    ElasticsearchBulk,
}

impl ExportFormat {
    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            Self::JsonLines => "jsonl",
            Self::Cef => "cef",
            Self::Leef => "leef",
            Self::Syslog => "log",
            Self::Csv => "csv",
            Self::SplunkHec => "json",
            Self::ElasticsearchBulk => "ndjson",
        }
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::JsonLines => "application/x-ndjson",
            Self::Cef => "text/plain",
            Self::Leef => "text/plain",
            Self::Syslog => "text/plain",
            Self::Csv => "text/csv",
            Self::SplunkHec => "application/json",
            Self::ElasticsearchBulk => "application/x-ndjson",
        }
    }
}

/// Audit event exporter
pub struct Exporter {
    /// Export format
    format: ExportFormat,
    /// Device vendor (for CEF/LEEF)
    device_vendor: String,
    /// Device product (for CEF/LEEF)
    device_product: String,
    /// Device version (for CEF/LEEF)
    device_version: String,
    /// Elasticsearch index (for ES bulk)
    es_index: String,
    /// Splunk source type
    splunk_sourcetype: String,
}

impl Exporter {
    /// Create a new exporter
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            device_vendor: "Devmer".to_string(),
            device_product: "DevmerAudit".to_string(),
            device_version: "1.0".to_string(),
            es_index: "devmer-audit".to_string(),
            splunk_sourcetype: "devmer:audit".to_string(),
        }
    }

    /// Set device vendor
    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.device_vendor = vendor.into();
        self
    }

    /// Set device product
    pub fn with_product(mut self, product: impl Into<String>) -> Self {
        self.device_product = product.into();
        self
    }

    /// Set Elasticsearch index
    pub fn with_es_index(mut self, index: impl Into<String>) -> Self {
        self.es_index = index.into();
        self
    }

    /// Set Splunk source type
    pub fn with_splunk_sourcetype(mut self, sourcetype: impl Into<String>) -> Self {
        self.splunk_sourcetype = sourcetype.into();
        self
    }

    /// Export a single event
    pub fn export_event(&self, event: &AuditEvent) -> Result<String> {
        match self.format {
            ExportFormat::JsonLines => self.to_json_line(event),
            ExportFormat::Cef => self.to_cef(event),
            ExportFormat::Leef => self.to_leef(event),
            ExportFormat::Syslog => self.to_syslog(event),
            ExportFormat::Csv => self.to_csv_row(event),
            ExportFormat::SplunkHec => self.to_splunk_hec(event),
            ExportFormat::ElasticsearchBulk => self.to_es_bulk(event),
        }
    }

    /// Export multiple events
    pub fn export_events(&self, events: &[AuditEvent]) -> Result<String> {
        let mut output = String::new();

        // Add CSV header if needed
        if self.format == ExportFormat::Csv {
            output.push_str(&self.csv_header());
            output.push('\n');
        }

        for event in events {
            output.push_str(&self.export_event(event)?);
            output.push('\n');
        }

        Ok(output)
    }

    /// Export events to a writer
    pub fn export_to_writer<W: Write>(&self, events: &[AuditEvent], mut writer: W) -> Result<()> {
        // Add CSV header if needed
        if self.format == ExportFormat::Csv {
            writeln!(writer, "{}", self.csv_header())?;
        }

        for event in events {
            writeln!(writer, "{}", self.export_event(event)?)?;
        }

        Ok(())
    }

    /// Convert event to JSON line
    fn to_json_line(&self, event: &AuditEvent) -> Result<String> {
        Ok(serde_json::to_string(event)?)
    }

    /// Convert event to CEF format
    /// Common Event Format: CEF:Version|Device Vendor|Device Product|Device Version|Signature ID|Name|Severity|Extension
    fn to_cef(&self, event: &AuditEvent) -> Result<String> {
        let severity = cef_severity(&event.severity);
        let signature_id = format!("{:?}", event.event_type);
        let name = &event.description;

        let mut extensions = Vec::new();
        extensions.push(format!("rt={}", event.timestamp.timestamp_millis()));
        extensions.push(format!("suser={}", escape_cef(&event.actor.id)));
        
        if let Some(ref stack) = event.stack {
            extensions.push(format!("cs1={}", escape_cef(stack)));
            extensions.push(format!("cs1Label=Stack"));
        }
        
        if let Some(ref project) = event.project {
            extensions.push(format!("cs2={}", escape_cef(project)));
            extensions.push(format!("cs2Label=Project"));
        }

        if let Some(ip) = event.actor.ip_address {
            extensions.push(format!("src={}", ip));
        }

        if let Some(ref resource) = event.resource {
            extensions.push(format!("cs3={}", escape_cef(&resource.resource_type)));
            extensions.push(format!("cs3Label=ResourceType"));
            extensions.push(format!("cs4={}", escape_cef(&resource.name)));
            extensions.push(format!("cs4Label=ResourceName"));
        }

        extensions.push(format!("externalId={}", event.id));
        extensions.push(format!("outcome={:?}", event.outcome));

        Ok(format!(
            "CEF:0|{}|{}|{}|{}|{}|{}|{}",
            escape_cef(&self.device_vendor),
            escape_cef(&self.device_product),
            escape_cef(&self.device_version),
            escape_cef(&signature_id),
            escape_cef(name),
            severity,
            extensions.join(" ")
        ))
    }

    /// Convert event to LEEF format
    /// Log Event Extended Format for IBM QRadar
    fn to_leef(&self, event: &AuditEvent) -> Result<String> {
        let mut attrs = HashMap::new();
        
        attrs.insert("devTime".to_string(), event.timestamp.format("%b %d %Y %H:%M:%S").to_string());
        attrs.insert("usrName".to_string(), event.actor.id.clone());
        attrs.insert("cat".to_string(), event.event_type.category().to_string());
        attrs.insert("sev".to_string(), format!("{}", event.severity.level()));
        attrs.insert("src".to_string(), event.actor.ip_address.map(|ip| ip.to_string()).unwrap_or_default());
        
        if let Some(ref stack) = event.stack {
            attrs.insert("stack".to_string(), stack.clone());
        }

        if let Some(ref resource) = event.resource {
            attrs.insert("resourceType".to_string(), resource.resource_type.clone());
            attrs.insert("resourceName".to_string(), resource.name.clone());
        }

        let attr_str: Vec<String> = attrs
            .iter()
            .map(|(k, v)| format!("{}={}", k, escape_leef(v)))
            .collect();

        Ok(format!(
            "LEEF:2.0|{}|{}|{}|{}|{}",
            self.device_vendor,
            self.device_product,
            self.device_version,
            format!("{:?}", event.event_type),
            attr_str.join("\t")
        ))
    }

    /// Convert event to Syslog RFC 5424 format
    fn to_syslog(&self, event: &AuditEvent) -> Result<String> {
        let facility = 4; // security/authorization
        let severity = syslog_severity(&event.severity);
        let pri = facility * 8 + severity;

        let timestamp = event.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let hostname = hostname::get()
            .map(|h: OsString| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "-".to_string());
        let app_name = &self.device_product;
        let proc_id = std::process::id();
        let msg_id = format!("{:?}", event.event_type);

        // Structured data
        let sd = format!(
            "[devmer@1 actor=\"{}\" stack=\"{}\" project=\"{}\"]",
            event.actor.id,
            event.stack.as_deref().unwrap_or("-"),
            event.project.as_deref().unwrap_or("-")
        );

        Ok(format!(
            "<{}>{} {} {} {} {} {} {} {}",
            pri,
            1, // version
            timestamp,
            hostname,
            app_name,
            proc_id,
            msg_id,
            sd,
            event.description
        ))
    }

    /// Get CSV header
    fn csv_header(&self) -> String {
        "timestamp,event_id,event_type,severity,outcome,actor_id,actor_type,stack,project,resource_type,resource_name,description".to_string()
    }

    /// Convert event to CSV row
    fn to_csv_row(&self, event: &AuditEvent) -> Result<String> {
        Ok(format!(
            "{},{},{:?},{},{:?},{},{:?},{},{},{},{},{}",
            event.timestamp.to_rfc3339(),
            event.id,
            event.event_type,
            event.severity,
            event.outcome,
            escape_csv(&event.actor.id),
            event.actor.actor_type,
            event.stack.as_deref().unwrap_or(""),
            event.project.as_deref().unwrap_or(""),
            event.resource.as_ref().map(|r| r.resource_type.as_str()).unwrap_or(""),
            event.resource.as_ref().map(|r| r.name.as_str()).unwrap_or(""),
            escape_csv(&event.description),
        ))
    }

    /// Convert event to Splunk HEC format
    fn to_splunk_hec(&self, event: &AuditEvent) -> Result<String> {
        let hec_event = SplunkHecEvent {
            time: event.timestamp.timestamp() as f64,
            host: hostname::get()
                .map(|h: OsString| h.to_string_lossy().to_string())
                .ok(),
            source: Some("devmer".to_string()),
            sourcetype: Some(self.splunk_sourcetype.clone()),
            event: event.clone(),
        };

        Ok(serde_json::to_string(&hec_event)?)
    }

    /// Convert event to Elasticsearch bulk format
    fn to_es_bulk(&self, event: &AuditEvent) -> Result<String> {
        let index_line = serde_json::json!({
            "index": {
                "_index": format!("{}-{}", self.es_index, event.timestamp.format("%Y.%m.%d")),
                "_id": event.id.to_string()
            }
        });

        Ok(format!(
            "{}\n{}",
            serde_json::to_string(&index_line)?,
            serde_json::to_string(event)?
        ))
    }
}

/// Splunk HEC event format
#[derive(Serialize)]
struct SplunkHecEvent {
    time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sourcetype: Option<String>,
    event: AuditEvent,
}

/// Convert severity to CEF severity (0-10)
fn cef_severity(severity: &EventSeverity) -> u8 {
    match severity {
        EventSeverity::Debug => 0,
        EventSeverity::Info => 3,
        EventSeverity::Warning => 5,
        EventSeverity::Error => 7,
        EventSeverity::Critical => 10,
    }
}

/// Convert severity to Syslog severity (0-7)
fn syslog_severity(severity: &EventSeverity) -> u8 {
    match severity {
        EventSeverity::Debug => 7,
        EventSeverity::Info => 6,
        EventSeverity::Warning => 4,
        EventSeverity::Error => 3,
        EventSeverity::Critical => 2,
    }
}

/// Escape string for CEF format
fn escape_cef(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Escape string for LEEF format
fn escape_leef(s: &str) -> String {
    s.replace('\t', " ")
        .replace('\n', " ")
        .replace('\r', " ")
}

/// Escape string for CSV
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Actor, EventType};

    fn sample_event() -> AuditEvent {
        AuditEvent::new(
            EventType::AuthenticationSuccess,
            Actor::user("user@example.com"),
            "User logged in successfully",
        )
        .with_stack("dev")
        .with_project("my-project")
    }

    #[test]
    fn test_json_lines_export() {
        let exporter = Exporter::new(ExportFormat::JsonLines);
        let event = sample_event();
        let output = exporter.export_event(&event).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["actor"]["id"], "user@example.com");
    }

    #[test]
    fn test_cef_export() {
        let exporter = Exporter::new(ExportFormat::Cef);
        let event = sample_event();
        let output = exporter.export_event(&event).unwrap();

        assert!(output.starts_with("CEF:0|"));
        assert!(output.contains("Devmer"));
        assert!(output.contains("suser=user@example.com"));
    }

    #[test]
    fn test_syslog_export() {
        let exporter = Exporter::new(ExportFormat::Syslog);
        let event = sample_event();
        let output = exporter.export_event(&event).unwrap();

        // Should have PRI value
        assert!(output.starts_with("<"));
        assert!(output.contains("User logged in"));
    }

    #[test]
    fn test_csv_export() {
        let exporter = Exporter::new(ExportFormat::Csv);
        let events = vec![sample_event(), sample_event()];
        let output = exporter.export_events(&events).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 events
        assert!(lines[0].contains("timestamp"));
    }

    #[test]
    fn test_splunk_hec_export() {
        let exporter = Exporter::new(ExportFormat::SplunkHec);
        let event = sample_event();
        let output = exporter.export_event(&event).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed["time"].is_number());
        assert_eq!(parsed["sourcetype"], "devmer:audit");
    }

    #[test]
    fn test_elasticsearch_bulk_export() {
        let exporter = Exporter::new(ExportFormat::ElasticsearchBulk);
        let event = sample_event();
        let output = exporter.export_event(&event).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        // First line should be index action
        let action: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert!(action["index"]["_index"].is_string());
    }
}
