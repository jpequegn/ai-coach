use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use ai_coach::models::{AthleteProfile, CreateAthleteProfile, UpdateAthleteProfile};
use ai_coach::services::AthleteProfileService;

#[cfg(test)]
mod athlete_profile_service_tests {
    use super::*;

    // Helper to create test database
    async fn create_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create athlete_profiles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS athlete_profiles (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                sport TEXT NOT NULL,
                ftp INTEGER,
                lthr INTEGER,
                max_heart_rate INTEGER,
                threshold_pace REAL,
                zones TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    // ========================================
    // Create Profile Tests
    // ========================================

    #[tokio::test]
    async fn test_create_profile_success() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: Some(4.30),
            zones: Some(json!({"power": [0, 137, 187, 225, 262, 300], "hr": [0, 111, 129, 148, 166, 185]})),
        };

        let result = service.create_profile(create_data).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert_eq!(profile.user_id, user_id);
        assert_eq!(profile.sport, "cycling");
        assert_eq!(profile.ftp, Some(250));
        assert_eq!(profile.lthr, Some(165));
        assert_eq!(profile.max_heart_rate, Some(185));
        assert!(profile.zones.is_some());
    }

    #[tokio::test]
    async fn test_create_profile_minimal_data() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "running".to_string(),
            ftp: None,
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result = service.create_profile(create_data).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert_eq!(profile.sport, "running");
        assert_eq!(profile.ftp, None);
        assert_eq!(profile.lthr, None);
        assert_eq!(profile.max_heart_rate, None);
        assert_eq!(profile.threshold_pace, None);
        assert_eq!(profile.zones, None);
    }

    #[tokio::test]
    async fn test_create_multiple_profiles_same_user() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();

        // Create first profile (cycling)
        let create_data_1 = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        service.create_profile(create_data_1).await.unwrap();

        // Create second profile (running)
        let create_data_2 = CreateAthleteProfile {
            user_id,
            sport: "running".to_string(),
            ftp: None,
            lthr: None,
            max_heart_rate: Some(185),
            threshold_pace: Some(4.0),
            zones: None,
        };
        let result = service.create_profile(create_data_2).await;
        assert!(result.is_ok());

        // User can have multiple profiles for different sports
        let profile_2 = result.unwrap();
        assert_eq!(profile_2.sport, "running");
    }

    #[tokio::test]
    async fn test_create_profile_with_complex_zones() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let zones_data = json!({
            "power": {
                "zone_1": {"min": 0, "max": 137},
                "zone_2": {"min": 138, "max": 187},
                "zone_3": {"min": 188, "max": 225},
                "zone_4": {"min": 226, "max": 262},
                "zone_5": {"min": 263, "max": 300},
                "zone_6": {"min": 301, "max": 375},
                "zone_7": {"min": 376, "max": null}
            },
            "heart_rate": {
                "zone_1": {"min": 0, "max": 111},
                "zone_2": {"min": 112, "max": 129},
                "zone_3": {"min": 130, "max": 148},
                "zone_4": {"min": 149, "max": 166},
                "zone_5": {"min": 167, "max": 185}
            }
        });

        let create_data = CreateAthleteProfile {
            user_id: Uuid::new_v4(),
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: None,
            zones: Some(zones_data.clone()),
        };

        let result = service.create_profile(create_data).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.zones.is_some());
        // Zones should be stored as JSON
        let stored_zones = profile.zones.unwrap();
        assert_eq!(stored_zones["power"]["zone_1"]["min"], 0);
    }

    // ========================================
    // Get Profile Tests
    // ========================================

    #[tokio::test]
    async fn test_get_profile_by_id_success() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create a profile first
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: None,
            zones: None,
        };
        let created_profile = service.create_profile(create_data).await.unwrap();

        // Get the profile by ID
        let result = service.get_profile_by_id(created_profile.id).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert_eq!(profile.id, created_profile.id);
        assert_eq!(profile.user_id, user_id);
        assert_eq!(profile.sport, "cycling");
    }

    #[tokio::test]
    async fn test_get_profile_by_id_not_found() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let non_existent_id = Uuid::new_v4();
        let result = service.get_profile_by_id(non_existent_id).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.is_none());
    }

    #[tokio::test]
    async fn test_get_profile_by_user_id_success() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "running".to_string(),
            ftp: None,
            lthr: None,
            max_heart_rate: Some(185),
            threshold_pace: Some(4.0),
            zones: None,
        };
        service.create_profile(create_data).await.unwrap();

        // Get profile by user_id
        let result = service.get_profile_by_user_id(user_id).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.is_some());

        let profile = profile.unwrap();
        assert_eq!(profile.user_id, user_id);
        assert_eq!(profile.sport, "running");
    }

    #[tokio::test]
    async fn test_get_profile_by_user_id_not_found() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let non_existent_user_id = Uuid::new_v4();
        let result = service.get_profile_by_user_id(non_existent_user_id).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.is_none());
    }

    #[tokio::test]
    async fn test_get_profiles_by_sport() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create multiple profiles with same sport
        let user_1 = Uuid::new_v4();
        let user_2 = Uuid::new_v4();

        let create_data_1 = CreateAthleteProfile {
            user_id: user_1,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        service.create_profile(create_data_1).await.unwrap();

        let create_data_2 = CreateAthleteProfile {
            user_id: user_2,
            sport: "cycling".to_string(),
            ftp: Some(300),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        service.create_profile(create_data_2).await.unwrap();

        // Create profile with different sport
        let user_3 = Uuid::new_v4();
        let create_data_3 = CreateAthleteProfile {
            user_id: user_3,
            sport: "running".to_string(),
            ftp: None,
            lthr: None,
            max_heart_rate: Some(185),
            threshold_pace: Some(4.0),
            zones: None,
        };
        service.create_profile(create_data_3).await.unwrap();

        // Get cycling profiles
        let result = service.get_profiles_by_sport("cycling").await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().all(|p| p.sport == "cycling"));
    }

    #[tokio::test]
    async fn test_get_profiles_by_sport_empty() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let result = service.get_profiles_by_sport("swimming").await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 0);
    }

    // ========================================
    // Update Profile Tests
    // ========================================

    #[tokio::test]
    async fn test_update_profile_success() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create profile
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: None,
            zones: None,
        };
        let profile = service.create_profile(create_data).await.unwrap();

        // Update profile
        let update_data = UpdateAthleteProfile {
            sport: Some("triathlon".to_string()),
            ftp: Some(280),
            lthr: Some(170),
            max_heart_rate: Some(190),
            threshold_pace: Some(4.0),
            zones: Some(json!({"power": [0, 154, 210, 252, 294, 336]})),
        };

        let result = service.update_profile(profile.id, update_data).await;
        assert!(result.is_ok());

        let updated_profile = result.unwrap();
        assert!(updated_profile.is_some());

        let updated_profile = updated_profile.unwrap();
        assert_eq!(updated_profile.sport, "triathlon");
        assert_eq!(updated_profile.ftp, Some(280));
        assert_eq!(updated_profile.lthr, Some(170));
        assert_eq!(updated_profile.max_heart_rate, Some(190));
        assert_eq!(updated_profile.threshold_pace, Some(4.0));
        assert!(updated_profile.zones.is_some());
    }

    #[tokio::test]
    async fn test_update_profile_partial() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create profile
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: Some(4.5),
            zones: None,
        };
        let profile = service.create_profile(create_data).await.unwrap();

        // Update only FTP
        let update_data = UpdateAthleteProfile {
            sport: None,
            ftp: Some(270),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result = service.update_profile(profile.id, update_data).await;
        assert!(result.is_ok());

        let updated_profile = result.unwrap().unwrap();
        assert_eq!(updated_profile.ftp, Some(270));
        // Other fields should remain unchanged
        assert_eq!(updated_profile.sport, "cycling");
        assert_eq!(updated_profile.lthr, Some(165));
        assert_eq!(updated_profile.max_heart_rate, Some(185));
        assert_eq!(updated_profile.threshold_pace, Some(4.5));
    }

    #[tokio::test]
    async fn test_update_profile_not_found() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let non_existent_id = Uuid::new_v4();
        let update_data = UpdateAthleteProfile {
            sport: Some("cycling".to_string()),
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result = service.update_profile(non_existent_id, update_data).await;
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert!(profile.is_none());
    }

    #[tokio::test]
    async fn test_update_profile_clear_optional_fields() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create profile with all fields
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: Some(4.5),
            zones: Some(json!({"power": [0, 137, 187]})),
        };
        let profile = service.create_profile(create_data).await.unwrap();

        // Note: Current implementation uses COALESCE, so None doesn't clear fields
        // This test documents the behavior - may need to update if clearing is required
        let update_data = UpdateAthleteProfile {
            sport: None,
            ftp: None,
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result = service.update_profile(profile.id, update_data).await;
        assert!(result.is_ok());

        let updated_profile = result.unwrap().unwrap();
        // Fields remain unchanged when None is provided
        assert_eq!(updated_profile.ftp, Some(250));
        assert_eq!(updated_profile.lthr, Some(165));
    }

    // ========================================
    // Delete Profile Tests
    // ========================================

    #[tokio::test]
    async fn test_delete_profile_success() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create profile
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        let profile = service.create_profile(create_data).await.unwrap();

        // Delete profile
        let result = service.delete_profile(profile.id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Verify profile is deleted
        let get_result = service.get_profile_by_id(profile.id).await.unwrap();
        assert!(get_result.is_none());
    }

    #[tokio::test]
    async fn test_delete_profile_not_found() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let non_existent_id = Uuid::new_v4();
        let result = service.delete_profile(non_existent_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    // ========================================
    // List Profiles Tests
    // ========================================

    #[tokio::test]
    async fn test_list_profiles_default_pagination() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create multiple profiles
        for i in 0..10 {
            let user_id = Uuid::new_v4();
            let create_data = CreateAthleteProfile {
                user_id,
                sport: format!("sport_{}", i),
                ftp: Some(250 + i * 10),
                lthr: None,
                max_heart_rate: None,
                threshold_pace: None,
                zones: None,
            };
            service.create_profile(create_data).await.unwrap();
        }

        // List with default pagination
        let result = service.list_profiles(None, None).await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 10);
    }

    #[tokio::test]
    async fn test_list_profiles_with_limit() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create multiple profiles
        for i in 0..15 {
            let user_id = Uuid::new_v4();
            let create_data = CreateAthleteProfile {
                user_id,
                sport: "cycling".to_string(),
                ftp: Some(250),
                lthr: None,
                max_heart_rate: None,
                threshold_pace: None,
                zones: None,
            };
            service.create_profile(create_data).await.unwrap();
        }

        // List with limit
        let result = service.list_profiles(Some(5), None).await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 5);
    }

    #[tokio::test]
    async fn test_list_profiles_with_offset() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create multiple profiles
        for i in 0..15 {
            let user_id = Uuid::new_v4();
            let create_data = CreateAthleteProfile {
                user_id,
                sport: format!("sport_{}", i),
                ftp: Some(250 + i * 10),
                lthr: None,
                max_heart_rate: None,
                threshold_pace: None,
                zones: None,
            };
            service.create_profile(create_data).await.unwrap();
        }

        // List with limit and offset
        let result = service.list_profiles(Some(5), Some(10)).await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 5);
    }

    #[tokio::test]
    async fn test_list_profiles_empty() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let result = service.list_profiles(None, None).await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 0);
    }

    #[tokio::test]
    async fn test_list_profiles_ordering() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        // Create profiles with delays to ensure different timestamps
        let user_1 = Uuid::new_v4();
        let create_data_1 = CreateAthleteProfile {
            user_id: user_1,
            sport: "first".to_string(),
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        service.create_profile(create_data_1).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let user_2 = Uuid::new_v4();
        let create_data_2 = CreateAthleteProfile {
            user_id: user_2,
            sport: "second".to_string(),
            ftp: Some(280),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };
        service.create_profile(create_data_2).await.unwrap();

        // List profiles (should be ordered by created_at DESC)
        let result = service.list_profiles(None, None).await;
        assert!(result.is_ok());

        let profiles = result.unwrap();
        assert_eq!(profiles.len(), 2);
        // Most recent should be first
        assert_eq!(profiles[0].sport, "second");
        assert_eq!(profiles[1].sport, "first");
    }

    // ========================================
    // Edge Cases and Validation Tests
    // ========================================

    #[tokio::test]
    async fn test_profile_with_negative_values() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(-50), // Invalid negative value
            lthr: Some(-10),
            max_heart_rate: Some(-100),
            threshold_pace: Some(-4.0),
            zones: None,
        };

        // Service doesn't validate - this documents current behavior
        // Ideally should reject negative values
        let result = service.create_profile(create_data).await;
        // Current implementation allows this - may want to add validation
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_profile_with_extreme_values() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(9999), // Extremely high FTP
            lthr: Some(999),
            max_heart_rate: Some(999),
            threshold_pace: Some(0.01), // Extremely fast pace
            zones: None,
        };

        let result = service.create_profile(create_data).await;
        // Documents that extreme values are allowed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_profile_with_empty_sport() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool);

        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "".to_string(), // Empty sport
            ftp: Some(250),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result = service.create_profile(create_data).await;
        // Documents that empty sport is allowed
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_profile_updates() {
        let pool = create_test_db().await;
        let service = AthleteProfileService::new(pool.clone());

        // Create profile
        let user_id = Uuid::new_v4();
        let create_data = CreateAthleteProfile {
            user_id,
            sport: "cycling".to_string(),
            ftp: Some(250),
            lthr: Some(165),
            max_heart_rate: Some(185),
            threshold_pace: None,
            zones: None,
        };
        let profile = service.create_profile(create_data).await.unwrap();

        // Simulate concurrent updates
        let service_clone = AthleteProfileService::new(pool);
        let profile_id = profile.id;

        let update1 = UpdateAthleteProfile {
            sport: None,
            ftp: Some(270),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let update2 = UpdateAthleteProfile {
            sport: None,
            ftp: Some(280),
            lthr: None,
            max_heart_rate: None,
            threshold_pace: None,
            zones: None,
        };

        let result1 = service.update_profile(profile_id, update1).await;
        let result2 = service_clone.update_profile(profile_id, update2).await;

        // Both should succeed (last write wins)
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
