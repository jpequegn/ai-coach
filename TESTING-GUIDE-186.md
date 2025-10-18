# Testing Guide: Issue #186 - Recommendation System SQLite Migration

## Prerequisites

You'll need one of the following tools to run migrations and test:

### Option A: sqlx-cli (Recommended)
```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

### Option B: sqlite3 CLI
**Windows**: Download from https://www.sqlite.org/download.html
**macOS**: `brew install sqlite3`
**Linux**: `sudo apt-get install sqlite3` or `sudo yum install sqlite`

### Option C: DBeaver or DB Browser for SQLite
GUI tools that can run SQL files and query the database.

---

## Phase 5 Testing Steps

### Step 1: Set Up Database

**Create .env file** (if not exists):
```bash
cd ai-coach-api
cp .env.example .env
```

**Verify DATABASE_URL** in `.env`:
```
DATABASE_URL=sqlite://data/ai-coach.db
```

**Ensure data directory exists**:
```bash
mkdir -p data
```

---

### Step 2: Run All Migrations

#### Using sqlx-cli (Recommended):
```bash
cd ai-coach-api
export DATABASE_URL=sqlite://data/ai-coach.db  # Linux/macOS
# OR
set DATABASE_URL=sqlite://data/ai-coach.db     # Windows CMD
# OR
$env:DATABASE_URL="sqlite://data/ai-coach.db"  # Windows PowerShell

# Run all migrations
sqlx migrate run

# Verify migration status
sqlx migrate info
```

#### Using sqlite3 CLI:
```bash
cd ai-coach-api

# Create database
sqlite3 data/ai-coach.db ".databases"

