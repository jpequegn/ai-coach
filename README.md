# AI Coach

An AI-powered coaching application built with Rust, featuring REST API endpoints and machine learning capabilities.

## Architecture Overview

### Project Structure

```
src/
├── main.rs       # Application entry point and server setup
├── lib.rs        # Library root module
├── api/          # REST API routes and handlers
│   ├── health.rs # Health check endpoint
│   └── routes.rs # Route configuration
├── models/       # Data models and ML structures
├── services/     # Business logic services
├── auth/         # Authentication and authorization
└── config/       # Configuration management
```

### Technology Stack

- **Web Framework**: Axum - Modern async web framework for Rust
- **Async Runtime**: Tokio - Asynchronous runtime for Rust
- **Database**: PostgreSQL with SQLx for type-safe database interactions
- **Serialization**: Serde for JSON serialization/deserialization
- **Authentication**: JWT tokens with jsonwebtoken
- **Machine Learning**: Candle-core ML framework
- **Logging**: Tracing for structured logging
- **Error Handling**: Anyhow for ergonomic error handling
- **Date/Time**: Chrono for date and time handling
- **Unique IDs**: UUID for generating unique identifiers

### Dependencies

- `trainrs` - Core training and coaching logic (git dependency)
- `axum` - Web framework
- `tokio` - Async runtime
- `sqlx` - PostgreSQL database driver
- `serde` & `serde_json` - Serialization
- `anyhow` - Error handling
- `chrono` - Date/time handling
- `uuid` - Unique identifiers
- `jsonwebtoken` - JWT authentication
- `candle-core` - Machine learning framework
- `tracing` - Logging

## Getting Started

### Prerequisites

- Rust 1.75 or later
- PostgreSQL 15 or later
- Docker and Docker Compose (for development)

### Development Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd ai-coach
   ```

2. **Using Docker Compose (Recommended)**
   ```bash
   docker-compose up --build
   ```
   This will start both the application and PostgreSQL database.

3. **Manual Setup**
   ```bash
   # Start PostgreSQL database
   # Set environment variable
   export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach

   # Run the application
   cargo run
   ```

### API Endpoints

- `GET /health` - Health check endpoint

The server runs on `http://localhost:3000` by default.

### Testing

```bash
# Check compilation
cargo check

# Run tests
cargo test

# Health check
curl http://localhost:3000/health
```

## Development

### Architecture Principles

- **Modular Design**: Clear separation of concerns with dedicated modules
- **Async-First**: Built on Tokio for high-performance async operations
- **Type Safety**: Leveraging Rust's type system and SQLx for compile-time guarantees
- **Error Handling**: Comprehensive error handling with anyhow
- **Observability**: Structured logging with tracing

### Adding New Features

1. **API Endpoints**: Add to `src/api/` directory
2. **Data Models**: Define in `src/models/`
3. **Business Logic**: Implement in `src/services/`
4. **Authentication**: Extend `src/auth/`
5. **Configuration**: Add to `src/config/`

## Docker

The project includes Docker configuration for containerized deployment:

- `Dockerfile` - Multi-stage build for optimized production images
- `docker-compose.yml` - Development environment with PostgreSQL
- `.dockerignore` - Optimized Docker build context

## Contributing

1. Create a feature branch
2. Make your changes
3. Add tests
4. Ensure `cargo check` and `cargo test` pass
5. Submit a pull request

## License

[Add license information here]