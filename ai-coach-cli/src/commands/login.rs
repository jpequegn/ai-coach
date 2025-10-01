use anyhow::Result;
use clap::Args;
use dialoguer::{Input, Password};

use crate::config::Config;

#[derive(Args)]
pub struct LoginCommand {}

impl LoginCommand {
    pub async fn execute(self) -> Result<()> {
        println!("AI Coach - Login");
        println!();

        // Get username
        let username: String = Input::new()
            .with_prompt("Username")
            .interact_text()?;

        // Get password
        let _password = Password::new()
            .with_prompt("Password")
            .interact()?;

        println!();
        println!("Logging in as {}...", username);

        // TODO: Call API login endpoint
        // For now, just save a mock token
        let mut config = Config::load()?;
        config.set_tokens(
            "mock_access_token".to_string(),
            "mock_refresh_token".to_string(),
        );
        config.save()?;

        println!("✓ Login successful!");
        println!();
        println!("You can now use AI Coach CLI commands.");

        Ok(())
    }
}
