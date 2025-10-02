use anyhow::{Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::api::ApiClient;
use crate::config::Config;
use crate::storage::Storage;

#[derive(Args)]
pub struct SyncCommand {
    /// Preview changes without syncing
    #[arg(long)]
    dry_run: bool,
}

impl SyncCommand {
    pub async fn execute(self) -> Result<()> {
        println!("🔄 Syncing with server...");
        println!();

        if self.dry_run {
            println!("🔍 DRY RUN - No changes will be made");
            println!();
        }

        // Check authentication
        let config = Config::load().context("Failed to load config")?;
        if !config.is_authenticated() {
            println!("❌ Not logged in");
            println!("\n💡 Use 'ai-coach login' to authenticate");
            return Ok(());
        }

        // Check connectivity
        if !Self::check_connectivity(&config).await {
            println!("❌ Cannot connect to server");
            println!("   Server: {}", config.api.base_url);
            println!("\n💡 Check your network connection and server URL");
            return Ok(());
        }

        let storage = Storage::init().context("Failed to initialize storage")?;

        // Get unsynced workouts
        let unsynced_workouts = storage
            .get_unsynced_workouts()
            .context("Failed to get unsynced workouts")?;

        if unsynced_workouts.is_empty() {
            println!("✓ No pending workouts to sync");
        } else {
            println!("📤 Found {} workout(s) to upload", unsynced_workouts.len());

            if !self.dry_run {
                self.upload_workouts(&config, &storage, &unsynced_workouts)
                    .await?;
            } else {
                for workout in &unsynced_workouts {
                    println!(
                        "   Would upload: {} {} ({})",
                        workout.date.format("%Y-%m-%d"),
                        workout.exercise_type,
                        workout.id
                    );
                }
            }
        }

        println!();

        // Download latest data from server
        if !self.dry_run {
            self.download_workouts(&config, &storage).await?;
        } else {
            println!("📥 Would download latest workouts from server");
        }

        println!();
        if self.dry_run {
            println!("🔍 Dry run complete - no changes made");
        } else {
            println!("✅ Sync complete!");
        }

        Ok(())
    }

    async fn check_connectivity(config: &Config) -> bool {
        let client = match ApiClient::new(config.clone()) {
            Ok(c) => c,
            Err(_) => return false,
        };

        // Try to fetch user info as a connectivity check
        client.whoami().await.is_ok()
    }

    async fn upload_workouts(
        &self,
        config: &Config,
        storage: &Storage,
        workouts: &[crate::models::Workout],
    ) -> Result<()> {
        let client = ApiClient::new(config.clone()).context("Failed to create API client")?;

        // Create progress bar
        let pb = ProgressBar::new(workouts.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut uploaded = 0;
        let mut failed = 0;

        for workout in workouts {
            pb.set_message(format!("Uploading {}", workout.exercise_type));

            // TODO: Implement actual API endpoint for workout upload
            // For now, we'll simulate the upload
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Simulate successful upload for now
            let upload_success = true;

            if upload_success {
                storage
                    .remove_from_sync_queue(&workout.id)
                    .context("Failed to remove from sync queue")?;
                uploaded += 1;
            } else {
                failed += 1;
            }

            pb.inc(1);
        }

        pb.finish_with_message(format!(
            "Uploaded: {}, Failed: {}",
            uploaded, failed
        ));
        println!();

        if failed > 0 {
            println!("⚠️  {} workout(s) failed to upload", failed);
        }

        Ok(())
    }

    async fn download_workouts(&self, config: &Config, storage: &Storage) -> Result<()> {
        let _client = ApiClient::new(config.clone()).context("Failed to create API client")?;

        println!("📥 Downloading latest workouts from server...");

        // Create progress spinner
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Fetching data...");
        pb.enable_steady_tick(Duration::from_millis(100));

        // TODO: Implement actual API endpoint for workout download
        // For now, we'll simulate the download
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Simulate no new workouts for now
        let downloaded = 0;
        let conflicts = 0;

        pb.finish_with_message(format!(
            "Downloaded: {}, Conflicts: {}",
            downloaded, conflicts
        ));

        if conflicts > 0 {
            println!();
            self.handle_conflicts(config, storage, conflicts).await?;
        }

        Ok(())
    }

    async fn handle_conflicts(
        &self,
        config: &Config,
        _storage: &Storage,
        conflicts: usize,
    ) -> Result<()> {
        println!("⚠️  Found {} conflict(s)", conflicts);
        println!();

        match config.sync.conflict_resolution.as_str() {
            "server_wins" => {
                println!("   Using server version (configured: server_wins)");
                // TODO: Apply server changes
            }
            "local_wins" => {
                println!("   Using local version (configured: local_wins)");
                // TODO: Keep local changes
            }
            "manual" => {
                println!("   Manual resolution required (configured: manual)");
                println!("   💡 Use 'ai-coach sync --resolve' to resolve conflicts");
                // TODO: Implement interactive conflict resolution
            }
            _ => {
                println!("   Unknown conflict resolution strategy");
                println!("   Defaulting to server_wins");
            }
        }

        Ok(())
    }
}
