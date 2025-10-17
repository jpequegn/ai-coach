use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::recovery_protocol::*;

pub struct RecoveryProtocolService {
    db: SqlitePool,
}

impl RecoveryProtocolService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Get all available protocols with optional filtering
    pub async fn get_applicable_protocols(
        &self,
        scenario: Option<String>,
        duration_max: Option<i32>,
        category: Option<String>,
    ) -> Result<Vec<RecoveryProtocol>> {
        let mut query = String::from(
            r#"
            SELECT * FROM recovery_protocols
            WHERE is_active = 1
            "#,
        );

        // Filter by scenario if provided
        if let Some(ref scenario_filter) = scenario {
            query.push_str(&format!(
                " AND target_scenarios LIKE '%{}%'",
                scenario_filter.replace("'", "''")
            ));
        }

        // Filter by duration if provided
        if let Some(max_days) = duration_max {
            query.push_str(&format!(" AND duration_days <= {}", max_days));
        }

        // Filter by category if provided
        if let Some(ref cat) = category {
            query.push_str(&format!(" AND category = '{}'", cat.replace("'", "''")));
        }

        query.push_str(" ORDER BY effectiveness_score DESC, name ASC");

        let protocols = sqlx::query_as::<_, RecoveryProtocol>(&query)
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch recovery protocols")?;

        info!(
            "Found {} applicable protocols (scenario: {:?}, duration_max: {:?}, category: {:?})",
            protocols.len(),
            scenario,
            duration_max,
            category
        );

        Ok(protocols)
    }

    /// Get a specific protocol by ID
    pub async fn get_protocol(&self, protocol_id: Uuid) -> Result<Option<RecoveryProtocol>> {
        let protocol = sqlx::query_as::<_, RecoveryProtocol>(
            r#"
            SELECT * FROM recovery_protocols
            WHERE id = ?
            "#,
        )
        .bind(protocol_id.to_string())
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch protocol")?;

        Ok(protocol)
    }

    /// Get protocol recommendations
    pub async fn get_protocol_recommendations(
        &self,
        protocol_id: Uuid,
    ) -> Result<Vec<ProtocolRecommendation>> {
        let recommendations = sqlx::query_as::<_, ProtocolRecommendation>(
            r#"
            SELECT * FROM protocol_recommendations
            WHERE protocol_id = ?
            ORDER BY sequence_order ASC
            "#,
        )
        .bind(protocol_id.to_string())
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch protocol recommendations")?;

        Ok(recommendations)
    }

    /// Activate a protocol for a user
    pub async fn activate_protocol(
        &self,
        user_id: Uuid,
        protocol_id: Uuid,
    ) -> Result<UserProtocolActivation> {
        // Check if protocol exists and is active
        let protocol = self
            .get_protocol(protocol_id)
            .await?
            .context("Protocol not found")?;

        if !protocol.is_active {
            anyhow::bail!("Protocol is not active");
        }

        // Check if user already has this protocol active
        let existing = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE user_id = ? AND protocol_id = ? AND status = 'active'
            "#,
        )
        .bind(user_id.to_string())
        .bind(protocol_id.to_string())
        .fetch_optional(&self.db)
        .await
        .context("Failed to check existing activation")?;

        if existing.is_some() {
            anyhow::bail!("Protocol is already active for this user");
        }

        // Create new activation
        let activation_id = Uuid::new_v4();
        let now = Utc::now();
        let progress = json!({}).to_string();

        sqlx::query(
            r#"
            INSERT INTO user_protocol_activations
            (id, user_id, protocol_id, status, progress, activated_at, created_at, updated_at)
            VALUES (?, ?, ?, 'active', ?, ?, ?, ?)
            "#,
        )
        .bind(activation_id.to_string())
        .bind(user_id.to_string())
        .bind(protocol_id.to_string())
        .bind(&progress)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await
        .context("Failed to create protocol activation")?;

        // Update protocol activation count
        sqlx::query(
            r#"
            UPDATE recovery_protocols
            SET times_activated = times_activated + 1
            WHERE id = ?
            "#,
        )
        .bind(protocol_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to update protocol activation count")?;

        info!(
            "Activated protocol {} for user {} (activation_id: {})",
            protocol_id, user_id, activation_id
        );

        // Fetch and return the created activation
        let activation = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch created activation")?;

        Ok(activation)
    }

    /// Get active protocols for a user
    pub async fn get_active_protocols(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserProtocolActivation>> {
        let activations = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE user_id = ? AND status = 'active'
            ORDER BY activated_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch active protocols")?;

        Ok(activations)
    }

    /// Get all protocol activations for a user (including completed/abandoned)
    pub async fn get_user_protocol_history(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserProtocolActivation>> {
        let activations = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE user_id = ?
            ORDER BY activated_at DESC
            "#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch protocol history")?;

        Ok(activations)
    }

    /// Update protocol progress
    pub async fn update_protocol_progress(
        &self,
        activation_id: Uuid,
        recommendation_id: Uuid,
        completed: bool,
    ) -> Result<UserProtocolActivation> {
        // Fetch current activation
        let mut activation = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Activation not found")?;

        if activation.status != ProtocolStatus::Active {
            anyhow::bail!("Cannot update progress for non-active protocol");
        }

        // Parse progress JSON
        let mut progress: serde_json::Value = serde_json::from_str(&activation.progress)
            .unwrap_or_else(|_| json!({}));

        // Update progress
        let recommendation_key = recommendation_id.to_string();
        if completed {
            progress[&recommendation_key] = json!({
                "completed": true,
                "completed_at": Utc::now().to_rfc3339()
            });
        } else {
            if let Some(obj) = progress.as_object_mut() {
                obj.remove(&recommendation_key);
            }
        }

        let progress_str = progress.to_string();

        // Update activation
        sqlx::query(
            r#"
            UPDATE user_protocol_activations
            SET progress = ?
            WHERE id = ?
            "#,
        )
        .bind(&progress_str)
        .bind(activation_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to update protocol progress")?;

        info!(
            "Updated progress for activation {} - recommendation {} set to {}",
            activation_id, recommendation_id, completed
        );

        // Fetch updated activation
        activation.progress = progress_str;
        Ok(activation)
    }

    /// Complete a protocol
    pub async fn complete_protocol(&self, activation_id: Uuid) -> Result<UserProtocolActivation> {
        // Fetch activation
        let activation = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Activation not found")?;

        if activation.status != ProtocolStatus::Active {
            anyhow::bail!("Protocol is not active");
        }

        // Update status to completed
        sqlx::query(
            r#"
            UPDATE user_protocol_activations
            SET status = 'completed'
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to complete protocol")?;

        // Update protocol completion count
        sqlx::query(
            r#"
            UPDATE recovery_protocols
            SET times_completed = times_completed + 1
            WHERE id = ?
            "#,
        )
        .bind(activation.protocol_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to update protocol completion count")?;

        info!(
            "Completed protocol activation {} for user {}",
            activation_id, activation.user_id
        );

        // Fetch updated activation
        let updated = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch updated activation")?;

        Ok(updated)
    }

    /// Abandon a protocol
    pub async fn abandon_protocol(
        &self,
        activation_id: Uuid,
        reason: Option<String>,
    ) -> Result<UserProtocolActivation> {
        // Fetch activation
        let activation = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Activation not found")?;

        if activation.status != ProtocolStatus::Active {
            anyhow::bail!("Protocol is not active");
        }

        // Update status to abandoned
        sqlx::query(
            r#"
            UPDATE user_protocol_activations
            SET status = 'abandoned', abandonment_reason = ?
            WHERE id = ?
            "#,
        )
        .bind(reason.clone())
        .bind(activation_id.to_string())
        .execute(&self.db)
        .await
        .context("Failed to abandon protocol")?;

        warn!(
            "Abandoned protocol activation {} for user {} - reason: {:?}",
            activation_id, activation.user_id, reason
        );

        // Fetch updated activation
        let updated = sqlx::query_as::<_, UserProtocolActivation>(
            r#"
            SELECT * FROM user_protocol_activations
            WHERE id = ?
            "#,
        )
        .bind(activation_id.to_string())
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch updated activation")?;

        Ok(updated)
    }
}
