# AI Coach API - MVP Feature Set

**Version**: 1.0.0-mvp
**Last Updated**: 2025-10-16
**Status**: Production Ready

## Overview

The AI Coach API MVP provides a **production-ready authentication and user management system** with SQLite database. This is a minimal viable product focused on core functionality without advanced ML/recovery features.

## Available Features

### 1. Authentication System ✅

**Endpoints**: `/api/v1/auth/*`

Full JWT-based authentication with token refresh and blacklisting.

#### Register New User
```bash
POST /api/v1/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "full_name": "John Doe"
}

Response: 201 Created
{
  "access_token": "eyJ0eXAiOiJKV1...",
  "refresh_token": "eyJ0eXAiOiJKV1...",
  "token_type": "Bearer",
  "expires_in": 900,
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "role": "athlete",
    "created_at": "2025-10-16T...",
    "updated_at": "2025-10-16T..."
  }
}
```

#### Login
```bash
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}

Response: 200 OK
{
  "access_token": "eyJ0eXAiOiJKV1...",
  "refresh_token": "eyJ0eXAiOiJKV1...",
  "token_type": "Bearer",
  "expires_in": 900,
  "user": { ... }
}
```

#### Refresh Token
```bash
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJ0eXAiOiJKV1..."
}

Response: 200 OK
{
  "access_token": "new_access_token...",
  "refresh_token": "new_refresh_token...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

#### Logout
```bash
POST /api/v1/auth/logout
Authorization: Bearer <access_token>

