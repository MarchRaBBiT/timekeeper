use crate::{config::Config, db::connection::DbPool};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub config: Config,
}

impl AppState {
    pub fn new(db_pool: DbPool, config: Config) -> Self {
        Self { db_pool, config }
    }

    pub fn into_parts(self) -> (DbPool, Config) {
        (self.db_pool, self.config)
    }

    pub fn as_tuple(&self) -> (DbPool, Config) {
        (self.db_pool.clone(), self.config.clone())
    }
}

impl From<AppState> for (DbPool, Config) {
    fn from(state: AppState) -> Self {
        state.into_parts()
    }
}

impl From<(DbPool, Config)> for AppState {
    fn from((db_pool, config): (DbPool, Config)) -> Self {
        Self::new(db_pool, config)
    }
}
