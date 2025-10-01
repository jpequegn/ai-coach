use anyhow::Result;
use clap::Args;

use crate::config::Config;

#[derive(Args)]
pub struct LogoutCommand {}

impl LogoutCommand {
    pub async fn execute(self) -> Result<()> {
        let mut config = Config::load()?;

        if !config.is_authenticated() {
            println!("You are not logged in.");
            return Ok(());
        }

        config.clear_tokens();
        config.save()?;

        println!("✓ Logged out successfully!");

        Ok(())
    }
}
