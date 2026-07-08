use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use timekeeper_app::attendance_classification::{
    ClassificationReadRepository, ClassificationWorkdayMaterializer, DayActuals, FlexPeriodStatus,
    GetMonthlyClassification, MonthlyClassification, MonthlyClassificationError,
    MonthlyClassificationQuery, WorkRuleRow,
};
use timekeeper_app::work_schedules::{
    ResolvedDayKind, ResolvedWorkday, ScheduleType, WorkScheduleSource,
};
use timekeeper_domain::attendance_classification::WorkRuleParameters;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    date(year, month, day)
        .and_hms_opt(hour, minute, 0)
        .expect("valid datetime")
}

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid time")
}

fn resolved_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_783_396_800, 0).expect("resolved at")
}

fn default_rule_row() -> WorkRuleRow {
    WorkRuleRow {
        valid_from: date(1900, 1, 1),
        parameters: WorkRuleParameters {
            statutory_daily_minutes: 480,
            statutory_weekly_minutes: 2400,
            night_start: time(22, 0),
            night_end: time(5, 0),
            week_start_weekday: 7,
            legal_holiday_weekday: 7,
        },
    }
}

fn workday(
    work_date: NaiveDate,
    day_kind: ResolvedDayKind,
    schedule_type: ScheduleType,
    version_id: &str,
    expected: i32,
) -> ResolvedWorkday {
    ResolvedWorkday {
        id: format!("resolved-{work_date}"),
        user_id: "user-1".to_string(),
        work_date,
        work_schedule_id: "schedule-1".to_string(),
        work_schedule_version_id: version_id.to_string(),
        source: WorkScheduleSource::User,
        source_id: "assignment-1".to_string(),
        day_kind,
        timezone: "Asia/Tokyo".to_string(),
        workday_boundary: time(5, 0),
        expected_work_minutes: expected,
        work_intervals: Vec::new(),
        planned_breaks: Vec::new(),
        schedule_type,
        core_time_windows: Vec::new(),
        resolved_at: resolved_at(),
        locked_at: None,
    }
}

/// 月-金 working / 土日 non-working の fixed 月を丸ごと resolve 済みにする。
fn fixed_month(year: i32, month: u32, version_id: &str, expected: i32) -> Vec<ResolvedWorkday> {
    let mut days = Vec::new();
    let mut current = date(year, month, 1);
    while current.month() == month {
        let weekday = current.weekday().number_from_monday();
        let day_kind = if weekday >= 6 {
            ResolvedDayKind::ScheduledNonWorkingDay
        } else {
            ResolvedDayKind::ScheduledWorkday
        };
        let expected = if weekday >= 6 { 0 } else { expected };
        days.push(workday(
            current,
            day_kind,
            ScheduleType::Fixed,
            version_id,
            expected,
        ));
        current += Duration::days(1);
    }
    days
}

fn flex_month(year: i32, month: u32, version_id: &str) -> Vec<ResolvedWorkday> {
    fixed_month(year, month, version_id, 480)
        .into_iter()
        .map(|mut day| {
            day.schedule_type = ScheduleType::Flex;
            day
        })
        .collect()
}

fn full_day_actuals(work_date: NaiveDate, start_hour: u32, end_hour: u32) -> DayActuals {
    DayActuals {
        work_date,
        clock_in_time: Some(
            work_date
                .and_hms_opt(start_hour, 0, 0)
                .expect("clock in time"),
        ),
        clock_out_time: Some(
            work_date
                .and_hms_opt(end_hour, 0, 0)
                .expect("clock out time"),
        ),
        breaks: Vec::new(),
    }
}

#[derive(Default, Clone)]
struct FakeRepository {
    resolved: Vec<ResolvedWorkday>,
    actuals: Vec<DayActuals>,
    settlement_by_version: HashMap<String, i64>,
    rules: Vec<WorkRuleRow>,
}

