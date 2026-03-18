//! Handlers for bulk CSV import of departments and users.

use axum::{extract::State, Extension, Json};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    error::AppError,
    models::user::{User, UserRole},
    repositories::{department as dept_repo, user as user_repo},
    state::AppState,
    types::{DepartmentId, UserId},
    utils::{
        encryption::{encrypt_pii, hash_email},
        password::{hash_password, validate_password_complexity},
    },
};

#[derive(Debug, Deserialize)]
pub struct BulkImportRequest {
    pub csv_data: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImportRowError {
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BulkImportResponse {
    pub imported: usize,
    pub failed: usize,
    pub errors: Vec<ImportRowError>,
}

pub async fn import_departments(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<BulkImportRequest>,
) -> Result<Json<BulkImportResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let mut reader = ReaderBuilder::new().from_reader(payload.csv_data.as_bytes());

    {
        let headers = reader
            .headers()
            .map_err(|e| AppError::BadRequest(format!("CSV parse error: {}", e)))?;
        let header_vec: Vec<&str> = headers.iter().collect();
        let expected = ["name", "parent_name"];
        if header_vec.len() < 2
            || header_vec[0].trim() != expected[0]
            || header_vec[1].trim() != expected[1]
        {
            return Err(AppError::BadRequest(
                "Invalid CSV headers. Expected: name,parent_name".into(),
            ));
        }
    }

    let mut raw_rows: Vec<(String, String)> = Vec::new();
    for (i, result) in reader.records().enumerate() {
        let record = result.map_err(|e| {
            AppError::BadRequest(format!("CSV parse error at row {}: {}", i + 2, e))
        })?;
        let name = record.get(0).unwrap_or("").trim().to_string();
        let parent_name = record.get(1).unwrap_or("").trim().to_string();
        raw_rows.push((name, parent_name));
    }

    if raw_rows.is_empty() {
        return Ok(Json(BulkImportResponse {
            imported: 0,
            failed: 0,
            errors: Vec::new(),
        }));
    }

    let mut errors: Vec<ImportRowError> = Vec::new();
    let mut csv_names: HashSet<String> = HashSet::new();

    // Phase 1: name validation and CSV duplicate check
    for (i, (name, _)) in raw_rows.iter().enumerate() {
        let row_num = i + 2;
        if name.is_empty() {
            errors.push(ImportRowError {
                row: row_num,
                message: "name must not be empty".into(),
            });
        } else if !csv_names.insert(name.clone()) {
            errors.push(ImportRowError {
                row: row_num,
                message: format!("Duplicate name '{}' in CSV", name),
            });
        }
    }

    // Phase 2: parent_name validation
    for (i, (name, parent_name)) in raw_rows.iter().enumerate() {
        let row_num = i + 2;
        if name.is_empty() {
            continue;
        }
        if !parent_name.is_empty() && !csv_names.contains(parent_name.as_str()) {
            let found = dept_repo::find_department_by_name(&state.write_pool, parent_name)
                .await
                .map_err(|e| AppError::InternalServerError(e.into()))?;
            if found.is_none() {
                errors.push(ImportRowError {
                    row: row_num,
                    message: format!(
                        "Parent department '{}' not found in CSV or database",
                        parent_name
                    ),
                });
            }
        }
    }

    if !errors.is_empty() {
        let failed = errors.len();
        return Ok(Json(BulkImportResponse {
            imported: 0,
            failed,
            errors,
        }));
    }

    // Phase 3: Topological sort (Kahn's algorithm)
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();

    for (name, parent_name) in &raw_rows {
        in_degree.entry(name.clone()).or_insert(0);
        if !parent_name.is_empty() && csv_names.contains(parent_name.as_str()) {
            *in_degree.entry(name.clone()).or_insert(0) += 1;
            children
                .entry(parent_name.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut sorted_names: Vec<String> = Vec::new();
    while let Some(name) = queue.pop_front() {
        sorted_names.push(name.clone());
        if let Some(child_list) = children.get(&name) {
            for child in child_list {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }

    // Cycle detection
    if sorted_names.len() < raw_rows.len() {
        let cycled: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(name, _)| name.clone())
            .collect();
        for (i, (name, _)) in raw_rows.iter().enumerate() {
            if cycled.contains(name) {
                errors.push(ImportRowError {
                    row: i + 2,
                    message: format!("Circular dependency detected involving '{}'", name),
                });
            }
        }
        let failed = errors.len();
        return Ok(Json(BulkImportResponse {
            imported: 0,
            failed,
            errors,
        }));
    }

    // Pre-fetch DB parent IDs to avoid holding a pool connection inside the transaction
    let rows_map: HashMap<String, String> = raw_rows
        .iter()
        .map(|(name, parent)| (name.clone(), parent.clone()))
        .collect();
    let mut db_parent_ids: HashMap<String, String> = HashMap::new();
    for parent_name in rows_map.values() {
        if !parent_name.is_empty()
            && !csv_names.contains(parent_name.as_str())
            && !db_parent_ids.contains_key(parent_name)
        {
            let dept = dept_repo::find_department_by_name(&state.write_pool, parent_name)
                .await
                .map_err(|e| AppError::InternalServerError(e.into()))?;
            if let Some(d) = dept {
                db_parent_ids.insert(parent_name.clone(), d.id.to_string());
            }
        }
    }

    // Phase 4: Insert in topological order within a transaction
    let mut tx = state
        .write_pool
        .begin()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let mut inserted_ids: HashMap<String, String> = HashMap::new();
    for name in &sorted_names {
        let parent_name = rows_map.get(name).map(String::as_str).unwrap_or("");
        let parent_id_str: Option<String> = if parent_name.is_empty() {
            None
        } else if let Some(id) = inserted_ids.get(parent_name) {
            Some(id.clone())
        } else {
            db_parent_ids.get(parent_name).cloned()
        };

        let new_id = DepartmentId::new().to_string();
        sqlx::query("INSERT INTO departments (id, name, parent_id) VALUES ($1, $2, $3)")
            .bind(&new_id)
            .bind(name.as_str())
            .bind(parent_id_str.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::InternalServerError(e.into()))?;
        inserted_ids.insert(name.clone(), new_id);
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let count = sorted_names.len();
    Ok(Json(BulkImportResponse {
        imported: count,
        failed: 0,
        errors: Vec::new(),
    }))
}

pub async fn import_users(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(payload): Json<BulkImportRequest>,
) -> Result<Json<BulkImportResponse>, AppError> {
    if !user.is_system_admin() {
        return Err(AppError::Forbidden("Forbidden".into()));
    }

    let mut reader = ReaderBuilder::new().from_reader(payload.csv_data.as_bytes());

    {
        let headers = reader
            .headers()
            .map_err(|e| AppError::BadRequest(format!("CSV parse error: {}", e)))?;
        let header_vec: Vec<&str> = headers.iter().collect();
        let expected = [
            "username",
            "password",
            "full_name",
            "email",
            "role",
            "is_system_admin",
            "department_name",
        ];
        if header_vec.len() < expected.len() {
            return Err(AppError::BadRequest(format!(
                "Invalid CSV headers. Expected: {}",
                expected.join(",")
            )));
        }
        for (i, exp) in expected.iter().enumerate() {
            if header_vec[i].trim() != *exp {
                return Err(AppError::BadRequest(format!(
                    "Invalid CSV header at column {}: expected '{}', got '{}'",
                    i + 1,
                    exp,
                    header_vec[i].trim()
                )));
            }
        }
    }

    struct CsvUserRow {
        username: String,
        password: String,
        full_name: String,
        email: String,
        role_str: String,
        is_system_admin_str: String,
        department_name: String,
    }

    let mut raw_rows: Vec<CsvUserRow> = Vec::new();
    for (i, result) in reader.records().enumerate() {
        let record = result.map_err(|e| {
            AppError::BadRequest(format!("CSV parse error at row {}: {}", i + 2, e))
        })?;
        raw_rows.push(CsvUserRow {
            username: record.get(0).unwrap_or("").trim().to_string(),
            password: record.get(1).unwrap_or("").trim().to_string(),
            full_name: record.get(2).unwrap_or("").trim().to_string(),
            email: record.get(3).unwrap_or("").trim().to_string(),
            role_str: record.get(4).unwrap_or("").trim().to_string(),
            is_system_admin_str: record.get(5).unwrap_or("").trim().to_string(),
            department_name: record.get(6).unwrap_or("").trim().to_string(),
        });
    }

    if raw_rows.is_empty() {
        return Ok(Json(BulkImportResponse {
            imported: 0,
            failed: 0,
            errors: Vec::new(),
        }));
    }

    struct ValidRow {
        username: String,
        password: String,
        full_name: String,
        email: String,
        role: UserRole,
        is_system_admin: bool,
        department_id: Option<String>,
    }

    let mut errors: Vec<ImportRowError> = Vec::new();
    let mut csv_usernames: HashSet<String> = HashSet::new();
    let mut csv_emails: HashSet<String> = HashSet::new();
    let mut validated: Vec<Option<ValidRow>> = Vec::with_capacity(raw_rows.len());

    for (i, row) in raw_rows.iter().enumerate() {
        let row_num = i + 2;
        let mut row_ok = true;

        let role = match row.role_str.as_str() {
            "employee" => UserRole::Employee,
            "manager" => UserRole::Manager,
            other => {
                errors.push(ImportRowError {
                    row: row_num,
                    message: format!("Invalid role '{}': must be 'employee' or 'manager'", other),
                });
                row_ok = false;
                UserRole::Employee
            }
        };

        let is_system_admin = match row.is_system_admin_str.to_ascii_lowercase().as_str() {
            "true" => true,
            "false" => false,
            other => {
                errors.push(ImportRowError {
                    row: row_num,
                    message: format!(
                        "Invalid is_system_admin '{}': must be 'true' or 'false'",
                        other
                    ),
                });
                row_ok = false;
                false
            }
        };

        if row.username.is_empty() {
            errors.push(ImportRowError {
                row: row_num,
                message: "username must not be empty".into(),
            });
            row_ok = false;
        } else if !csv_usernames.insert(row.username.clone()) {
            errors.push(ImportRowError {
                row: row_num,
                message: format!("Duplicate username '{}' in CSV", row.username),
            });
            row_ok = false;
        }

        if row.full_name.is_empty() {
            errors.push(ImportRowError {
                row: row_num,
                message: "full_name must not be empty".into(),
            });
            row_ok = false;
        }

        if row.email.is_empty() {
            errors.push(ImportRowError {
                row: row_num,
                message: "email must not be empty".into(),
            });
            row_ok = false;
        } else if !row.email.contains('@') {
            errors.push(ImportRowError {
                row: row_num,
                message: format!("Invalid email format: '{}'", row.email),
            });
            row_ok = false;
        } else if !csv_emails.insert(row.email.to_ascii_lowercase()) {
            errors.push(ImportRowError {
                row: row_num,
                message: format!("Duplicate email '{}' in CSV", row.email),
            });
            row_ok = false;
        }

        if row.password.is_empty() {
            errors.push(ImportRowError {
                row: row_num,
                message: "password must not be empty".into(),
            });
            row_ok = false;
        } else if let Err(e) = validate_password_complexity(&row.password, &state.config) {
            errors.push(ImportRowError {
                row: row_num,
                message: format!("Password policy violation: {}", e),
            });
            row_ok = false;
        }

        if row_ok {
            validated.push(Some(ValidRow {
                username: row.username.clone(),
                password: row.password.clone(),
                full_name: row.full_name.clone(),
                email: row.email.clone(),
                role,
                is_system_admin,
                department_id: None,
            }));
        } else {
            validated.push(None);
        }
    }

    // DB checks for structurally-valid rows
    for (i, row) in raw_rows.iter().enumerate() {
        let row_num = i + 2;
        if validated[i].is_none() {
            continue;
        }

        match user_repo::username_exists(&state.write_pool, &row.username).await {
            Ok(true) => {
                errors.push(ImportRowError {
                    row: row_num,
                    message: format!("Username '{}' already exists", row.username),
                });
                validated[i] = None;
                continue;
            }
            Err(e) => return Err(AppError::InternalServerError(e.into())),
            Ok(false) => {}
        }

        let email_hash = hash_email(&row.email, &state.config);
        match user_repo::email_exists(&state.write_pool, &email_hash).await {
            Ok(true) => {
                errors.push(ImportRowError {
                    row: row_num,
                    message: format!("Email '{}' already exists", row.email),
                });
                validated[i] = None;
                continue;
            }
            Err(e) => return Err(AppError::InternalServerError(e.into())),
            Ok(false) => {}
        }

        if !row.department_name.is_empty() {
            match dept_repo::find_department_by_name(&state.write_pool, &row.department_name).await
            {
                Ok(Some(dept)) => {
                    if let Some(ref mut v) = validated[i] {
                        v.department_id = Some(dept.id.to_string());
                    }
                }
                Ok(None) => {
                    errors.push(ImportRowError {
                        row: row_num,
                        message: format!("Department '{}' not found", row.department_name),
                    });
                    validated[i] = None;
                    continue;
                }
                Err(e) => return Err(AppError::InternalServerError(e.into())),
            }
        }
    }

    if !errors.is_empty() {
        let failed = errors.len();
        return Ok(Json(BulkImportResponse {
            imported: 0,
            failed,
            errors,
        }));
    }

    let valid_rows: Vec<ValidRow> = validated.into_iter().flatten().collect();

    // Hash passwords (CPU-intensive, use spawn_blocking per row)
    let mut hashed_passwords: Vec<String> = Vec::with_capacity(valid_rows.len());
    for row in &valid_rows {
        let password = row.password.clone();
        let hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|_| {
                AppError::InternalServerError(anyhow::anyhow!("Password hashing task failed"))
            })?
            .map_err(|e| {
                AppError::InternalServerError(anyhow::anyhow!("Failed to hash password: {}", e))
            })?;
        hashed_passwords.push(hash);
    }

    // Insert all users in a single transaction
    let mut tx = state
        .write_pool
        .begin()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    let count = valid_rows.len();
    for (row, password_hash) in valid_rows.iter().zip(hashed_passwords.iter()) {
        let encrypted_full_name = encrypt_pii(&row.full_name, &state.config).map_err(|_| {
            AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt full_name"))
        })?;
        let encrypted_email = encrypt_pii(&row.email, &state.config).map_err(|_| {
            AppError::InternalServerError(anyhow::anyhow!("Failed to encrypt email"))
        })?;
        let email_hash = hash_email(&row.email, &state.config);
        let user_id = UserId::new();
        let now = chrono::Utc::now();
        let role_str = row.role.as_str();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, full_name_enc, email_enc, \
             email_hash, role, is_system_admin, mfa_secret_enc, mfa_enabled_at, \
             password_changed_at, failed_login_attempts, locked_until, lock_reason, \
             lockout_count, department_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, NULL, $9, 0, NULL, NULL, 0, $10, $11, $12)",
        )
        .bind(user_id.to_string())
        .bind(&row.username)
        .bind(password_hash)
        .bind(&encrypted_full_name)
        .bind(&encrypted_email)
        .bind(&email_hash)
        .bind(role_str)
        .bind(row.is_system_admin)
        .bind(now)
        .bind(row.department_id.as_deref())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalServerError(e.into()))?;

    Ok(Json(BulkImportResponse {
        imported: count,
        failed: 0,
        errors: Vec::new(),
    }))
}
