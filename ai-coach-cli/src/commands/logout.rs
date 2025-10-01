use anyhow::Result;
use clap::Args;

use crate::api::ApiClient;
use crate::config::Config;

#[derive(Args)]
pub struct LogoutCommand {}

impl LogoutCommand {
    pub async fn execute(self) -> Result<()> {
        let config = Config::load()?;

        if !config.is_authenticated() {
            println!("You are not logged in.");
            return Ok(());
        }

        // Create API client and call logout endpoint
        let client = ApiClient::new(config)?;

        match client.post("/api/v1/auth/logout", &serde_json::json!({})).await {
            Ok(_) => {
                tracing::debug!("Server-side logout successful");
            }
            Err(e) => {
                tracing::warn!("Server-side logout failed: {}", e);
                println!("⚠ Warning: Could not invalidate token on server");
            }
        }

        // Clear local tokens regardless of API response (handled through the mutex)
        // Note: Since ApiClient has Arc<Mutex<Config>>, we need to access through it
        // But actually, we should clear after API calls are done, so let's load a fresh config
        let mut fresh_config = Config::load()?;
        fresh_config.clear_tokens();
        fresh_config.save()?;

        println!("✓ Logged out successfully!");

        Ok(())
    }
}
