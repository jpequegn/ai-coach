use anyhow::Result;
use clap::Args;

use crate::api::ApiClient;
use crate::config::Config;

#[derive(Args)]
pub struct WhoamiCommand {}

impl WhoamiCommand {
    pub async fn execute(self) -> Result<()> {
        let config = Config::load()?;

        if !config.is_authenticated() {
            println!("You are not logged in.");
            println!();
            println!("Use 'ai-coach login' to authenticate.");
            return Ok(());
        }

        println!("Fetching user information...");
        println!();

        let client = ApiClient::new(config)?;

        match client.whoami().await {
            Ok(user_info) => {
                println!("✓ Authenticated as:");
                println!();
                println!("  Username: {}", user_info.username);
                println!("  Email:    {}", user_info.email);
                println!("  User ID:  {}", user_info.id);

                Ok(())
            }
            Err(e) => {
                println!("✗ Failed to fetch user information: {}", e);
                println!();
                println!("Your authentication token may have expired.");
                println!("Use 'ai-coach login' to authenticate again.");
                Err(e)
            }
        }
    }
}
