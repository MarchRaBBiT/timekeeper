//! Unified validation framework for request payloads.
//!
//! This module provides reusable validation rules and utilities
//! to ensure consistent input validation across all API endpoints.

pub mod rules;

pub use validator::Validate;
