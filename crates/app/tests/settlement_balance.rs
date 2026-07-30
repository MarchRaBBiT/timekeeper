use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};
use timekeeper_app::{
    settlement_balance::{
        CalculateSettlementBalance, SettlementActual, SettlementBalanceError,
        SettlementBalanceQuery, SettlementBalanceReadRepository, SettlementBalanceResult,
        SettlementBreak, SettlementWorkday, SettlementWorkdayMaterializer,
    },
    work_schedules::ScheduleType,
};

#[derive(Clone, Default)]
struct FakeRepository {
    workdays: Vec<SettlementWorkday>,
    actuals: Vec<SettlementActual>,
    settlements: HashMap<String, Option<i64>>,
}

#[async_trait]
impl SettlementBalanceReadRepository for FakeRepository {
    async fn list_workdays(
        &self,
        _user_id: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<Vec<SettlementWorkday>, SettlementBalanceError> {
        Ok(self.workdays.clone())
    }

    async fn list_actuals(
        &self,
        _user_id: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<Vec<SettlementActual>, SettlementBalanceError> {
        Ok(self.actuals.clone())
    }

    async fn settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, SettlementBalanceError> {
        Ok(self.settlements.get(version_id).copied().flatten())
    }
}

#[derive(Clone, Copy)]
struct NoopMaterializer;

#[async_trait]
impl SettlementWorkdayMaterializer for NoopMaterializer {
    async fn materialize(
        &self,
        _user_id: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<(), SettlementBalanceError> {
        Ok(())
    }
}

fn dt(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").expect("valid fixture datetime")
}

fn month_workdays(schedule_type: ScheduleType, version_id: &str) -> Vec<SettlementWorkday> {
    (1..=31)
        .map(|day| SettlementWorkday {
            work_date: NaiveDate::from_ymd_opt(2026, 7, day).expect("valid fixture date"),
            version_id: version_id.to_string(),
            schedule_type,
            locked: day == 1,
        })
        .collect()
}

async fn execute(repository: FakeRepository) -> SettlementBalanceResult {
    CalculateSettlementBalance::new(repository, NoopMaterializer)
        .execute(SettlementBalanceQuery {
            user_id: "user-1".to_string(),
            year: 2026,
            month: 7,
        })
        .await
        .expect("calculation succeeds")
}

#[tokio::test]
async fn calculates_integer_minutes_from_effective_timestamps_and_actual_breaks() {
    let repository = FakeRepository {
        workdays: month_workdays(ScheduleType::Flex, "v1"),
        actuals: vec![SettlementActual {
            work_date: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
            clock_in: Some(dt("2026-07-01 09:00")),
            clock_out: Some(dt("2026-07-01 17:31")),
            breaks: vec![SettlementBreak {
                start: dt("2026-07-01 12:00"),
                end: Some(dt("2026-07-01 12:47")),
            }],
        }],
        settlements: HashMap::from([("v1".to_string(), Some(480))]),
    };

    let SettlementBalanceResult::Calculated(result) = execute(repository).await else {
        panic!("expected calculated");
    };
    assert_eq!(result.actual_minutes, 464);
    assert_eq!(result.balance_minutes, -16);
    assert_eq!(result.days[0].actual_minutes, 464);
    assert!(result.days[0].locked);
}

#[tokio::test]
async fn in_progress_attendance_is_zero_minutes_and_flagged() {
    let repository = FakeRepository {
        workdays: month_workdays(ScheduleType::Flex, "v1"),
        actuals: vec![SettlementActual {
            work_date: NaiveDate::from_ymd_opt(2026, 7, 1).expect("date"),
            clock_in: Some(dt("2026-07-01 09:00")),
            clock_out: None,
            breaks: Vec::new(),
        }],
        settlements: HashMap::from([("v1".to_string(), Some(480))]),
    };

    let SettlementBalanceResult::Calculated(result) = execute(repository).await else {
        panic!("expected calculated");
    };
    assert_eq!(result.actual_minutes, 0);
    assert!(result.days[0].in_progress);
}

#[tokio::test]
async fn status_priority_is_unresolved_then_not_applicable_then_mixed_then_not_configured() {
    let mut unresolved = month_workdays(ScheduleType::Fixed, "v1");
    unresolved.pop();
    assert_eq!(
        execute(FakeRepository {
            workdays: unresolved,
            ..FakeRepository::default()
        })
        .await,
        SettlementBalanceResult::UnresolvedDays
    );

    let mut mixed_type = month_workdays(ScheduleType::Flex, "v1");
    mixed_type[30].schedule_type = ScheduleType::Fixed;
    mixed_type[30].version_id = "v2".to_string();
    assert_eq!(
        execute(FakeRepository {
            workdays: mixed_type,
            settlements: HashMap::from([
                ("v1".to_string(), Some(480)),
                ("v2".to_string(), Some(500)),
            ]),
            ..FakeRepository::default()
        })
        .await,
        SettlementBalanceResult::NotApplicable
    );

    let mut mixed_version = month_workdays(ScheduleType::Flex, "v1");
    mixed_version[30].version_id = "v2".to_string();
    assert_eq!(
        execute(FakeRepository {
            workdays: mixed_version,
            settlements: HashMap::from([
                ("v1".to_string(), Some(480)),
                ("v2".to_string(), Some(500)),
            ]),
            ..FakeRepository::default()
        })
        .await,
        SettlementBalanceResult::VersionMixed
    );

    assert_eq!(
        execute(FakeRepository {
            workdays: month_workdays(ScheduleType::Flex, "v1"),
            settlements: HashMap::from([("v1".to_string(), None)]),
            ..FakeRepository::default()
        })
        .await,
        SettlementBalanceResult::NotConfigured
    );
}

#[tokio::test]
async fn same_contract_value_across_versions_is_calculable() {
    let mut workdays = month_workdays(ScheduleType::Flex, "v1");
    workdays[30].version_id = "v2".to_string();
    let result = execute(FakeRepository {
        workdays,
        settlements: HashMap::from([("v1".to_string(), Some(480)), ("v2".to_string(), Some(480))]),
        ..FakeRepository::default()
    })
    .await;
    assert!(matches!(result, SettlementBalanceResult::Calculated(_)));
}

#[tokio::test]
async fn validates_year_and_month() {
    let use_case = CalculateSettlementBalance::new(FakeRepository::default(), NoopMaterializer);
    for (year, month) in [(1899, 1), (10000, 1), (2026, 0), (2026, 13)] {
        assert!(use_case
            .execute(SettlementBalanceQuery {
                user_id: "user-1".to_string(),
                year,
                month,
            })
            .await
            .is_err());
    }
}

#[tokio::test]
async fn accepts_supported_year_boundaries_including_december_9999() {
    for (year, month) in [(1900, 1), (9999, 12)] {
        let result = CalculateSettlementBalance::new(FakeRepository::default(), NoopMaterializer)
            .execute(SettlementBalanceQuery {
                user_id: "user-1".to_string(),
                year,
                month,
            })
            .await;
        assert!(result.is_ok(), "{year}-{month} must be accepted");
    }
}
