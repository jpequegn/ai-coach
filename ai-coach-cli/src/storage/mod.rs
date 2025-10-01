// Local storage module - Phase 5 implementation
// TODO: Implement with sled or redb to avoid rusqlite/sqlx libsqlite3-sys conflict

use anyhow::Result;
use std::path::PathBuf;

/// Storage manager for local database
/// Currently a placeholder - will be implemented in Phase 5
pub struct Storage {}

impl Storage {
    /// Get database file path (~/.ai-coach/local.db)
    pub fn db_path() -> Result<PathBuf> {
        let config_dir = crate::config::Config::config_dir()?;
        Ok(config_dir.join("local.db"))
    }

    /// Initialize storage
    /// Placeholder - to be implemented in Phase 5 with sled or redb
    pub fn init() -> Result<Self> {
        tracing::info!("Storage initialization (Phase 5 - coming soon)");
        Ok(Self {})
    }

    /// Check if database is initialized
    pub fn is_initialized() -> Result<bool> {
        let db_path = Self::db_path()?;
        Ok(db_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_placeholder() {
        let storage = Storage::init();
        assert!(storage.is_ok());
    }
}
