//! Output formatting utilities

use colored::Colorize;

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print a step message
pub fn step(number: usize, total: usize, message: &str) {
    println!(
        "{} {}",
        format!("[{}/{}]", number, total).dimmed(),
        message
    );
}

/// Print a resource change
pub fn resource_change(change_type: &str, resource_type: &str, name: &str) {
    let symbol = match change_type {
        "create" => "+".green(),
        "update" => "~".yellow(),
        "replace" => "±".magenta(),
        "delete" => "-".red(),
        "same" => " ".normal(),
        _ => "?".normal(),
    };

    println!(
        "  {} {} {}",
        symbol.bold(),
        resource_type.cyan(),
        name.bold()
    );
}

/// Print a property diff
pub fn property_diff(path: &str, old_value: Option<&str>, new_value: Option<&str>) {
    match (old_value, new_value) {
        (None, Some(new)) => {
            println!("      {} {}: {}", "+".green(), path.dimmed(), new.green());
        }
        (Some(old), None) => {
            println!("      {} {}: {}", "-".red(), path.dimmed(), old.red());
        }
        (Some(old), Some(new)) if old != new => {
            println!(
                "      {} {}: {} → {}",
                "~".yellow(),
                path.dimmed(),
                old.red(),
                new.green()
            );
        }
        _ => {}
    }
}

/// Print a summary line
pub fn summary(creates: usize, updates: usize, deletes: usize, same: usize) {
    let mut parts = vec![];

    if creates > 0 {
        parts.push(format!("{} to create", creates).green().to_string());
    }
    if updates > 0 {
        parts.push(format!("{} to update", updates).yellow().to_string());
    }
    if deletes > 0 {
        parts.push(format!("{} to delete", deletes).red().to_string());
    }
    if same > 0 {
        parts.push(format!("{} unchanged", same).dimmed().to_string());
    }

    if parts.is_empty() {
        println!("\n{}", "No changes.".dimmed());
    } else {
        println!("\n{}", parts.join(", "));
    }
}

/// Print deployment result
pub fn deploy_result(success: bool, created: usize, updated: usize, deleted: usize, duration: f64) {
    if success {
        println!(
            "\n{} Deployment completed in {:.1}s",
            "✓".green().bold(),
            duration
        );
        println!(
            "  Resources: {} created, {} updated, {} deleted",
            created.to_string().green(),
            updated.to_string().yellow(),
            deleted.to_string().red()
        );
    } else {
        println!(
            "\n{} Deployment failed after {:.1}s",
            "✗".red().bold(),
            duration
        );
    }
}

/// Print a banner
pub fn banner(text: &str) {
    println!("\n{}", text.bold());
    println!("{}", "─".repeat(text.len()));
}

/// Print a table header
pub fn table_header(columns: &[&str]) {
    let header = columns.join("  ");
    println!("{}", header.bold());
    println!("{}", "─".repeat(header.len()));
}

/// Format bytes as human-readable
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration as human-readable
pub fn format_duration(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{}ms", (seconds * 1000.0) as u64)
    } else if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else if seconds < 3600.0 {
        let mins = (seconds / 60.0) as u64;
        let secs = (seconds % 60.0) as u64;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = (seconds / 3600.0) as u64;
        let mins = ((seconds % 3600.0) / 60.0) as u64;
        format!("{}h {}m", hours, mins)
    }
}