#[async_trait]
impl ClassificationReadRepository for FakeRepository {
    async fn list_resolved_in_range(
        &self,
        _user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ResolvedWorkday>, MonthlyClassificationError> {
        Ok(self
            .resolved
            .iter()
            .filter(|workday| workday.work_date >= from && workday.work_date <= to)
            .cloned()
            .collect())
    }

    async fn list_day_actuals_in_range(
        &self,
        _user_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DayActuals>, MonthlyClassificationError> {
        Ok(self
            .actuals
            .iter()
            .filter(|actual| actual.work_date >= from && actual.work_date <= to)
            .cloned()
            .collect())
    }

    async fn find_settlement_minutes(
        &self,
        version_id: &str,
    ) -> Result<Option<i64>, MonthlyClassificationError> {
        Ok(self.settlement_by_version.get(version_id).copied())
    }

    async fn list_work_rules_effective_until(
        &self,
        until: NaiveDate,
    ) -> Result<Vec<WorkRuleRow>, MonthlyClassificationError> {
        Ok(self
            .rules
            .iter()
            .filter(|row| row.valid_from <= until)
            .copied()
            .collect())
    }
}

struct NoopMaterializer;

#[async_trait]
impl ClassificationWorkdayMaterializer for NoopMaterializer {
    async fn materialize(
        &self,
        _user_id: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<(), MonthlyClassificationError> {
        Ok(())
    }
}

fn use_case(
    repository: FakeRepository,
) -> GetMonthlyClassification<FakeRepository, NoopMaterializer> {
    GetMonthlyClassification::new(repository, NoopMaterializer)
}

fn query(year: i32, month: u32) -> MonthlyClassificationQuery {
    MonthlyClassificationQuery {
        user_id: "user-1".to_string(),
        year,
        month,
    }
}

#[tokio::test]
async fn rejects_out_of_range_year_and_month() {
    let use_case = use_case(FakeRepository {
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    assert_eq!(
        use_case.execute(query(1899, 7)).await,
        Err(MonthlyClassificationError::InvalidYear)
    );
    assert_eq!(
        use_case.execute(query(10000, 7)).await,
        Err(MonthlyClassificationError::InvalidYear)
    );
    assert_eq!(
        use_case.execute(query(2026, 0)).await,
        Err(MonthlyClassificationError::InvalidMonth)
    );
    assert_eq!(
        use_case.execute(query(2026, 13)).await,
        Err(MonthlyClassificationError::InvalidMonth)
    );
}

#[tokio::test]
async fn missing_work_rules_fail_closed() {
    let use_case = use_case(FakeRepository {
        resolved: fixed_month(2026, 7, "v1", 420),
        ..FakeRepository::default()
    });

    assert_eq!(
        use_case.execute(query(2026, 7)).await,
        Ok(MonthlyClassification::WorkRuleNotConfigured)
    );
}

#[tokio::test]
async fn work_rules_starting_mid_window_fail_closed() {
    // 有効行が月初以降にしかない → 窓の先頭（前月末を含む週）が未設定期間。
    let use_case = use_case(FakeRepository {
        resolved: fixed_month(2026, 7, "v1", 420),
        rules: vec![WorkRuleRow {
            valid_from: date(2026, 7, 1),
            ..default_rule_row()
        }],
        ..FakeRepository::default()
    });

    assert_eq!(
        use_case.execute(query(2026, 7)).await,
        Ok(MonthlyClassification::WorkRuleNotConfigured)
    );
}

#[tokio::test]
async fn classifies_a_fixed_month_and_sums_totals() {
    // 7/6(月) 9:00-18:00 無休憩 = 540分: 所定 420 / 法定内 60 / 法定外 60。
    let actuals = vec![full_day_actuals(date(2026, 7, 6), 9, 18)];
    let use_case = use_case(FakeRepository {
        resolved: fixed_month(2026, 7, "v1", 420),
        actuals,
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.days.len(), 31);
    let monday = calculated
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 7, 6))
        .expect("monday");
    assert_eq!(monday.actual_minutes, 540);
    assert_eq!(monday.scheduled_minutes, 420);
    assert_eq!(monday.statutory_within_minutes, 60);
    assert_eq!(monday.statutory_excess_minutes, 60);
    assert_eq!(calculated.totals.actual_minutes, 540);
    assert_eq!(calculated.totals.statutory_excess_minutes, 60);
    assert_eq!(calculated.flex_period, FlexPeriodStatus::NotApplicable);
}

#[tokio::test]
async fn adjacent_month_days_consume_the_weekly_quota_consistently() {
    // 2026-08-01 は土曜。7/26(日)起算の週に 7/27-7/31 で 2400 分積むと、
    // 8 月から見ても 8/1 の労働は全量週次法定外になる（月跨ぎ週の不変条件）。
    let mut resolved = fixed_month(2026, 7, "v1", 420);
    resolved.extend(fixed_month(2026, 8, "v1", 420));
    let mut actuals: Vec<DayActuals> = (27..=31)
        .map(|day| full_day_actuals(date(2026, 7, day), 9, 17))
        .collect();
    actuals.push(full_day_actuals(date(2026, 8, 1), 9, 13));

    let repository = FakeRepository {
        resolved,
        actuals,
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    };
    let use_case = use_case(repository);

    let august = use_case.execute(query(2026, 8)).await.expect("august");
    let MonthlyClassification::Calculated(august) = august else {
        panic!("expected calculated august, got {august:?}");
    };
    let first = august
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 8, 1))
        .expect("aug 1");
    assert_eq!(first.actual_minutes, 240);
    assert_eq!(first.statutory_excess_minutes, 240);
    // 応答には対象月の日だけが含まれる。
    assert!(august.days.iter().all(|day| day.work_date.month() == 8));

    // 7 月側から見ても 7/31 の区分は変わらない（週内で 2400 丁度 → 法定外 0）。
    let july = use_case.execute(query(2026, 7)).await.expect("july");
    let MonthlyClassification::Calculated(july) = july else {
        panic!("expected calculated july, got {july:?}");
    };
    let last = july
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 7, 31))
        .expect("jul 31");
    assert_eq!(last.statutory_excess_minutes, 0);
}

