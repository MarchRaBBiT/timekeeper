//! Holiday repository.
//!
//! Provides CRUD operations for holidays.
//! This module re-exports the trait-based implementation from holiday_repository.

pub use crate::repositories::holiday_repository::{HolidayRepository, HolidayRepositoryTrait};

#[cfg(test)]
pub use crate::repositories::holiday_repository::MockHolidayRepositoryTrait;
