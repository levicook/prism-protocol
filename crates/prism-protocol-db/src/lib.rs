/*!
# Prism Protocol Database Management

This crate provides unified database access for Prism Protocol campaigns.

## Purpose

This crate will contain all campaign database operations, replacing the scattered
`Connection::open()` calls throughout the CLI commands with a clean, unified interface.

## Planned Features

- **Unified Connection Management**: Single database connection interface
- **Campaign Operations**: CRUD operations for campaigns
- **Transaction Management**: Proper transaction handling
- **Migration Support**: Database schema versioning
- **Error Handling**: Consistent error types across all database operations

## Status

🚧 **Under Development** - This crate is currently a placeholder during the technical debt cleanup phase.

The full implementation will be completed during **Phase 3A: Infrastructure Cleanup**.
*/

pub mod database;
pub mod errors;

// Re-export main types for convenience
pub use errors::{DbError, DbResult};
