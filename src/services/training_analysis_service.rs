use anyhow::{Result, anyhow};
use bytes::Bytes;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::models::TrainingSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub duration_seconds: Option<i32>,
    pub distance_meters: Option<f64>,
    pub elevation_gain_meters: Option<f64>,
    pub average_power: Option<f64>,
    pub normalized_power: Option<f64>,
    pub average_heart_rate: Option<f64>,
    pub average_cadence: Option<f64>,
    pub average_speed: Option<f64>,
    pub tss: Option<f64>, // Training Stress Score
    pub intensity_factor: Option<f64>,
    pub work: Option<f64>, // Total work in kJ
    pub power_zones: Option<HashMap<String, f64>>, // Zone distribution
    pub heart_rate_zones: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceManagementChart {
    pub ctl: f64, // Chronic Training Load (Fitness)
    pub atl: f64, // Acute Training Load (Fatigue)
    pub tsb: f64, // Training Stress Balance (Form)
    pub date: chrono::NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneSettings {
    pub power_zones: Option<Vec<f64>>, // Thresholds for power zones (W)
    pub heart_rate_zones: Option<Vec<f64>>, // Thresholds for HR zones (bpm)
    pub ftp: Option<f64>, // Functional Threshold Power
    pub lthr: Option<f64>, // Lactate Threshold Heart Rate
}

#[derive(Debug, Clone)]
pub enum FileType {
    Tcx,
    Gpx,
    Csv,
}

impl FileType {
    pub fn from_filename(filename: &str) -> Result<Self> {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("tcx") => Ok(FileType::Tcx),
            Some("gpx") => Ok(FileType::Gpx),
            Some("csv") => Ok(FileType::Csv),
            _ => Err(anyhow!("Unsupported file type: {}", filename)),
        }
    }
}

#[derive(Clone)]
pub struct TrainingAnalysisService {
    db: PgPool,
    redis_client: Option<redis::Client>,
    upload_dir: String,
}

impl TrainingAnalysisService {
    pub fn new(db: PgPool, redis_url: Option<String>) -> Result<Self> {
        let redis_client = if let Some(url) = redis_url {
            Some(redis::Client::open(url)?)
        } else {
            None
        };

        let upload_dir = std::env::var("UPLOAD_DIR")
            .unwrap_or_else(|_| "uploads/training_files".to_string());

        Ok(Self {
            db,
            redis_client,
            upload_dir,
        })
    }

    /// Process uploaded training file and extract metrics
    pub async fn process_training_file(
        &self,
        file_data: Bytes,
        filename: &str,
        user_id: Uuid,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        info!("Processing training file: {} for user: {}", filename, user_id);

        // Validate file type
        let file_type = FileType::from_filename(filename)?;

        // Save file temporarily for processing
        let temp_file = self.save_temp_file(&file_data, filename).await?;

        // Process file based on type
        let metrics = match file_type {
            FileType::Tcx => self.process_tcx_file(&temp_file, zone_settings).await?,
            FileType::Gpx => self.process_gpx_file(&temp_file, zone_settings).await?,
            FileType::Csv => self.process_csv_file(&temp_file, zone_settings).await?,
        };

        // Clean up temporary file
        if let Err(e) = fs::remove_file(&temp_file).await {
            warn!("Failed to remove temporary file {}: {}", temp_file, e);
        }

        info!("Successfully processed training file: {}", filename);
        Ok(metrics)
    }

    /// Calculate Performance Management Chart metrics for a user
    pub async fn calculate_pmc(
        &self,
        user_id: Uuid,
        days: i32,
    ) -> Result<Vec<PerformanceManagementChart>> {
        info!("Calculating PMC for user: {} over {} days", user_id, days);

        // Check cache first
        if let Some(cached) = self.get_cached_pmc(user_id, days).await? {
            return Ok(cached);
        }

        // Get training sessions for the specified period
        let sessions = self.get_training_sessions_for_pmc(user_id, days).await?;

        // Calculate PMC values
        let pmc_data = self.compute_pmc_values(sessions).await?;

        // Cache the results
        self.cache_pmc_data(user_id, days, &pmc_data).await?;

        Ok(pmc_data)
    }

