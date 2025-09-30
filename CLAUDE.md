# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AI Coach is a Rust-based REST API for AI-powered athletic coaching, featuring machine learning models for training predictions, workout recommendations, and performance insights. Built with Axum web framework, PostgreSQL database, and Linfa ML framework.

## Core Architecture

### Three-Layer Design Pattern

1. **API Layer** (`src/api/`): Axum HTTP handlers and route definitions
2. **Service Layer** (`src/services/`): Business logic and orchestration
3. **Model Layer** (`src/models/`): Data structures and database entities

### Key Architectural Concepts

- **Database-First**: All data models map to PostgreSQL tables via SQLx migrations
- **JWT Authentication**: Token-based auth with refresh tokens and role-based access control
- **ML Pipeline**: Feature engineering → model training → predictions → recommendations
- **Background Jobs**: Redis-backed async processing with tokio-cron-scheduler

### Module Organization

```
src/
├── api/              # HTTP handlers (16 route modules)
├── services/         # Business logic (22 service modules)
├── models/           # Data structures (16 model modules)
├── auth/             # Authentication system (JWT, roles, tokens)
├── config/           # App configuration (env, database, seeding)
└── middleware/       # HTTP middleware (auth, logging, CORS)
```

## Development Commands

### Database Setup

```bash
# Start PostgreSQL via Docker
docker-compose up -d db

# Set database URL (required)
export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach

# Run migrations (automatic on server start)
cargo run  # Migrations run automatically

# Seed database with test data
cargo test seed_database --test database_integration_test
```

### Running the Application

```bash
# Development with hot reload
cargo watch -x run

# Standard run
cargo run

# With custom logging
RUST_LOG=debug cargo run

# Docker Compose (app + database)
docker-compose up --build
```

Server starts on `http://localhost:3000`

### Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_test          # API integration tests
cargo test --test database_integration_test # Database tests
cargo test --test security_testing          # Security tests
cargo test --test load_testing              # Performance tests

# Run unit tests only
cargo test --lib

# Run single test
cargo test test_user_creation -- --exact

# Run with output
cargo test -- --nocapture

# Run tests serially (for database tests)
cargo test -- --test-threads=1
```

### Code Quality

```bash
# Check compilation without building
cargo check

# Lint with Clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

## Testing Architecture

### Test Organization

- `tests/unit/`: Service-level unit tests with mocks
- `tests/integration/`: API endpoint integration tests
- `tests/common/`: Shared test utilities and fixtures
- Root test files: Specialized test suites (load, security, ML validation)

### Key Test Patterns

**Database Tests**: Use `#[sqlx::test]` macro for automatic transaction rollback
```rust
#[sqlx::test]
async fn test_user_creation(pool: PgPool) -> sqlx::Result<()> {
    // Test code with automatic rollback
}
```

**API Tests**: Use `common::test_helpers::TestApp` for server setup
```rust
let app = TestApp::spawn().await;
let response = app.client.get("/api/v1/health").send().await?;
```

**Mock Data**: Use `fake` crate for realistic test data generation

## Database Migrations

Located in `migrations/` directory with sequential numbering:

- `001-007`: Core tables (users, profiles, sessions, recommendations, plans, predictions)
- `008-012`: Authentication system (roles, tokens, blacklist)
- `013-016`: Extended features (goals, notifications, events, analytics)

**Migration Pattern**: Each file is `NNN_description.sql` where NNN is sequential

## API Endpoints Structure

### Authentication Routes (`/api/v1/auth`)
- `POST /register` - User registration
- `POST /login` - User login with JWT
- `POST /logout` - Token invalidation
- `POST /refresh` - Refresh access token
- `POST /forgot-password` - Password reset request
- `POST /reset-password` - Complete password reset

### Training Routes (`/api/v1/training`)
- `POST /sessions` - Create training session
- `GET /sessions` - List user sessions
- `GET /sessions/:id` - Get session details
- `POST /analyze` - Analyze training data

### ML Routes (`/api/v1/ml`)
- `POST /predict` - Generate predictions
- `GET /predictions` - List predictions
- `POST /train` - Train ML models (admin)

### Goal & Plan Routes
- `/api/v1/goals` - CRUD operations for user goals
- `/api/v1/plans` - AI-generated training plan creation
- `/api/v1/events` - Training event tracking

### Analytics & Insights
- `/api/v1/analytics` - Performance metrics aggregation
- `/api/v1/performance` - Detailed insights and trends
- `/api/v1/notifications` - Alert and reminder system

## Environment Variables

Required variables (see `src/config/app.rs` and `src/config/database.rs`):

```bash
# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach

# Server (optional, defaults shown)
HOST=0.0.0.0
PORT=3000

# JWT Authentication (required in production)
JWT_SECRET=your-secret-key-here

# Logging (optional)
RUST_LOG=info  # Options: error, warn, info, debug, trace
```

## Machine Learning Components

### ML Services Architecture

1. **Feature Engineering** (`feature_engineering_service.rs`): Transform raw training data
2. **Model Training** (`model_training_service.rs`): Train regression/classification models
3. **Predictions** (`model_prediction_service.rs`): Generate predictions from trained models
4. **Recommendations** (`workout_recommendation_service.rs`): Personalized workout suggestions

### ML Libraries Used

- **linfa**: Core ML framework (linear models, trees)
- **ndarray**: N-dimensional array operations
- **statrs**: Statistical computations

## Common Development Patterns

### Adding New API Endpoint

1. Create handler in `src/api/your_module.rs`
2. Define request/response models in `src/models/your_model.rs`
3. Implement business logic in `src/services/your_service.rs`
4. Register route in `src/api/routes.rs`
5. Add migration if database tables needed
6. Write tests in `tests/integration/your_module_test.rs`

### Service Layer Pattern

Services follow constructor injection pattern:
```rust
pub struct YourService {
    db: PgPool,
}

impl YourService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn your_method(&self) -> Result<Data, Error> {
        // Implementation
    }
}
```

### Error Handling

Uses `anyhow::Result` for services, custom error types for API responses. All API errors should be mapped to appropriate HTTP status codes in handlers.

## Important Notes

- **Database Connection**: Always ensure `DATABASE_URL` is set before running
- **Migrations**: Run automatically on server startup via `run_migrations()`
- **Authentication**: Most endpoints require JWT token in `Authorization: Bearer <token>` header
- **Test Isolation**: Database tests use transactions for automatic rollback
- **Background Jobs**: Redis required for notification scheduler and background processing
- **Port Conflicts**: Ensure port 3000 (app) and 5432 (PostgreSQL) are available

## Code Style Conventions

- Use `async/await` for all I/O operations
- Prefer `Result<T, E>` over panicking
- Use `tracing` macros for logging, not `println!`
- Follow Rust naming conventions (snake_case for functions/variables)
- Keep handlers thin, move logic to services
- Use SQLx compile-time query verification when possible