use anyhow::Result;
use clap::Args;
use dialoguer::{Input, Password};

use crate::api::ApiClient;
use crate::config::Config;

#[derive(Args)]
pub struct LoginCommand {}

impl LoginCommand {
    pub async fn execute(self) -> Result<()> {
        println!("AI Coach - Login");
        println!();

        // Get email
        let email: String = Input::new().with_prompt("Email").interact_text()?;

        // Get password
        let password = Password::new().with_prompt("Password").interact()?;

        println!();
        println!("Logging in as {}...", email);

        // Load config and create API client
        let config = Config::load()?;
        let client = ApiClient::new(config)?;

        // Call API login endpoint
        match client.login(&email, &password).await {
            Ok(response) => {
                // Tokens are automatically saved by ApiClient
                println!("✓ Login successful!");
                println!();
                println!("Welcome, {}!", response.user.email);
                println!("Role: {}", response.user.role);
                println!();
                println!("You can now use AI Coach CLI commands.");

                Ok(())
            }
            Err(e) => {
                println!("✗ Login failed: {}", e);
                Err(e)
            }
        }
    }
}