    /// Save uploaded file to permanent storage
    pub async fn save_training_file(
        &self,
        file_data: Bytes,
        filename: &str,
        user_id: Uuid,
    ) -> Result<String> {
        // Create user-specific directory
        let user_dir = format!("{}/{}", self.upload_dir, user_id);
        fs::create_dir_all(&user_dir).await?;

        // Generate unique filename
        let file_extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        let unique_filename = format!("{}_{}.{}",
            chrono::Utc::now().timestamp(),
            uuid::Uuid::new_v4(),
            file_extension
        );

        let file_path = format!("{}/{}", user_dir, unique_filename);

        // Write file to disk
        let mut file = fs::File::create(&file_path).await?;
        file.write_all(&file_data).await?;
        file.flush().await?;

        info!("Saved training file to: {}", file_path);
        Ok(file_path)
    }

    /// Validate training file before processing
    pub fn validate_training_file(&self, file_data: &Bytes, filename: &str) -> Result<()> {
        // Check file size (max 50MB for safety, but allow reasonable large files)
        const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
        const MIN_FILE_SIZE: usize = 10; // At least 10 bytes

        if file_data.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File too large: {} bytes (max: {} bytes)",
                file_data.len(), MAX_FILE_SIZE));
        }

        if file_data.len() < MIN_FILE_SIZE {
            return Err(anyhow!("File too small: {} bytes (min: {} bytes)",
                file_data.len(), MIN_FILE_SIZE));
        }

        // Validate filename
        if filename.is_empty() {
            return Err(anyhow!("Filename is empty"));
        }

        // Check for potentially dangerous characters in filename
        if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
            return Err(anyhow!("Invalid filename: contains dangerous characters"));
        }

        // Validate file type
        let file_type = FileType::from_filename(filename)?;

        // Content-based validation
        if file_data.is_empty() {
            return Err(anyhow!("File is empty"));
        }

        // Basic format validation based on file type
        self.validate_file_format(file_data, &file_type)?;

        info!("File validation passed for: {}", filename);
        Ok(())
    }

    /// Validate file format based on content
    fn validate_file_format(&self, file_data: &Bytes, file_type: &FileType) -> Result<()> {
        let content = String::from_utf8_lossy(file_data);

        match file_type {
            FileType::Tcx => {
                // TCX files should be XML and contain TrainingCenterDatabase
                if !content.contains("<?xml") && !content.contains("<TrainingCenterDatabase") {
                    return Err(anyhow!("Invalid TCX file: missing XML header or TrainingCenterDatabase element"));
                }

                // Check for basic TCX structure
                if !content.contains("<Activities>") && !content.contains("<Activity") {
                    return Err(anyhow!("Invalid TCX file: missing Activities element"));
                }
            },
            FileType::Gpx => {
                // GPX files should be XML and contain gpx element
                if !content.contains("<?xml") && !content.contains("<gpx") {
                    return Err(anyhow!("Invalid GPX file: missing XML header or gpx element"));
                }

                // Check for basic GPX structure
                if !content.contains("<trk>") && !content.contains("<rte>") && !content.contains("<wpt>") {
                    return Err(anyhow!("Invalid GPX file: missing track, route, or waypoint data"));
                }
            },
            FileType::Csv => {
                // CSV files should have at least one comma or recognizable header
                let lines: Vec<&str> = content.lines().collect();
                if lines.is_empty() {
                    return Err(anyhow!("Invalid CSV file: no content"));
                }

                // Check if first line looks like a header
                let first_line = lines[0].to_lowercase();
                let has_recognizable_headers = first_line.contains("time") ||
                    first_line.contains("power") ||
                    first_line.contains("heart") ||
                    first_line.contains("speed") ||
                    first_line.contains("distance") ||
                    first_line.contains("cadence");

                if !has_recognizable_headers && !first_line.contains(",") {
                    return Err(anyhow!("Invalid CSV file: no recognizable format"));
                }
            }
        }

        Ok(())
    }

    /// Enhanced error handling for file processing
    fn handle_processing_error(&self, error: anyhow::Error, filename: &str) -> anyhow::Error {
        error!("Error processing file {}: {}", filename, error);

        // Categorize errors for better user feedback
        let error_message = if error.to_string().contains("XML") {
            format!("Failed to parse XML in file '{}': {}", filename, error)
        } else if error.to_string().contains("parse") {
            format!("Failed to parse data in file '{}': {}", filename, error)
        } else if error.to_string().contains("permission") {
            format!("Permission denied accessing file '{}': {}", filename, error)
        } else if error.to_string().contains("not found") {
            format!("File '{}' not found: {}", filename, error)
        } else {
            format!("Error processing file '{}': {}", filename, error)
        };

        anyhow!(error_message)
    }

    // Private helper methods

    async fn save_temp_file(&self, file_data: &Bytes, filename: &str) -> Result<String> {
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(file_data).await?;
        file.flush().await?;

        Ok(temp_path)
    }

    async fn process_tcx_file(
        &self,
        file_path: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        info!("Processing TCX file: {}", file_path);

        // Read file content with error handling
        let file_content = fs::read_to_string(file_path).await
            .map_err(|e| self.handle_processing_error(anyhow!("Failed to read TCX file: {}", e), file_path))?;

        // Parse TCX content with error handling
        let metrics = match self.parse_tcx_content(&file_content, zone_settings).await {
            Ok(metrics) => metrics,
            Err(e) => return Err(self.handle_processing_error(e, file_path)),
        };

        info!("Successfully processed TCX file with metrics: duration={:?}s, distance={:?}m",
              metrics.duration_seconds, metrics.distance_meters);

        Ok(metrics)
    }

    async fn process_gpx_file(
        &self,
        file_path: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        info!("Processing GPX file: {}", file_path);

        // Read file content with error handling
        let file_content = fs::read_to_string(file_path).await
            .map_err(|e| self.handle_processing_error(anyhow!("Failed to read GPX file: {}", e), file_path))?;

        // Parse GPX content with error handling
        let metrics = match self.parse_gpx_content(&file_content, zone_settings).await {
            Ok(metrics) => metrics,
            Err(e) => return Err(self.handle_processing_error(e, file_path)),
        };

        info!("Successfully processed GPX file with metrics: duration={:?}s, distance={:?}m",
              metrics.duration_seconds, metrics.distance_meters);

        Ok(metrics)
    }

    async fn process_csv_file(
        &self,
        file_path: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        info!("Processing CSV file: {}", file_path);

        // Read file content with error handling
        let file_content = fs::read_to_string(file_path).await
            .map_err(|e| self.handle_processing_error(anyhow!("Failed to read CSV file: {}", e), file_path))?;

        // Parse CSV content with error handling
        let metrics = match self.parse_csv_content(&file_content, zone_settings).await {
            Ok(metrics) => metrics,
            Err(e) => return Err(self.handle_processing_error(e, file_path)),
        };

        info!("Successfully processed CSV file with metrics: duration={:?}s, distance={:?}m",
              metrics.duration_seconds, metrics.distance_meters);

        Ok(metrics)
    }

    async fn get_training_sessions_for_pmc(
        &self,
        user_id: Uuid,
        days: i32,
    ) -> Result<Vec<TrainingSession>> {
        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(days as i64);

        let sessions = sqlx::query_as!(
            TrainingSession,
            r#"
            SELECT id, user_id, date, trainrs_data, uploaded_file_path,
                   session_type, duration_seconds, distance_meters,
                   created_at, updated_at
            FROM training_sessions
            WHERE user_id = $1 AND date >= $2 AND date <= $3
            ORDER BY date ASC
            "#,
            user_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(sessions)
    }

    async fn compute_pmc_values(
        &self,
        sessions: Vec<TrainingSession>,
    ) -> Result<Vec<PerformanceManagementChart>> {
        // PMC calculation constants
        const CTL_TIME_CONSTANT: f64 = 42.0; // Chronic Training Load decay
        const ATL_TIME_CONSTANT: f64 = 7.0;  // Acute Training Load decay

        let mut pmc_data = Vec::new();
        let mut ctl = 0.0;
        let mut atl = 0.0;

        // Group sessions by date
        let mut sessions_by_date: HashMap<chrono::NaiveDate, Vec<&TrainingSession>> = HashMap::new();
        for session in &sessions {
            sessions_by_date.entry(session.date).or_default().push(session);
        }

        // Calculate PMC for each day
        let start_date = sessions.first().map(|s| s.date).unwrap_or_else(chrono::Utc::now().date_naive);
        let end_date = sessions.last().map(|s| s.date).unwrap_or_else(chrono::Utc::now().date_naive);

        let mut current_date = start_date;
        while current_date <= end_date {
            let daily_tss = if let Some(day_sessions) = sessions_by_date.get(&current_date) {
                day_sessions.iter()
                    .filter_map(|session| {
                        session.trainrs_data.as_ref()
                            .and_then(|data| data.get("tss"))
                            .and_then(|tss| tss.as_f64())
                    })
                    .sum::<f64>()
            } else {
                0.0
            };

            // Update CTL and ATL using exponential weighted moving average
            ctl = ctl + (daily_tss - ctl) * (1.0 - (-1.0 / CTL_TIME_CONSTANT).exp());
            atl = atl + (daily_tss - atl) * (1.0 - (-1.0 / ATL_TIME_CONSTANT).exp());

            let tsb = ctl - atl; // Training Stress Balance

            pmc_data.push(PerformanceManagementChart {
                ctl,
                atl,
                tsb,
                date: current_date,
            });

            current_date += chrono::Duration::days(1);
        }

        Ok(pmc_data)
    }

    async fn get_cached_pmc(
        &self,
        user_id: Uuid,
        days: i32,
    ) -> Result<Option<Vec<PerformanceManagementChart>>> {
        if let Some(redis_client) = &self.redis_client {
            let mut conn = redis_client.get_async_connection().await?;
            let cache_key = format!("pmc:{}:{}", user_id, days);

            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(pmc_data) = serde_json::from_str::<Vec<PerformanceManagementChart>>(&cached_data) {
                    info!("Retrieved PMC data from cache for user: {}", user_id);
                    return Ok(Some(pmc_data));
                }
            }
        }

        Ok(None)
    }

    async fn cache_pmc_data(
        &self,
        user_id: Uuid,
        days: i32,
        pmc_data: &[PerformanceManagementChart],
    ) -> Result<()> {
        if let Some(redis_client) = &self.redis_client {
            let mut conn = redis_client.get_async_connection().await?;
            let cache_key = format!("pmc:{}:{}", user_id, days);
            let cache_data = serde_json::to_string(pmc_data)?;

            // Cache for 1 hour
            let _: () = conn.setex(&cache_key, 3600, cache_data).await?;
            info!("Cached PMC data for user: {}", user_id);
        }

        Ok(())
    }

    // File parsing methods (placeholder implementations until trainrs integration)

    async fn parse_tcx_content(
        &self,
        content: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        // Basic TCX parsing - in production this would use trainrs
        // For now, extract basic information using regex

        let duration_regex = regex::Regex::new(r"<TotalTimeSeconds>([^<]+)</TotalTimeSeconds>")
            .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;
        let distance_regex = regex::Regex::new(r"<DistanceMeters>([^<]+)</DistanceMeters>")
            .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;

        let duration_seconds = duration_regex
            .captures(content)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .map(|d| d as i32);

        let distance_meters = distance_regex
            .captures(content)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok());

        // Extract trackpoints for more detailed analysis
        let trackpoints = self.extract_tcx_trackpoints(content)?;
        let calculated_metrics = self.calculate_metrics_from_trackpoints(&trackpoints, zone_settings)?;

        Ok(TrainingMetrics {
            duration_seconds,
            distance_meters,
            elevation_gain_meters: calculated_metrics.elevation_gain,
            average_power: calculated_metrics.avg_power,
            normalized_power: calculated_metrics.normalized_power,
            average_heart_rate: calculated_metrics.avg_heart_rate,
            average_cadence: calculated_metrics.avg_cadence,
            average_speed: calculated_metrics.avg_speed,
            tss: calculated_metrics.tss,
            intensity_factor: calculated_metrics.intensity_factor,
            work: calculated_metrics.work,
            power_zones: calculated_metrics.power_zones,
            heart_rate_zones: calculated_metrics.heart_rate_zones,
        })
    }

    async fn parse_gpx_content(
        &self,
        content: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        // Basic GPX parsing - in production this would use trainrs
        // GPX files typically contain GPS data but may lack power/HR data

        let trackpoints = self.extract_gpx_trackpoints(content)?;
        let calculated_metrics = self.calculate_metrics_from_trackpoints(&trackpoints, zone_settings)?;

        Ok(TrainingMetrics {
            duration_seconds: calculated_metrics.duration,
            distance_meters: calculated_metrics.distance,
            elevation_gain_meters: calculated_metrics.elevation_gain,
            average_power: calculated_metrics.avg_power,
            normalized_power: calculated_metrics.normalized_power,
            average_heart_rate: calculated_metrics.avg_heart_rate,
            average_cadence: calculated_metrics.avg_cadence,
            average_speed: calculated_metrics.avg_speed,
            tss: calculated_metrics.tss,
            intensity_factor: calculated_metrics.intensity_factor,
            work: calculated_metrics.work,
            power_zones: calculated_metrics.power_zones,
            heart_rate_zones: calculated_metrics.heart_rate_zones,
        })
    }

    async fn parse_csv_content(
        &self,
        content: &str,
        zone_settings: Option<ZoneSettings>,
    ) -> Result<TrainingMetrics> {
        // Basic CSV parsing - in production this would use trainrs
        // CSV format varies by device, need to detect column structure

        let trackpoints = self.extract_csv_trackpoints(content)?;
        let calculated_metrics = self.calculate_metrics_from_trackpoints(&trackpoints, zone_settings)?;

        Ok(TrainingMetrics {
            duration_seconds: calculated_metrics.duration,
            distance_meters: calculated_metrics.distance,
            elevation_gain_meters: calculated_metrics.elevation_gain,
            average_power: calculated_metrics.avg_power,
            normalized_power: calculated_metrics.normalized_power,
            average_heart_rate: calculated_metrics.avg_heart_rate,
            average_cadence: calculated_metrics.avg_cadence,
            average_speed: calculated_metrics.avg_speed,
            tss: calculated_metrics.tss,
            intensity_factor: calculated_metrics.intensity_factor,
            work: calculated_metrics.work,
            power_zones: calculated_metrics.power_zones,
            heart_rate_zones: calculated_metrics.heart_rate_zones,
        })
    }

    fn extract_tcx_trackpoints(&self, _content: &str) -> Result<Vec<TrackPoint>> {
        // Placeholder implementation - would use proper XML parsing in production
        let trackpoints = Vec::new();

        // This is a very basic implementation
        // In production, we would use an XML parser and trainrs
        warn!("Using placeholder TCX trackpoint extraction");

        Ok(trackpoints)
    }

    fn extract_gpx_trackpoints(&self, _content: &str) -> Result<Vec<TrackPoint>> {
        // Placeholder implementation - would use proper XML parsing in production
        let trackpoints = Vec::new();

        // This is a very basic implementation
        // In production, we would use an XML parser and trainrs
        warn!("Using placeholder GPX trackpoint extraction");

        Ok(trackpoints)
    }

    fn extract_csv_trackpoints(&self, _content: &str) -> Result<Vec<TrackPoint>> {
        // Placeholder implementation - would use proper CSV parsing in production
        let trackpoints = Vec::new();

        // This is a very basic implementation
        // In production, we would parse CSV headers and data properly
        warn!("Using placeholder CSV trackpoint extraction");

        Ok(trackpoints)
    }

    fn calculate_metrics_from_trackpoints(
        &self,
        _trackpoints: &[TrackPoint],
        _zone_settings: Option<ZoneSettings>,
    ) -> Result<CalculatedMetrics> {
        // Placeholder implementation for metrics calculation
        // In production, this would implement proper power analysis,
        // normalized power calculation, TSS calculation, etc.

        warn!("Using placeholder metrics calculation");

        Ok(CalculatedMetrics {
            duration: None,
            distance: None,
            elevation_gain: None,
            avg_power: None,
            normalized_power: None,
            avg_heart_rate: None,
            avg_cadence: None,
            avg_speed: None,
            tss: None,
            intensity_factor: None,
            work: None,
            power_zones: None,
            heart_rate_zones: None,
        })
    }
}

