use anyhow::{Context, Result};
use aws_sdk_s3::{
    primitives::ByteStream,
    types::{Delete, ObjectIdentifier},
    Client as S3Client,
};
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

/// Service for managing video storage in S3-compatible object storage
pub struct VideoStorageService {
    client: S3Client,
    bucket_name: String,
    url_expiry_seconds: u64,
}

impl VideoStorageService {
    /// Create a new VideoStorageService
    pub fn new(client: S3Client, bucket_name: String) -> Self {
        Self {
            client,
            bucket_name,
            url_expiry_seconds: 3600, // 1 hour default
        }
    }

    /// Upload video file to S3-compatible storage
    /// Returns the storage key (path) for the uploaded file
    pub async fn upload_video(
        &self,
        user_id: Uuid,
        analysis_id: Uuid,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> Result<String> {
        let storage_key = self.generate_storage_key(user_id, analysis_id, content_type);

        info!(
            "Uploading video to storage: bucket={}, key={}, size={}",
            self.bucket_name,
            storage_key,
            file_data.len()
        );

        let body = ByteStream::from(file_data);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&storage_key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .context("Failed to upload video to S3")?;

        info!("Successfully uploaded video: {}", storage_key);
        Ok(storage_key)
    }

    /// Generate a presigned URL for secure video access
    pub async fn generate_presigned_url(&self, storage_key: &str) -> Result<String> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
            .expires_in(Duration::from_secs(self.url_expiry_seconds))
            .build()
            .context("Failed to build presigning config")?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(storage_key)
            .presigned(presigning_config)
            .await
            .context("Failed to generate presigned URL")?;

        Ok(presigned_request.uri().to_string())
    }

    /// Delete video from storage
    pub async fn delete_video(&self, storage_key: &str) -> Result<()> {
        info!("Deleting video from storage: {}", storage_key);

        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(storage_key)
            .send()
            .await
            .context("Failed to delete video from S3")?;

        info!("Successfully deleted video: {}", storage_key);
        Ok(())
    }

    /// Delete multiple videos in batch
    pub async fn delete_videos_batch(&self, storage_keys: Vec<String>) -> Result<()> {
        if storage_keys.is_empty() {
            return Ok(());
        }

        info!("Batch deleting {} videos", storage_keys.len());

        let objects: Vec<ObjectIdentifier> = storage_keys
            .iter()
            .map(|key| ObjectIdentifier::builder().key(key).build().unwrap())
            .collect();

        let delete = Delete::builder().set_objects(Some(objects)).build()?;

        self.client
            .delete_objects()
            .bucket(&self.bucket_name)
            .delete(delete)
            .send()
            .await
            .context("Failed to batch delete videos from S3")?;

        info!("Successfully batch deleted videos");
        Ok(())
    }

    /// Download video from storage
    pub async fn download_video(&self, storage_key: &str) -> Result<Vec<u8>> {
        info!("Downloading video from storage: {}", storage_key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(storage_key)
            .send()
            .await
            .context("Failed to download video from S3")?;

        let data = response
            .body
            .collect()
            .await
            .context("Failed to collect video data")?
            .into_bytes()
            .to_vec();

        info!("Successfully downloaded video: {} bytes", data.len());
        Ok(data)
    }

    /// Check if video exists in storage
    pub async fn video_exists(&self, storage_key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(storage_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(anyhow::anyhow!("Failed to check video existence: {}", e))
                }
            }
        }
    }

    /// Get video metadata (size, content type, etc.)
    pub async fn get_video_metadata(&self, storage_key: &str) -> Result<VideoMetadata> {
        let response = self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(storage_key)
            .send()
            .await
            .context("Failed to get video metadata")?;

        Ok(VideoMetadata {
            size_bytes: response.content_length().unwrap_or(0),
            content_type: response.content_type().map(|s| s.to_string()),
            last_modified: response.last_modified().map(|dt| dt.to_string()),
        })
    }

    /// Generate storage key based on user and analysis IDs
    fn generate_storage_key(&self, user_id: Uuid, analysis_id: Uuid, content_type: &str) -> String {
        let extension = self.extract_file_extension(content_type);
        format!("videos/{}/{}.{}", user_id, analysis_id, extension)
    }

    /// Extract file extension from content type
    fn extract_file_extension(&self, content_type: &str) -> &str {
        match content_type {
            "video/mp4" => "mp4",
            "video/quicktime" => "mov",
            "video/x-msvideo" => "avi",
            "video/webm" => "webm",
            "video/x-matroska" => "mkv",
            _ => "mp4", // Default to mp4
        }
    }

    /// Set custom URL expiry time
    pub fn set_url_expiry_seconds(&mut self, seconds: u64) {
        self.url_expiry_seconds = seconds;
    }
}

/// Video metadata from storage
#[derive(Debug)]
pub struct VideoMetadata {
    pub size_bytes: i64,
    pub content_type: Option<String>,
    pub last_modified: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_storage_key() {
        let mock_config = aws_config::SdkConfig::builder().build();
        let client = S3Client::new(&mock_config);
        let service = VideoStorageService::new(client, "test-bucket".to_string());

        let user_id = Uuid::new_v4();
        let analysis_id = Uuid::new_v4();
        let key = service.generate_storage_key(user_id, analysis_id, "video/mp4");

        assert!(key.starts_with("videos/"));
        assert!(key.contains(&user_id.to_string()));
        assert!(key.contains(&analysis_id.to_string()));
        assert!(key.ends_with(".mp4"));
    }

    #[test]
    fn test_extract_file_extension() {
        let mock_config = aws_config::SdkConfig::builder().build();
        let client = S3Client::new(&mock_config);
        let service = VideoStorageService::new(client, "test-bucket".to_string());

        assert_eq!(service.extract_file_extension("video/mp4"), "mp4");
        assert_eq!(service.extract_file_extension("video/quicktime"), "mov");
        assert_eq!(service.extract_file_extension("video/webm"), "webm");
        assert_eq!(service.extract_file_extension("unknown"), "mp4");
    }
}
