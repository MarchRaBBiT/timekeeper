//! Attendance repository.
//!
//! Provides CRUD operations for attendance records.
//! This module re-exports the trait-based implementation from attendance_repository.

pub use crate::repositories::attendance_repository::{
    AttendanceRepository, AttendanceRepositoryTrait,
};

// MockAttendanceRepositoryTrait is only available in test builds via #[cfg(test)]
#[cfg(test)]
pub use crate::repositories::attendance_repository::MockAttendanceRepositoryTrait;
