use ai_coach::api::routes::create_routes;
use ai_coach::config::{AppConfig, DatabaseConfig, run_migrations};
use ai_coach::services::RecoveryJobScheduler;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, instrument};
use tracing_subscriber;

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let app_config = AppConfig::from_env()?;
    let db_config = DatabaseConfig::from_env()?;

    // Create database connection pool
    let db = db_config.create_pool().await?;

    // Run migrations
    run_migrations(&db).await?;

    // Create and start the recovery job scheduler
    let scheduler = Arc::new(RecoveryJobScheduler::new(db.clone()).await?);
    scheduler.start().await?;
    info!("Recovery job scheduler started");

    // Create the application routes
    let app = create_routes(db, &app_config.jwt_secret, &app_config, Some(scheduler.clone()));

    // Start the server
    let listener = TcpListener::bind(&app_config.server_address()).await?;
    info!("AI Coach server starting on http://{}", app_config.server_address());
    info!("Health check available at http://{}/health", app_config.server_address());
    info!("Authentication endpoints available at http://{}/api/auth", app_config.server_address());

    axum::serve(listener, app).await?;

    Ok(())
}
