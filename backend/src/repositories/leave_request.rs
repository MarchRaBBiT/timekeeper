//! Leave request repository.
//!
//! Provides CRUD operations for leave requests.
//! This module re-exports the trait-based implementation from leave_request_repository.

pub use crate::repositories::leave_request_repository::{
    LeaveRequestRepository, LeaveRequestRepositoryTrait,
};

// MockLeaveRequestRepositoryTrait is only available in test builds via #[cfg(test)]
#[cfg(test)]
pub use crate::repositories::leave_request_repository::MockLeaveRequestRepositoryTrait;
