# Build stage
FROM rust:1.83 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# Build dependencies (this will generate Cargo.lock)
RUN cargo build --release && rm src/*.rs

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/ai-coach /app/ai-coach

# Create non-root user
RUN useradd -r -u 1000 appuser && chown appuser:appuser /app/ai-coach
USER appuser

EXPOSE 3000

CMD ["./ai-coach"]