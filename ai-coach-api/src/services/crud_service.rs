//! Generic CRUD service trait and implementations
//!
//! This module provides a standard trait for CRUD operations that can be
//! implemented by any service, reducing boilerplate and ensuring consistency.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Parameters for listing/filtering entities
#[derive(Debug, Clone, Deserialize)]
pub struct ListParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,

    /// Sort field
    #[serde(default)]
    pub sort_by: Option<String>,

    /// Sort direction (asc or desc)
    #[serde(default = "default_sort_order")]
    pub sort_order: SortOrder,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    50
}

fn default_sort_order() -> SortOrder {
    SortOrder::Desc
}

/// Sort order for list operations
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
            sort_by: None,
            sort_order: SortOrder::Desc,
        }
    }
}

impl ListParams {
    /// Calculate the SQL OFFSET value for pagination
    pub fn offset(&self) -> i64 {
        ((self.page - 1) * self.per_page) as i64
    }

    /// Get the LIMIT value for pagination
    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }

    /// Get the sort order as SQL string
    pub fn sort_order_sql(&self) -> &str {
        match self.sort_order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

/// Result of a paginated list operation
#[derive(Debug, Serialize)]
pub struct PaginatedResult<T> {
    /// The items for the current page
    pub items: Vec<T>,

    /// Total number of items across all pages
    pub total: u64,

    /// Current page number
    pub page: u32,

    /// Items per page
    pub per_page: u32,
}

/// Generic CRUD operations trait
///
/// Implement this trait for services that need standard CRUD functionality.
/// The trait provides default implementations for common operations.
#[async_trait]
pub trait CrudService<T, ID>
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// Get the database pool
    fn pool(&self) -> &PgPool;

    /// Create a new entity
    async fn create(&self, entity: T) -> Result<T>;

    /// Get an entity by ID
    async fn get(&self, id: ID) -> Result<Option<T>>;

    /// Update an existing entity
    async fn update(&self, id: ID, entity: T) -> Result<T>;

    /// Delete an entity by ID
    async fn delete(&self, id: ID) -> Result<()>;

    /// List entities with pagination
    async fn list(&self, params: ListParams) -> Result<PaginatedResult<T>>;

    /// Count total entities (useful for pagination)
    async fn count(&self) -> Result<u64>;

    /// Check if an entity exists
    async fn exists(&self, id: ID) -> Result<bool> {
        Ok(self.get(id).await?.is_some())
    }
}

/// Helper function to execute a count query
pub async fn count_query(pool: &PgPool, query: &str) -> Result<u64> {
    let count: i64 = sqlx::query_scalar(query).fetch_one(pool).await?;
    Ok(count as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_params_offset() {
        let params = ListParams {
            page: 1,
            per_page: 10,
            ..Default::default()
        };
        assert_eq!(params.offset(), 0);

        let params = ListParams {
            page: 2,
            per_page: 10,
            ..Default::default()
        };
        assert_eq!(params.offset(), 10);

        let params = ListParams {
            page: 5,
            per_page: 20,
            ..Default::default()
        };
        assert_eq!(params.offset(), 80);
    }

    #[test]
    fn test_list_params_limit() {
        let params = ListParams {
            page: 1,
            per_page: 25,
            ..Default::default()
        };
        assert_eq!(params.limit(), 25);
    }

    #[test]
    fn test_list_params_defaults() {
        let params = ListParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 50);
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 50);
    }

    #[test]
    fn test_sort_order_sql() {
        let params = ListParams {
            sort_order: SortOrder::Asc,
            ..Default::default()
        };
        assert_eq!(params.sort_order_sql(), "ASC");

        let params = ListParams {
            sort_order: SortOrder::Desc,
            ..Default::default()
        };
        assert_eq!(params.sort_order_sql(), "DESC");
    }
}