#[tokio::test]
async fn in_progress_attendance_counts_zero_minutes_and_is_flagged() {
    let use_case = use_case(FakeRepository {
        resolved: fixed_month(2026, 7, "v1", 420),
        actuals: vec![DayActuals {
            work_date: date(2026, 7, 6),
            clock_in_time: Some(at(2026, 7, 6, 9, 0)),
            clock_out_time: None,
            breaks: Vec::new(),
        }],
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    let monday = calculated
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 7, 6))
        .expect("monday");
    assert!(monday.in_progress);
    assert_eq!(monday.actual_minutes, 0);
}

#[tokio::test]
async fn unresolved_day_with_positive_work_fails_closed() {
    // 7/6 の resolved workday を落とし、その日に実績を置く。
    let resolved: Vec<ResolvedWorkday> = fixed_month(2026, 7, "v1", 420)
        .into_iter()
        .filter(|day| day.work_date != date(2026, 7, 6))
        .collect();
    let use_case = use_case(FakeRepository {
        resolved,
        actuals: vec![full_day_actuals(date(2026, 7, 6), 9, 17)],
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    assert_eq!(
        use_case.execute(query(2026, 7)).await,
        Ok(MonthlyClassification::UnresolvedDays)
    );
}

#[tokio::test]
async fn unresolved_day_without_work_is_treated_as_zero_and_month_still_calculates() {
    let resolved: Vec<ResolvedWorkday> = fixed_month(2026, 7, "v1", 420)
        .into_iter()
        .filter(|day| day.work_date != date(2026, 7, 6))
        .collect();
    let use_case = use_case(FakeRepository {
        resolved,
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.days.len(), 30);
    assert!(calculated
        .days
        .iter()
        .all(|day| day.work_date != date(2026, 7, 6)));
}

#[tokio::test]
async fn flex_month_calculates_period_partition() {
    let mut repository = FakeRepository {
        resolved: flex_month(2026, 7, "v1"),
        actuals: vec![
            full_day_actuals(date(2026, 7, 6), 9, 17),
            full_day_actuals(date(2026, 7, 7), 9, 17),
        ],
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    };
    repository
        .settlement_by_version
        .insert("v1".to_string(), 9600);
    let use_case = use_case(repository);

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    // flex 日の日次 partition は 0、actual のみ。
    let monday = calculated
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 7, 6))
        .expect("monday");
    assert_eq!(monday.actual_minutes, 480);
    assert_eq!(monday.scheduled_minutes, 0);
    let FlexPeriodStatus::Calculated(period) = calculated.flex_period else {
        panic!(
            "expected calculated flex period, got {:?}",
            calculated.flex_period
        );
    };
    assert_eq!(period.contracted_minutes, 9600);
    assert_eq!(period.statutory_frame_minutes, 2400 * 31 / 7);
    assert_eq!(period.actual_minutes, 960);
    assert_eq!(period.scheduled_minutes, 960);
    assert_eq!(period.statutory_excess_minutes, 0);
}

#[tokio::test]
async fn flex_period_excludes_legal_holiday_minutes_from_actual() {
    let mut repository = FakeRepository {
        resolved: flex_month(2026, 7, "v1"),
        actuals: vec![
            full_day_actuals(date(2026, 7, 6), 9, 17),
            // 7/5 は日曜（法定休日、non-working）。
            full_day_actuals(date(2026, 7, 5), 10, 12),
        ],
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    };
    repository
        .settlement_by_version
        .insert("v1".to_string(), 9600);
    let use_case = use_case(repository);

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.totals.legal_holiday_minutes, 120);
    let FlexPeriodStatus::Calculated(period) = calculated.flex_period else {
        panic!(
            "expected calculated flex period, got {:?}",
            calculated.flex_period
        );
    };
    assert_eq!(period.actual_minutes, 480);
}

