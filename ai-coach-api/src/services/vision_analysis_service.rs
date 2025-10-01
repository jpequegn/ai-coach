use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::vision_analysis::*;

/// Service for managing vision analysis database operations
pub struct VisionAnalysisService {
    pub db: PgPool,
}

impl VisionAnalysisService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Create a new vision analysis record
    pub async fn create_analysis(
        &self,
        user_id: Uuid,
        video_url: String,
        exercise_type: Option<String>,
    ) -> Result<VisionAnalysis> {
        let analysis = sqlx::query_as::<_, VisionAnalysis>(
            r#"
            INSERT INTO vision_analyses (user_id, video_url, exercise_type, status)
            VALUES ($1, $2, $3, 'uploaded')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(video_url)
        .bind(exercise_type)
        .fetch_one(&self.db)
        .await
        .context("Failed to create vision analysis")?;

        Ok(analysis)
    }

    /// Get vision analysis by ID
    pub async fn get_analysis(&self, analysis_id: Uuid) -> Result<Option<VisionAnalysis>> {
        let analysis = sqlx::query_as::<_, VisionAnalysis>(
            "SELECT * FROM vision_analyses WHERE id = $1",
        )
        .bind(analysis_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch vision analysis")?;

        Ok(analysis)
    }

    /// Get vision analysis by ID and user ID (for authorization)
    pub async fn get_user_analysis(
        &self,
        analysis_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<VisionAnalysis>> {
        let analysis = sqlx::query_as::<_, VisionAnalysis>(
            "SELECT * FROM vision_analyses WHERE id = $1 AND user_id = $2",
        )
        .bind(analysis_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch user vision analysis")?;

        Ok(analysis)
    }

    /// Update analysis status
    pub async fn update_status(
        &self,
        analysis_id: Uuid,
        status: AnalysisStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE vision_analyses
            SET status = $1,
                error_message = $2,
                processing_started_at = CASE
                    WHEN $1 = 'processing' AND processing_started_at IS NULL THEN $3
                    ELSE processing_started_at
                END,
                processing_completed_at = CASE
                    WHEN $1 IN ('completed', 'failed') THEN $3
                    ELSE processing_completed_at
                END
            WHERE id = $4
            "#,
        )
        .bind(status.to_string())
        .bind(error_message)
        .bind(now)
        .bind(analysis_id)
        .execute(&self.db)
        .await
        .context("Failed to update analysis status")?;

        Ok(())
    }

    /// Update video metadata after processing
    pub async fn update_video_metadata(
        &self,
        analysis_id: Uuid,
        duration_seconds: f64,
        resolution: String,
        format: String,
        size_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE vision_analyses
            SET video_duration_seconds = $1,
                video_resolution = $2,
                video_format = $3,
                video_size_bytes = $4
            WHERE id = $5
            "#,
        )
        .bind(duration_seconds)
        .bind(resolution)
        .bind(format)
        .bind(size_bytes)
        .bind(analysis_id)
        .execute(&self.db)
        .await
        .context("Failed to update video metadata")?;

        Ok(())
    }

    /// List user's vision analyses
    pub async fn list_user_analyses(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<VisionAnalysis>> {
        let analyses = sqlx::query_as::<_, VisionAnalysis>(
            r#"
            SELECT * FROM vision_analyses
            WHERE user_id = $1
            ORDER BY upload_timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .context("Failed to list user analyses")?;

        Ok(analyses)
    }

    /// Delete vision analysis
    pub async fn delete_analysis(&self, analysis_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM vision_analyses WHERE id = $1")
            .bind(analysis_id)
            .execute(&self.db)
            .await
            .context("Failed to delete vision analysis")?;

        Ok(())
    }

    /// Save pose detections for a frame
    pub async fn save_pose_detection(
        &self,
        analysis_id: Uuid,
        frame_number: i32,
        timestamp_ms: i32,
        keypoints: Vec<Keypoint>,
        confidence_score: f64,
    ) -> Result<PoseDetection> {
        let keypoints_json = serde_json::to_value(&keypoints)?;

        let detection = sqlx::query_as::<_, PoseDetection>(
            r#"
            INSERT INTO pose_detections (analysis_id, frame_number, timestamp_ms, keypoints, confidence_score)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (analysis_id, frame_number) DO UPDATE
            SET keypoints = EXCLUDED.keypoints,
                confidence_score = EXCLUDED.confidence_score
            RETURNING *
            "#,
        )
        .bind(analysis_id)
        .bind(frame_number)
        .bind(timestamp_ms)
        .bind(keypoints_json)
        .bind(confidence_score)
        .fetch_one(&self.db)
        .await
        .context("Failed to save pose detection")?;

        Ok(detection)
    }

    /// Get pose detections for an analysis
    pub async fn get_pose_detections(&self, analysis_id: Uuid) -> Result<Vec<PoseDetection>> {
        let detections = sqlx::query_as::<_, PoseDetection>(
            "SELECT * FROM pose_detections WHERE analysis_id = $1 ORDER BY frame_number",
        )
        .bind(analysis_id)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch pose detections")?;

        Ok(detections)
    }

    /// Save movement scores
    pub async fn save_movement_score(
        &self,
        analysis_id: Uuid,
        overall_score: f64,
        form_quality: Option<f64>,
        injury_risk: Option<f64>,
        range_of_motion: Option<f64>,
        tempo_consistency: Option<f64>,
        rep_count: Option<i32>,
        issues: Vec<MovementIssue>,
        recommendations: Vec<MovementRecommendation>,
        biomechanics_data: serde_json::Value,
    ) -> Result<MovementScore> {
        let issues_json = serde_json::to_value(&issues)?;
        let recommendations_json = serde_json::to_value(&recommendations)?;

        let score = sqlx::query_as::<_, MovementScore>(
            r#"
            INSERT INTO movement_scores (
                analysis_id, overall_score, form_quality, injury_risk,
                range_of_motion, tempo_consistency, rep_count,
                issues, recommendations, biomechanics_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (analysis_id) DO UPDATE
            SET overall_score = EXCLUDED.overall_score,
                form_quality = EXCLUDED.form_quality,
                injury_risk = EXCLUDED.injury_risk,
                range_of_motion = EXCLUDED.range_of_motion,
                tempo_consistency = EXCLUDED.tempo_consistency,
                rep_count = EXCLUDED.rep_count,
                issues = EXCLUDED.issues,
                recommendations = EXCLUDED.recommendations,
                biomechanics_data = EXCLUDED.biomechanics_data
            RETURNING *
            "#,
        )
        .bind(analysis_id)
        .bind(overall_score)
        .bind(form_quality)
        .bind(injury_risk)
        .bind(range_of_motion)
        .bind(tempo_consistency)
        .bind(rep_count)
        .bind(issues_json)
        .bind(recommendations_json)
        .bind(biomechanics_data)
        .fetch_one(&self.db)
        .await
        .context("Failed to save movement score")?;

        Ok(score)
    }

    /// Get movement score for an analysis
    pub async fn get_movement_score(&self, analysis_id: Uuid) -> Result<Option<MovementScore>> {
        let score = sqlx::query_as::<_, MovementScore>(
            "SELECT * FROM movement_scores WHERE analysis_id = $1",
        )
        .bind(analysis_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch movement score")?;

        Ok(score)
    }

    /// Get complete analysis result with scores and issues
    pub async fn get_complete_result(&self, analysis_id: Uuid) -> Result<Option<VisionAnalysisResult>> {
        let analysis = self.get_analysis(analysis_id).await?;
        let analysis = match analysis {
            Some(a) => a,
            None => return Ok(None),
        };

        let movement_score = self.get_movement_score(analysis_id).await?;

        let (scores, rep_count, issues, recommendations) = if let Some(score) = movement_score {
            let issues: Vec<MovementIssue> = serde_json::from_value(score.issues.clone())?;
            let recommendations: Vec<MovementRecommendation> =
                serde_json::from_value(score.recommendations.clone())?;

            let scores = ScoresSummary {
                overall: score.overall_score,
                form_quality: score.form_quality,
                injury_risk: score.injury_risk,
                range_of_motion: score.range_of_motion,
                tempo_consistency: score.tempo_consistency,
            };

            (Some(scores), score.rep_count, issues, recommendations)
        } else {
            (None, None, Vec::new(), Vec::new())
        };

        Ok(Some(VisionAnalysisResult {
            id: analysis.id,
            user_id: analysis.user_id,
            video_url: analysis.video_url,
            status: analysis.status,
            exercise_type: analysis.exercise_type,
            duration_seconds: analysis.video_duration_seconds,
            processing_time_seconds: analysis.processing_time_seconds(),
            scores,
            rep_count,
            issues,
            recommendations,
            overlay_url: None, // TODO: Generate overlay URL
            keypoints_data_url: None, // TODO: Generate keypoints data URL
            upload_timestamp: analysis.upload_timestamp,
            processing_completed_at: analysis.processing_completed_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require database setup
    // See tests/integration tests for full database test coverage
}