// Helper structs for trackpoint data
#[derive(Debug, Clone)]
struct TrackPoint {
    timestamp: chrono::DateTime<chrono::Utc>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    elevation: Option<f64>,
    heart_rate: Option<f64>,
    power: Option<f64>,
    cadence: Option<f64>,
    speed: Option<f64>,
    distance: Option<f64>,
}

#[derive(Debug, Clone)]
struct CalculatedMetrics {
    duration: Option<i32>,
    distance: Option<f64>,
    elevation_gain: Option<f64>,
    avg_power: Option<f64>,
    normalized_power: Option<f64>,
    avg_heart_rate: Option<f64>,
    avg_cadence: Option<f64>,
    avg_speed: Option<f64>,
    tss: Option<f64>,
    intensity_factor: Option<f64>,
    work: Option<f64>,
    power_zones: Option<HashMap<String, f64>>,
    heart_rate_zones: Option<HashMap<String, f64>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use bytes::Bytes;
    use sqlx::PgPool;
    use std::env;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;
    use uuid::Uuid;

    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/ai_coach_test".to_string());

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    fn create_test_service(db: PgPool) -> TrainingAnalysisService {
        TrainingAnalysisService::new(db, None)
            .expect("Failed to create TrainingAnalysisService")
    }

    #[tokio::test]
    async fn test_file_type_from_filename() {
        assert!(matches!(FileType::from_filename("test.tcx"), Ok(FileType::Tcx)));
        assert!(matches!(FileType::from_filename("test.gpx"), Ok(FileType::Gpx)));
        assert!(matches!(FileType::from_filename("test.csv"), Ok(FileType::Csv)));
        assert!(matches!(FileType::from_filename("TEST.TCX"), Ok(FileType::Tcx)));
        assert!(FileType::from_filename("test.txt").is_err());
        assert!(FileType::from_filename("test").is_err());
    }

