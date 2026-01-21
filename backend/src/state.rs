use crate::{config::Config, db::connection::DbPool};

#[derive(Clone)]
pub struct AppState {
    pub write_pool: DbPool,
    pub read_pool: Option<DbPool>,
    pub config: Config,
}

impl AppState {
    pub fn new(write_pool: DbPool, read_pool: Option<DbPool>, config: Config) -> Self {
        Self {
            write_pool,
            read_pool,
            config,
        }
    }

    /// Returns the read pool if configured, otherwise falls back to the write pool.
    /// Use this for SELECT queries that don't require read-after-write consistency.
    pub fn read_pool(&self) -> &DbPool {
        if self.config.feature_read_replica_enabled {
            self.read_pool.as_ref().unwrap_or(&self.write_pool)
        } else {
            &self.write_pool
        }
    }

    /// For backward compatibility: returns (write_pool, config) tuple.
    /// Deprecated - use AppState directly instead.
    pub fn into_parts(self) -> (DbPool, Config) {
        (self.write_pool, self.config)
    }

    /// For backward compatibility: returns (write_pool, config) tuple.
    /// Deprecated - use AppState directly instead.
    pub fn as_tuple(&self) -> (DbPool, Config) {
        (self.write_pool.clone(), self.config.clone())
    }
}

impl From<AppState> for (DbPool, Config) {
    fn from(state: AppState) -> Self {
        state.into_parts()
    }
}

impl From<(DbPool, Config)> for AppState {
    fn from((db_pool, config): (DbPool, Config)) -> Self {
        Self::new(db_pool, None, config)
    }
}
