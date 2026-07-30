use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;
use timekeeper_app::{
    attendance_classification::{ClassificationReadRepository, ClassificationWorkdayMaterializer},
    settlement_balance::{
        SettlementActual, SettlementBalanceError, SettlementBalanceReadRepository, SettlementBreak,
        SettlementWorkday, SettlementWorkdayMaterializer,
    },
};

use crate::attendance_classification::{
    ClassificationPostgresRepository, PostgresWorkdayMaterializer,
};

#[derive(Debug, Clone)]
pub struct SettlementBalancePostgresRepository {
    classification: ClassificationPostgresRepository,
}

impl SettlementBalancePostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            classification: ClassificationPostgresRepository::new(pool),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettlementPostgresWorkdayMaterializer {
    classification: PostgresWorkdayMaterializer,
}

impl SettlementPostgresWorkdayMaterializer {
    pub fn new(pool: PgPool) -> Self {
        Self {
            classification: PostgresWorkdayMaterializer::new(pool),
        }
    }
}

#[async_trait]
impl SettlementBalanceReadRepository for SettlementBalancePostgresRepository {
    async fn list_workdays(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<SettlementWorkday>, SettlementBalanceError> {
        self.classification
            .list_resolved_in_range(user_id, from, to)
            .await
            .map(|workdays| {
                workdays
                    .into_iter()
                    .map(|workday| SettlementWorkday {
                        work_date: workday.work_date,
                        version_id: workday.work_schedule_version_id,
                        schedule_type: workday.schedule_type,
                        locked: workday.locked_at.is_some(),
                    })
                    .collect()
            })
            .map_err(map_error)
    }

    async fn list_actuals(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<SettlementActual>, SettlementBalanceError> {
        self.classification
            .list_day_actuals_in_range(user_id, from, to)
            .await
            .map(|actuals| {
                actuals
                    .into_iter()
                    .map(|actual| SettlementActual {
                        work_date: actual.work_date,
                        clock_in: actual.clock_in_time,
                        clock_out: actual.clock_out_time,
                        breaks: actual
                            .breaks
                            .into_iter()
                            .map(|period| SettlementBreak {
                                start: period.break_start_time,
                                end: period.break_end_time,
                            })
                            .collect(),
                    })
                    .collect()
            })
            .map_err(map_error)
    }

    async fn settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, SettlementBalanceError> {
        self.classification
            .find_settlement_minutes(version_id)
            .await
            .map_err(map_error)
    }
}

#[async_trait]
impl SettlementWorkdayMaterializer for SettlementPostgresWorkdayMaterializer {
    async fn materialize(
        &self,
        user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<(), SettlementBalanceError> {
        self.classification
            .materialize(user_id, from, to)
            .await
            .map_err(map_error)
    }
}

fn map_error(
    error: timekeeper_app::attendance_classification::MonthlyClassificationError,
) -> SettlementBalanceError {
    use timekeeper_app::attendance_classification::MonthlyClassificationError;
    match error {
        MonthlyClassificationError::InvalidYear => SettlementBalanceError::InvalidYear,
        MonthlyClassificationError::InvalidMonth => SettlementBalanceError::InvalidMonth,
        MonthlyClassificationError::Repository(message) => {
            SettlementBalanceError::Repository(message)
        }
    }
}
