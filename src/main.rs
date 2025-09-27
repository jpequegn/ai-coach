use ai_coach::api::routes::create_routes;
use tokio::net::TcpListener;
use tracing::{info, instrument};
use tracing_subscriber;

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create the application routes
    let app = create_routes();

    // Start the server
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    info!("AI Coach server starting on http://0.0.0.0:3000");
    info!("Health check available at http://0.0.0.0:3000/health");

    axum::serve(listener, app).await?;

    Ok(())
}