    #[tokio::test]
    async fn test_validate_training_file() {
        let db = setup_test_db().await;
        let service = create_test_service(db);

        // Test valid TCX file
        let tcx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase>
    <Activities>
        <Activity Sport="Biking">
            <Id>2023-01-01T10:00:00Z</Id>
        </Activity>
    </Activities>
</TrainingCenterDatabase>"#;
        let tcx_bytes = Bytes::from(tcx_content);
        assert!(service.validate_training_file(&tcx_bytes, "test.tcx").is_ok());

        // Test file too large
        let large_content = "x".repeat(60 * 1024 * 1024); // 60MB
        let large_bytes = Bytes::from(large_content);
        assert!(service.validate_training_file(&large_bytes, "test.tcx").is_err());

        // Test dangerous filename
        assert!(service.validate_training_file(&tcx_bytes, "../test.tcx").is_err());
    }

    #[tokio::test]
    async fn test_training_metrics_calculate_tss() {
        let zone_settings = crate::models::ZoneSettings {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            ftp: Some(250.0),
            lthr: Some(165.0),
            max_heart_rate: Some(190.0),
            resting_heart_rate: Some(60.0),
            threshold_pace: None,
            weight: Some(70.0),
            power_zones: None,
            heart_rate_zones: None,
            pace_zones: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let metrics = TrainingMetrics {
            duration_seconds: Some(3600), // 1 hour
            distance_meters: Some(40000.0),
            elevation_gain_meters: Some(500.0),
            average_power: Some(200.0),
            normalized_power: Some(220.0),
            average_heart_rate: Some(150.0),
            average_cadence: Some(90.0),
            average_speed: Some(11.1),
            tss: None,
            intensity_factor: None,
            work: Some(800.0),
            power_zones: None,
            heart_rate_zones: None,
        };

        // Test power-based TSS calculation
        let tss = metrics.calculate_tss(&zone_settings);
        assert!(tss.is_some());
        let tss_value = tss.unwrap();

        // TSS = (duration_hours) * (normalized_power / ftp)^2 * 100
        // Expected: 1 * (220/250)^2 * 100 = 77.44
        assert!((tss_value - 77.44).abs() < 0.1);
    }
}