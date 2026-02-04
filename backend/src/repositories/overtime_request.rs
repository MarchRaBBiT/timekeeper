//! Overtime request repository.
//!
//! Provides CRUD operations for overtime requests.
//! This module re-exports the trait-based implementation from overtime_request_repository.

pub use crate::repositories::overtime_request_repository::{
    OvertimeRequestRepository, OvertimeRequestRepositoryTrait,
};

// MockOvertimeRequestRepositoryTrait is only available in test builds via #[cfg(test)]
#[cfg(test)]
pub use crate::repositories::overtime_request_repository::MockOvertimeRequestRepositoryTrait;