Response: 200 OK
{
  "message": "Successfully logged out"
}
```

**Features**:
- ✅ Password validation (8+ chars, uppercase, lowercase, number, special)
- ✅ Email validation
- ✅ JWT access tokens (15 min expiry)
- ✅ JWT refresh tokens (30 day expiry)
- ✅ Token blacklisting on logout
- ✅ Automatic token cleanup
- ✅ Secure password hashing (bcrypt)

---

### 2. User Profile Management ✅

**Endpoints**: `/api/v1/user/*`

Manage athlete profiles with sport-specific data.

#### Get Current User Profile
```bash
GET /api/v1/auth/profile
Authorization: Bearer <access_token>

Response: 200 OK
{
  "id": "uuid",
  "email": "user@example.com",
  "role": "athlete",
  "created_at": "...",
  "updated_at": "..."
}
```

#### Get Athlete Profile
```bash
GET /api/v1/user/profile
Authorization: Bearer <access_token>

Response: 200 OK
{
  "id": "uuid",
  "user_id": "uuid",
  "full_name": "John Doe",
  "date_of_birth": "1990-01-01",
  "gender": "male",
  "weight": 70.5,
  "height": 175.0,
  "sport_focus": "cycling",
  "fitness_level": "intermediate",
  "ftp": 250,
  "max_hr": 190,
  "resting_hr": 55,
  "created_at": "...",
  "updated_at": "..."
}
```

#### Update Athlete Profile
```bash
PUT /api/v1/user/profile
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "full_name": "John Doe Updated",
  "weight": 71.0,
  "ftp": 260
}

Response: 200 OK
{
  "id": "uuid",
  "user_id": "uuid",
  "full_name": "John Doe Updated",
  "weight": 71.0,
  "ftp": 260,
  ...
}
```

**Features**:
- ✅ Create/Read/Update athlete profiles
- ✅ Sport-specific metrics (FTP, max HR, resting HR)
- ✅ Physical attributes (weight, height, age)
- ✅ Fitness level tracking
- ✅ Automatic profile creation on registration

---

### 3. Admin Routes ✅

**Endpoints**: `/api/v1/admin/*`

Administrative functions for user management.

#### Update User Role
```bash
POST /api/v1/admin/users/:user_id/role
Authorization: Bearer <admin_access_token>
Content-Type: application/json

{
  "role": "coach"
}

Response: 200 OK
{
  "message": "User role updated successfully",
  "user_id": "uuid",
  "new_role": "coach"
}
```

**Roles**:
- `athlete` (default)
- `coach`
- `admin`

**Features**:
- ✅ Role-based access control
- ✅ Admin-only endpoints
- ✅ User role management

---

### 4. Health Checks ✅

**Endpoints**: `/health`, `/health/detailed`

#### Basic Health Check
```bash
GET /health

Response: 200 OK
{
  "status": "healthy"
}
```

#### Detailed Health Check
```bash
GET /health/detailed

Response: 200 OK
{
  "status": "healthy",
  "database": "connected",
  "scheduler": "disabled_in_mvp"
}
```

**Features**:
- ✅ Liveness probe
- ✅ Database connectivity check
- ✅ Deployment verification

---

## Security Features

### JWT Validation ✅

- **Secret Requirement**: 32+ character JWT_SECRET in production
- **Token Expiry**: Access tokens expire after 15 minutes
- **Refresh Flow**: Refresh tokens valid for 30 days
- **Blacklisting**: Logout invalidates tokens immediately

### CORS Configuration ✅

- **Development**: Permissive CORS for localhost
- **Production**: Explicit origin whitelisting required
- **Configuration**: Via `ALLOWED_ORIGINS` environment variable

### Password Security ✅

- **Hashing**: bcrypt with secure work factor
- **Validation**: Minimum 8 chars, complexity requirements
- **Storage**: Never stored in plaintext

---

## Database

### Technology

- **Type**: SQLite 3
- **Location**: `ai-coach-api/data/ai-coach.db`
- **Persistence**: File-based (survives restarts)

### Tables

1. **users** - User accounts and credentials
2. **user_roles** - Role assignments (athlete/coach/admin)
3. **athlete_profiles** - Athlete-specific data and metrics
4. **user_recovery_profiles** - Recovery preferences and settings
5. **refresh_tokens** - Active refresh tokens
6. **token_blacklist** - Invalidated tokens

### Migrations

Migrations run automatically on server startup. Located in `migrations/`:
- `001_create_users_table.sql`
- `002_create_athlete_profiles_table.sql`
- `003_create_user_recovery_profiles.sql`
- `004_create_user_roles_table.sql`
- `005_create_refresh_tokens_table.sql`
- `006_create_token_blacklist_table.sql`

---

## Configuration

### Required Environment Variables

```bash
# Security (REQUIRED in production)
JWT_SECRET=your-secure-jwt-secret-key-at-least-32-characters-long

# Database
DATABASE_URL=sqlite://ai-coach-api/data/ai-coach.db

# CORS (REQUIRED in production)
ALLOWED_ORIGINS=https://your-frontend-domain.com
```

### Optional Variables

```bash
# Server
HOST=0.0.0.0
PORT=3000
ENVIRONMENT=production
LOG_LEVEL=info

# Database Pool
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
```

See [.env.example](../.env.example) for complete configuration.

---

## Limitations & Roadmap

### Not Included in MVP

The following features exist in the codebase but are **disabled**:

- ❌ **Goals Management** - Blocked by SQLite DateTime compatibility
- ❌ **Training Session Tracking** - Disabled for MVP
- ❌ **ML-Based Recommendations** - Requires PostgreSQL
- ❌ **Recovery Analysis** - Requires background jobs
- ❌ **Performance Insights** - Disabled for MVP
- ❌ **Analytics Dashboard** - Disabled for MVP
- ❌ **Notification System** - Disabled for MVP
- ❌ **Background Jobs** - Disabled for MVP

### Future Enhancements

See [Issue #134 Status Report](issue-134-status-report.md) for detailed analysis of advanced features and [SQLite Compatibility Notes](sqlite-compatibility-notes.md) for technical blockers.

**Path to Full Features**:
1. Migrate to PostgreSQL for DateTime support
2. Re-enable recovery and ML services
3. Implement remaining Issue #134 features
4. Enable background job processing

---

## Deployment

### Quick Start

```bash
# 1. Clone and navigate
git clone <repo>
cd ai-coach/ai-coach-api

# 2. Configure environment
cp .env.example .env
# Edit .env with your JWT_SECRET and other config

# 3. Build and run
cargo build --release
./target/release/ai-coach-api
```

See [DEPLOY.md](../DEPLOY.md) for comprehensive deployment guide including:
- Production configuration
- Security checklist
- Docker deployment
- Systemd service setup
- Monitoring and logging
- Troubleshooting

### Testing the API

```bash
# Health check
curl http://localhost:3000/health

# Register user
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"TestPass123!","full_name":"Test User"}'

# Login
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"TestPass123!"}'
```

---

## CLI Integration

The AI Coach CLI (`ai-coach-cli`) works with this MVP API:

```bash
# Login from CLI
ai-coach login

# View profile
ai-coach whoami

# Logout
ai-coach logout
```

See [CLI Documentation](../../ai-coach-cli/README.md) for full CLI capabilities.

---

## Support & Documentation

- **API Documentation**: This file
- **Deployment Guide**: [DEPLOY.md](../DEPLOY.md)
- **Environment Config**: [.env.example](../.env.example)
- **SQLite Compatibility**: [sqlite-compatibility-notes.md](sqlite-compatibility-notes.md)
- **Issue #134 Status**: [issue-134-status-report.md](issue-134-status-report.md)
- **Main README**: [../../README.md](../../README.md)

---

**Last Updated**: 2025-10-16
**Version**: MVP 1.0.0
**Branch**: feature/minimal-viable-api
**Commit**: 5fa9f98
