use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::repositories::consent_log;

#[derive(Debug, Clone)]
pub struct ConsentLogService {
    pool: PgPool,
}

impl ConsentLogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn delete_logs_before(&self, cutoff: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        consent_log::delete_consent_logs_before(&self.pool, cutoff).await
    }
}
