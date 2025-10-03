use anyhow::Result;
use std::process::Command;

use crate::config::Config;

pub async fn show_config() -> Result<()> {
    let config = Config::load()?;
    let config_str = toml::to_string_pretty(&config)?;

    println!("Current Configuration");
    println!("────────────────────────────────");
    println!();
    println!("{}", config_str);

    Ok(())
}

pub async fn edit_config() -> Result<()> {
    let config_file = Config::config_file()?;

    // Ensure config file exists
    if !config_file.exists() {
        let config = Config::default();
        config.save()?;
    }

    // Open in default editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    Command::new(editor).arg(&config_file).status()?;

    println!("✓ Configuration saved!");

    Ok(())
}

pub async fn init_config(force: bool) -> Result<()> {
    let config_file = Config::config_file()?;

    if config_file.exists() && !force {
        println!(
            "Configuration file already exists at: {}",
            config_file.display()
        );
        println!("Use --force to overwrite");
        return Ok(());
    }

    let config = Config::default();
    config.save()?;

    println!("✓ Configuration initialized at: {}", config_file.display());
    println!();
    println!("You can edit it with: ai-coach config edit");

    Ok(())
}