# Run migrations in order (001-020)
for file in migrations/*.sql; do
    echo "Running $file..."
    sqlite3 data/ai-coach.db < "$file"
done
```

#### Using Windows PowerShell with sqlite3:
```powershell
cd ai-coach-api

# Create database
sqlite3.exe data/ai-coach.db ".databases"

# Run migrations
Get-ChildItem migrations/*.sql | Sort-Object Name | ForEach-Object {
    Write-Host "Running $_..."
    Get-Content $_ | sqlite3.exe data/ai-coach.db
}
```

---

### Step 3: Verify Database Schema

#### Query to check all tables:
```sql
SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;
```

**Expected tables** (including recommendations):
- `users`
- `athlete_profiles`
- `user_recovery_profiles`
- `recovery_scores`
- `recovery_score_trends`
- `user_recovery_settings`
- `recovery_alerts`
- `alert_recipients`
- `audit_log`
- `recovery_protocols`
- **`recommendation_templates`** ✅ NEW
- **`recommendation_content`** ✅ NEW
- **`user_recommendations`** ✅ NEW
- **`recommendation_outcomes`** ✅ NEW

#### Using sqlx-cli:
```bash
cd ai-coach-api
sqlx database create  # If needed
sqlx migrate run
sqlx database drop    # Only if you want to start fresh
```

#### Using sqlite3:
```bash
cd ai-coach-api
sqlite3 data/ai-coach.db

.tables                          # Show all tables
.schema recommendation_templates # Show table structure
```

---

### Step 4: Verify Seed Data (75 Recommendations)

#### Count total recommendations:
```sql
SELECT COUNT(*) as total FROM recommendation_templates;
```
**Expected**: 75

#### Count by category:
```sql
SELECT category, COUNT(*) as count
FROM recommendation_templates
GROUP BY category
ORDER BY category;
```

**Expected Results**:
| Category | Count |
|----------|-------|
| active_recovery | 13 |
| nutrition | 15 |
| sleep | 22 |
| stress_management | 12 |
| training_modification | 13 |
| **TOTAL** | **75** |

#### Verify UUID prefixes:
```sql
SELECT category,
       MIN(id) as first_id,
       MAX(id) as last_id,
       COUNT(*) as count
FROM recommendation_templates
GROUP BY category
ORDER BY category;
```

**Expected ID Ranges**:
- `sleep`: 10000000-... (22 templates)
- `nutrition`: 20000000-... (15 templates)
- `active_recovery`: 30000000-... (13 templates)
- `stress_management`: 40000000-... (12 templates)
- `training_modification`: 50000000-... (13 templates)

#### Sample a few recommendations:
```sql
SELECT id, category, title, difficulty, priority_default
FROM recommendation_templates
WHERE category = 'sleep'
LIMIT 5;
```

---

### Step 5: Test Table Constraints

#### Test CHECK constraints:
```sql
-- Should FAIL (invalid category)
INSERT INTO recommendation_templates (id, category, title, description, action)
VALUES ('test-uuid', 'invalid_category', 'Test', 'Test', 'Test');

-- Should FAIL (invalid difficulty)
INSERT INTO recommendation_templates (id, category, title, description, action, difficulty)
VALUES ('test-uuid', 'sleep', 'Test', 'Test', 'Test', 'invalid');

-- Should FAIL (invalid priority)
INSERT INTO recommendation_templates (id, category, title, description, action, priority_default)
VALUES ('test-uuid', 'sleep', 'Test', 'Test', 'Test', 'invalid');
```

#### Test triggers:
```sql
-- Create a template and check timestamps
INSERT INTO recommendation_templates (id, category, title, description, action)
VALUES ('99999999-0000-0000-0000-000000000001', 'sleep', 'Test Template', 'Test Description', 'Test Action');

SELECT id, created_at, updated_at
FROM recommendation_templates
WHERE id = '99999999-0000-0000-0000-000000000001';

-- Update and verify updated_at changed
UPDATE recommendation_templates
SET title = 'Updated Test Template'
WHERE id = '99999999-0000-0000-0000-000000000001';

SELECT id, created_at, updated_at
FROM recommendation_templates
WHERE id = '99999999-0000-0000-0000-000000000001';

-- Cleanup
DELETE FROM recommendation_templates
WHERE id = '99999999-0000-0000-0000-000000000001';
```

#### Test user_recommendations triggers:
```sql
-- This requires a user first, so document expected behavior:
-- 1. When status changes to 'completed', completed_at should auto-populate
-- 2. When status changes to 'skipped', skipped_at should auto-populate
-- 3. When status changes to 'expired', expired_at should auto-populate
-- 4. updated_at should always update
```

---

### Step 6: Test JSON Fields

#### Verify JSON parsing:
```sql
SELECT id, title,
       json_extract(trigger_conditions, '$.poor_sleep') as poor_sleep_trigger,
       json_extract(metadata, '$.evidence_level') as evidence_level
FROM recommendation_templates
WHERE category = 'sleep'
LIMIT 5;
```

#### Verify all JSON fields are valid:
```sql
-- Should return 0 (all JSON is valid)
SELECT COUNT(*) FROM recommendation_templates
WHERE json_valid(trigger_conditions) = 0
   OR json_valid(user_constraints) = 0
   OR json_valid(metadata) = 0;
```

---

### Step 7: Test Foreign Key Relationships

#### Verify foreign keys work (requires user first):
```sql
-- This would test after creating test user:
-- INSERT INTO user_recommendations should fail if:
-- - user_id doesn't exist in users
-- - recommendation_template_id doesn't exist in recommendation_templates
-- - recovery_score_id doesn't exist in recovery_scores (if not NULL)
```

---

### Step 8: Performance Checks

#### Test indexes:
```sql
-- Should use index
EXPLAIN QUERY PLAN
SELECT * FROM recommendation_templates WHERE category = 'sleep' AND is_active = 1;

-- Should use index
EXPLAIN QUERY PLAN
SELECT * FROM recommendation_templates WHERE effectiveness_score > 0.7;
```

---

## Expected Test Results Summary

### ✅ Success Criteria

**Schema**:
- [ ] All 14 tables exist
- [ ] 4 recommendation tables created (templates, content, user_recommendations, outcomes)

**Data**:
- [ ] 75 recommendation templates loaded
- [ ] Distribution: 22 sleep, 15 nutrition, 13 active recovery, 12 stress, 13 training
- [ ] All templates have valid JSON in trigger_conditions, user_constraints, metadata
- [ ] UUID prefixes correct (10000000-, 20000000-, etc.)

**Constraints**:
- [ ] CHECK constraints enforce valid enums (category, difficulty, priority, status)
- [ ] NOT NULL constraints work
- [ ] UNIQUE constraints work (e.g., user_recommendation_id in outcomes)

**Triggers**:
- [ ] Auto-update updated_at on UPDATE
- [ ] Auto-set completed_at/skipped_at/expired_at when status changes
- [ ] Auto-calculate recovery_improvement in outcomes

**Indexes**:
- [ ] Queries use indexes (check with EXPLAIN QUERY PLAN)
- [ ] Performance acceptable for expected data volumes

---

## Troubleshooting

### Migration Errors

**Error: "table already exists"**
```bash
# Drop and recreate (WARNING: destroys data)
rm data/ai-coach.db
sqlx migrate run
```

**Error: "no such table"**
```bash
# Check which migrations ran
sqlx migrate info

# Or manually check
sqlite3 data/ai-coach.db ".schema _sqlx_migrations"
```

### Data Issues

**Wrong count of recommendations**:
```sql
-- Check what actually loaded
SELECT category, COUNT(*) FROM recommendation_templates GROUP BY category;

-- Check for duplicates
SELECT id, COUNT(*) FROM recommendation_templates GROUP BY id HAVING COUNT(*) > 1;
```

**JSON parse errors**:
```sql
-- Find invalid JSON
SELECT id, title FROM recommendation_templates WHERE json_valid(trigger_conditions) = 0;
SELECT id, title FROM recommendation_templates WHERE json_valid(user_constraints) = 0;
SELECT id, title FROM recommendation_templates WHERE json_valid(metadata) = 0;
```

---

## Next Steps After Successful Testing

Once all tests pass:

1. **Update PR #187** with test results
2. **Merge PR #187** to integrate recommendation system
3. **Enable Recovery Protocols** (Issue #178)
4. **Implement Progressive Recommendations** (Issue #179)
5. **Complete effectiveness service** (optional - can be future PR)

---

## Manual Testing Commands (Copy-Paste Ready)

### Full Test Suite (sqlite3):
```bash
cd ai-coach-api

# Connect to database
sqlite3 data/ai-coach.db

# Run tests
.mode column
.headers on

-- Check tables
SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;

-- Count recommendations
SELECT COUNT(*) as total FROM recommendation_templates;

-- Count by category
SELECT category, COUNT(*) as count FROM recommendation_templates GROUP BY category ORDER BY category;

-- Sample recommendations
SELECT id, category, title, difficulty FROM recommendation_templates LIMIT 10;

-- Verify JSON
SELECT COUNT(*) as invalid_json_count FROM recommendation_templates WHERE json_valid(trigger_conditions) = 0 OR json_valid(user_constraints) = 0 OR json_valid(metadata) = 0;

.quit
```

### Quick Verification (sqlx-cli):
```bash
cd ai-coach-api
export DATABASE_URL=sqlite://data/ai-coach.db

# Show migration status
sqlx migrate info

# Query database
sqlx database drop    # Only if resetting
sqlx database create
sqlx migrate run
```

---

## Test Results Template

```markdown
## Phase 5 Testing Results - Issue #186

### Environment
- OS: Windows/macOS/Linux
- Tool: sqlx-cli / sqlite3 / DBeaver
- Database: SQLite 3.x

### Migration Results
- [ ] All migrations ran successfully (001-020)
- [ ] No errors during migration
- [ ] Migration timestamps recorded

### Schema Verification
- [ ] 14 tables exist
- [ ] recommendation_templates created
- [ ] recommendation_content created
- [ ] user_recommendations created
- [ ] recommendation_outcomes created

### Data Verification
- [ ] 75 total recommendations loaded
- [ ] Sleep: 22 templates
- [ ] Nutrition: 15 templates
- [ ] Active Recovery: 13 templates
- [ ] Stress Management: 12 templates
- [ ] Training Modifications: 13 templates
- [ ] All JSON fields valid
- [ ] UUID prefixes correct

### Constraint Testing
- [ ] CHECK constraints work (tested invalid values)
- [ ] Triggers auto-update timestamps
- [ ] Foreign keys enforced

### Performance
- [ ] Queries use indexes
- [ ] Response times acceptable

### Issues Found
- None / [List any issues]

### Conclusion
- [ ] All tests passed - Ready for merge
- [ ] Minor issues - Needs fixes
- [ ] Major issues - Needs rework
```
