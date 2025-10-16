// ============================================================================
// MVP Main - Simplified startup without background jobs
// ============================================================================
use ai_coach_api::api::routes::create_routes;
use ai_coach_api::config::{AppConfig, DatabaseConfig, run_migrations};
// Disabled for MVP - background jobs not needed
// use ai_coach::services::{
//     AlertDeliveryJob, AlertDeliveryQueueService, DailyRecoveryCalculationJob,
//     DataQualityCheckJob, JobRegistry, NotificationService, RecoveryAlertService,
//     RecoveryAnalysisService, RecoveryDataService, RecoveryJobScheduler,
//     WeeklyBaselineRecalculationJob,
// };
// use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn, instrument};
use tracing_subscriber;

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting AI Coach API - MVP Mode");
    warn!("⚠️  MVP Mode: Background jobs, ML features, and recovery tracking disabled");

    // Load configuration
    let app_config = AppConfig::from_env()?;
    let db_config = DatabaseConfig::from_env()?;

    // Create database connection pool (SQLite)
    info!("📦 Connecting to SQLite database...");
    let db = db_config.create_pool().await?;
    info!("✅ Database connected");

    // Run migrations
    info!("🔄 Running database migrations...");
    run_migrations(&db).await?;
    info!("✅ Migrations complete");

    // MVP: Skip background jobs entirely
    // Production version would start RecoveryJobScheduler, DailyRecoveryCalculationJob, etc.

    // Create the application routes (MVP configuration)
    let app = create_routes(
        db,
        &app_config.jwt_secret,
        &app_config,
        None,  // No scheduler in MVP
        None,  // No recovery job in MVP
    );

    // Start the server
    let addr = app_config.server_address();
    let listener = TcpListener::bind(&addr).await?;

    info!("✅ AI Coach API (MVP) ready!");
    info!("🌐 Server: http://{}", addr);
    info!("❤️  Health: http://{}/health", addr);
    info!("🔐 Auth: http://{}/api/v1/auth/*", addr);
    info!("👤 Profile: http://{}/api/v1/user/*", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
