//! Models for the department hierarchy and manager assignments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::types::{DepartmentId, UserId};

/// Database representation of a department node in the hierarchy tree.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Department {
    pub id: DepartmentId,
    pub name: String,
    pub parent_id: Option<DepartmentId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database representation of a manager assignment.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DepartmentManager {
    pub department_id: DepartmentId,
    pub user_id: UserId,
    pub assigned_at: DateTime<Utc>,
}

/// Payload to create a new department.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDepartmentPayload {
    pub name: String,
    pub parent_id: Option<String>,
}

/// Payload to update an existing department.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateDepartmentPayload {
    pub name: Option<String>,
    pub parent_id: Option<String>,
}

/// Payload to assign a manager to a department.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignManagerPayload {
    pub user_id: String,
}

/// API response for a department.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepartmentResponse {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Department> for DepartmentResponse {
    fn from(d: Department) -> Self {
        DepartmentResponse {
            id: d.id.to_string(),
            name: d.name,
            parent_id: d.parent_id.map(|p| p.to_string()),
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn department_response_from_department() {
        let now = Utc::now();
        let dept = Department {
            id: DepartmentId::new(),
            name: "Engineering".into(),
            parent_id: None,
            created_at: now,
            updated_at: now,
        };
        let resp = DepartmentResponse::from(dept.clone());
        assert_eq!(resp.name, "Engineering");
        assert!(resp.parent_id.is_none());
    }
}
