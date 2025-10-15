// Configuration management

pub mod app;
pub mod database;
// pub mod seeding;  // MVP: Disabled - references disabled services

pub use app::AppConfig;
pub use database::{DatabaseConfig, run_migrations};
// pub use seeding::DatabaseSeeder;
