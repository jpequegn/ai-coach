# AI Coach API - Production Deployment Guide

## Prerequisites

- Rust 1.70+ installed
- PostgreSQL 14+ (recommended for production) or SQLite 3 (MVP/development)
- Minimum 1GB RAM, 2GB recommended
- SSL/TLS certificate for HTTPS (required in production)

## Quick Start

### 1. Environment Configuration

Copy the example environment file and configure:

```bash
cp .env.example .env
```

**REQUIRED Configuration**:

```bash
# Generate secure JWT secret (32+ characters)
JWT_SECRET=$(openssl rand -base64 32)

# Database URL
DATABASE_URL=postgresql://user:password@localhost:5432/ai_coach

# Set environment
ENVIRONMENT=production

# Configure allowed origins for CORS
ALLOWED_ORIGINS=https://your-frontend-domain.com
```

### 2. Database Setup

#### PostgreSQL (Production Recommended)

```bash
# Create database
createdb ai_coach

# Migrations run automatically on startup
# Or run manually:
DATABASE_URL=postgresql://user:password@localhost:5432/ai_coach cargo run
```

####SQLite (MVP/Development)

```bash
# Database is created automatically
# Default location: ai-coach-api/data/ai-coach.db
DATABASE_URL=sqlite://ai-coach-api/data/ai-coach.db cargo run
```

### 3. Build and Run

#### Development

```bash
cargo run
```

#### Production

```bash
# Build optimized binary
cargo build --release

# Run
./target/release/ai-coach-api
```

## Environment Variables Reference

### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `JWT_SECRET` | JWT signing key (32+ chars) | `abc123...xyz789` |
| `DATABASE_URL` | Database connection string | `postgresql://...` |
| `ALLOWED_ORIGINS` | CORS allowed origins | `https://app.example.com` |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `3000` | Server port |
| `ENVIRONMENT` | `development` | Environment mode |
| `LOG_LEVEL` | `info` | Logging level |
| `DB_MAX_CONNECTIONS` | `20` | Max database connections |
| `DB_MIN_CONNECTIONS` | `5` | Min database connections |
| `DB_CONNECT_TIMEOUT` | `30` | Connection timeout (seconds) |
| `DB_IDLE_TIMEOUT` | `600` | Idle timeout (seconds) |

## Security Checklist

### Pre-Deployment

- [ ] JWT_SECRET is at least 32 characters and randomly generated
- [ ] DATABASE_URL uses strong password
- [ ] ALLOWED_ORIGINS explicitly configured (no wildcards)
- [ ] ENVIRONMENT set to `production`
- [ ] SSL/TLS certificates configured
- [ ] Database backups configured
- [ ] Log aggregation configured

### Post-Deployment

- [ ] Health check endpoint responding (`/health`)
- [ ] Authentication flow working (`/api/v1/auth/login`)
- [ ] CORS headers correct (check browser DevTools)
- [ ] No sensitive data in logs
- [ ] Database migrations applied
- [ ] Monitoring and alerting configured

## Docker Deployment (Coming Soon)

```bash
# Build image
docker build -t ai-coach-api .

# Run container
docker run -d \
  -p 3000:3000 \
  --env-file .env \
  --name ai-coach-api \
  ai-coach-api
```

## Systemd Service (Linux)

Create `/etc/systemd/system/ai-coach-api.service`:

```ini
[Unit]
Description=AI Coach API
After=network.target postgresql.service

[Service]
Type=simple
User=aicoach
WorkingDirectory=/opt/ai-coach-api
Environment="JWT_SECRET=your-secret"
Environment="DATABASE_URL=postgresql://..."
Environment="ENVIRONMENT=production"
ExecStart=/opt/ai-coach-api/target/release/ai-coach-api
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable ai-coach-api
sudo systemctl start ai-coach-api
sudo systemctl status ai-coach-api
```

## Monitoring

### Health Checks

- Basic: `GET /health`
- Detailed: `GET /health/detailed`

```bash
curl http://localhost:3000/health
# {"status":"healthy"}
```

### Logs

Logs are output to stderr in structured format. Configure log aggregation:

```bash
# Development - console output
LOG_LEVEL=debug cargo run

# Production - JSON structured logging (future enhancement)
LOG_LEVEL=warn ./target/release/ai-coach-api 2>&1 | tee /var/log/ai-coach-api.log
```

### Metrics (Future Enhancement)

- Prometheus metrics endpoint: `/metrics`
- Grafana dashboards for visualization

## Troubleshooting

### JWT_SECRET validation error

```
Error: JWT_SECRET must be at least 32 characters long
```

**Solution**: Generate a secure secret:
```bash
openssl rand -base64 32
```

### Database connection failed

```
Error: Failed to connect to database
```

**Solutions**:
- Check DATABASE_URL is correct
- Ensure database server is running
- Verify network connectivity
- Check firewall rules

### CORS errors in browser

```
Access to XMLHttpRequest blocked by CORS policy
```

**Solutions**:
- Add frontend origin to ALLOWED_ORIGINS
- Ensure protocol (http/https) matches
- Check for trailing slashes in URLs

### Port already in use

```
Error: Address already in use (os error 98)
```

**Solution**:
```bash
# Find process using port 3000
lsof -ti:3000 | xargs kill -9

# Or use different port
PORT=3001 cargo run
```

## Performance Tuning

### Database Connection Pool

For high-traffic deployments:

```bash
DB_MAX_CONNECTIONS=50
DB_MIN_CONNECTIONS=10
```

### Resource Limits

Recommended minimum:
- RAM: 512MB (MVP), 2GB (production)
- CPU: 1 core (MVP), 2+ cores (production)
- Disk: 1GB for application, 10GB+ for database

## Backup and Recovery

### Database Backups

#### PostgreSQL

```bash
# Backup
pg_dump -U user ai_coach > backup.sql

# Restore
psql -U user ai_coach < backup.sql
```

#### SQLite

```bash
# Backup
cp ai-coach-api/data/ai-coach.db backup-$(date +%Y%m%d).db

# Restore
cp backup-20241016.db ai-coach-api/data/ai-coach.db
```

### Disaster Recovery

1. Restore database from backup
2. Verify DATABASE_URL in environment
3. Restart application
4. Run health checks
5. Verify authentication flow

## Scaling

### Horizontal Scaling

For multiple instances:
1. Use external PostgreSQL database (not SQLite)
2. Configure load balancer
3. Ensure session state is database-backed
4. Use Redis for caching (future enhancement)

### Vertical Scaling

Increase resources:
- DB_MAX_CONNECTIONS based on available RAM
- CPU cores for parallel request handling

## Support

For issues and questions:
- GitHub Issues: https://github.com/your-org/ai-coach/issues
- Documentation: See README.md
- Security Issues: security@example.com (private)

## License

See LICENSE file in repository root.
