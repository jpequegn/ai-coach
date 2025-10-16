# SQLite Compatibility Notes

## Goals Feature - Blocked

**Status**: Implementation started but blocked by SQLite type incompatibilities

**Issue**: SQLite stores DateTime as TEXT, but sqlx query macros expect exact type matches

### Work Completed

1. ✅ Created migrations: `007_create_goals_table.sql`, `008_create_goal_progress_table.sql`
2. ✅ Fixed imports: Changed `PgPool` to `SqlitePool` in goals.rs and goal_service.rs
3. ✅ Enabled routes in routes.rs and mod.rs files
4. ✅ Ran migrations manually with sqlite3

### Blocking Errors

```
error[E0277]: the trait `From<std::string::String>` is not implemented for `DateTime<Utc>`
```

**Root Cause**: sqlx::query_as! macro performs compile-time type checking and cannot convert SQLite's TEXT timestamps to Rust's `DateTime<Utc>` automatically.

### Solutions

**Option 1**: Convert all `sqlx::query_as!()` macros to `sqlx::query_as()` builders
- Pros: Works with SQLite TEXT timestamps
- Cons: Loses compile-time query verification, more verbose code

**Option 2**: Create custom type wrappers for SQLite timestamps
- Pros: Maintains type safety
- Cons: Complex implementation, affects all DateTime fields

**Option 3**: Use `#[sqlx(type_name = "TEXT")]` annotations
- Pros: Minimal code changes
- Cons: May not work with query_as! macros

**Recommendation**: For MVP, prioritize features without DateTime complexities. Goals feature should be enabled after moving to PostgreSQL or implementing Option 1 comprehensively.

### Files Modified (Need Rollback if Not Pursuing)

- `ai-coach-api/migrations/007_create_goals_table.sql` (new)
- `ai-coach-api/migrations/008_create_goal_progress_table.sql` (new)
- `ai-coach-api/src/api/goals.rs` (PgPool → SqlitePool)
- `ai-coach-api/src/services/goal_service.rs` (PgPool → SqlitePool)
- `ai-coach-api/src/services/mod.rs` (uncommented goal_service)
- `ai-coach-api/src/models/mod.rs` (uncommented goal module)
- `ai-coach-api/src/api/mod.rs` (uncommented goals)
- `ai-coach-api/src/api/routes.rs` (added goals routes)

## Recommendations for Future Features

**Low Complexity** (Prioritize for MVP):
- Features without DateTime fields
- Simple CRUD with basic types (strings, integers, floats)
- Features using existing migrations

**Medium Complexity**:
- Features with DateTime but willing to use TEXT and handle conversion manually
- Features with enums stored as TEXT

**High Complexity** (Defer to PostgreSQL migration):
- Features with complex type mappings
- Features using PostgreSQL-specific types (JSON, arrays)
- Features requiring compile-time query verification