#[tokio::test]
async fn mixed_fixed_and_flex_month_reports_not_applicable_but_classifies_fixed_days() {
    let mut resolved = flex_month(2026, 7, "v1");
    for day in resolved.iter_mut().filter(|day| day.work_date.day() <= 15) {
        day.schedule_type = ScheduleType::Fixed;
    }
    let use_case = use_case(FakeRepository {
        resolved,
        actuals: vec![full_day_actuals(date(2026, 7, 6), 9, 17)],
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.flex_period, FlexPeriodStatus::NotApplicable);
    let monday = calculated
        .days
        .iter()
        .find(|day| day.work_date == date(2026, 7, 6))
        .expect("monday");
    // fixed 日は通常どおり日次区分される（expected 480 の勤務日で 480 分勤務）。
    assert_eq!(monday.scheduled_minutes, 480);
    assert_eq!(monday.statutory_excess_minutes, 0);
}

#[tokio::test]
async fn flex_month_with_unresolved_day_reports_unresolved_flex_period() {
    let resolved: Vec<ResolvedWorkday> = flex_month(2026, 7, "v1")
        .into_iter()
        .filter(|day| day.work_date != date(2026, 7, 6))
        .collect();
    let mut repository = FakeRepository {
        resolved,
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    };
    repository
        .settlement_by_version
        .insert("v1".to_string(), 9600);
    let use_case = use_case(repository);

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.flex_period, FlexPeriodStatus::UnresolvedDays);
}

#[tokio::test]
async fn flex_month_with_mismatched_settlement_values_reports_version_mixed() {
    let mut resolved = flex_month(2026, 7, "v1");
    for day in resolved.iter_mut().filter(|day| day.work_date.day() > 15) {
        day.work_schedule_version_id = "v2".to_string();
    }
    let mut repository = FakeRepository {
        resolved,
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    };
    repository
        .settlement_by_version
        .insert("v1".to_string(), 9600);
    repository
        .settlement_by_version
        .insert("v2".to_string(), 9000);
    let use_case = use_case(repository);

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.flex_period, FlexPeriodStatus::VersionMixed);
}

#[tokio::test]
async fn flex_month_without_settlement_row_reports_not_configured() {
    let use_case = use_case(FakeRepository {
        resolved: flex_month(2026, 7, "v1"),
        rules: vec![default_rule_row()],
        ..FakeRepository::default()
    });

    let result = use_case.execute(query(2026, 7)).await.expect("calculated");
    let MonthlyClassification::Calculated(calculated) = result else {
        panic!("expected calculated, got {result:?}");
    };
    assert_eq!(calculated.flex_period, FlexPeriodStatus::NotConfigured);
}
