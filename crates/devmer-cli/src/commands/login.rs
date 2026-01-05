//! `devmer login` command

use anyhow::Result;

use crate::output;

/// Execute the login command
pub async fn execute(provider: Option<String>) -> Result<()> {
    match provider.as_deref() {
        Some("aws") => {
            output::info("Configuring AWS credentials...");
            output::info("Run 'aws configure' or set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY environment variables.");
        }
        Some("gcp") => {
            output::info("Configuring GCP credentials...");
            output::info("Run 'gcloud auth application-default login' to authenticate.");
        }
        Some("azure") => {
            output::info("Configuring Azure credentials...");
            output::info("Run 'az login' to authenticate.");
        }
        Some(p) => {
            anyhow::bail!("Unknown provider: {}. Supported: aws, gcp, azure", p);
        }
        None => {
            output::info("Available providers:");
            println!("  aws   - Amazon Web Services");
            println!("  gcp   - Google Cloud Platform");
            println!("  azure - Microsoft Azure");
            println!();
            output::info("Run 'devmer login <provider>' to configure credentials.");
        }
    }

    Ok(())
}
