use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use timekeeper_domain::attendance_classification::{
    build_actual_intervals, classify_days, classify_flex_period, statutory_period_frame_minutes,
    week_start_date, ActualWorkInterval, ClassificationDayInput, WorkRuleParameters,
};
use timekeeper_domain::work_schedules::{ResolvedDayKind, ScheduleType};

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

fn default_rules() -> WorkRuleParameters {
    WorkRuleParameters {
        statutory_daily_minutes: 480,
        statutory_weekly_minutes: 2400,
        night_start: time(22, 0),
        night_end: time(5, 0),
        week_start_weekday: 7,
        legal_holiday_weekday: 7,
    }
}

fn workday(
    work_date: NaiveDate,
    expected: i64,
    intervals: Vec<ActualWorkInterval>,
) -> ClassificationDayInput {
    ClassificationDayInput {
        work_date,
        day_kind: ResolvedDayKind::ScheduledWorkday,
        schedule_type: ScheduleType::Fixed,
        expected_work_minutes: expected,
        intervals,
        rules: default_rules(),
    }
}

fn non_working_day(
    work_date: NaiveDate,
    intervals: Vec<ActualWorkInterval>,
) -> ClassificationDayInput {
    ClassificationDayInput {
        work_date,
        day_kind: ResolvedDayKind::ScheduledNonWorkingDay,
        schedule_type: ScheduleType::Fixed,
        expected_work_minutes: 0,
        intervals,
        rules: default_rules(),
    }
}

fn interval(start: NaiveDateTime, end: NaiveDateTime) -> ActualWorkInterval {
    ActualWorkInterval { start, end }
}

// --- build_actual_intervals -------------------------------------------------

#[test]
fn build_intervals_subtracts_completed_breaks_at_their_actual_time() {
    let intervals = build_actual_intervals(
        Some(at(2026, 7, 6, 9, 0)),
        Some(at(2026, 7, 6, 18, 0)),
        &[(at(2026, 7, 6, 12, 0), Some(at(2026, 7, 6, 13, 0)))],
    );

    assert_eq!(
        intervals,
        vec![
            interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 12, 0)),
            interval(at(2026, 7, 6, 13, 0), at(2026, 7, 6, 18, 0)),
        ]
    );
}

#[test]
fn build_intervals_ignores_open_breaks_and_clips_out_of_range_breaks() {
    let intervals = build_actual_intervals(
        Some(at(2026, 7, 6, 9, 0)),
        Some(at(2026, 7, 6, 12, 0)),
        &[
            (at(2026, 7, 6, 10, 0), None),
            (at(2026, 7, 6, 11, 30), Some(at(2026, 7, 6, 12, 30))),
        ],
    );

    assert_eq!(
        intervals,
        vec![interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 11, 30)),]
    );
}

#[test]
fn build_intervals_merges_overlapping_breaks() {
    let intervals = build_actual_intervals(
        Some(at(2026, 7, 6, 9, 0)),
        Some(at(2026, 7, 6, 18, 0)),
        &[
            (at(2026, 7, 6, 12, 0), Some(at(2026, 7, 6, 13, 0))),
            (at(2026, 7, 6, 12, 30), Some(at(2026, 7, 6, 13, 30))),
        ],
    );

    assert_eq!(
        intervals,
        vec![
            interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 12, 0)),
            interval(at(2026, 7, 6, 13, 30), at(2026, 7, 6, 18, 0)),
        ]
    );
}

#[test]
fn build_intervals_returns_empty_for_in_progress_attendance() {
    assert!(build_actual_intervals(Some(at(2026, 7, 6, 9, 0)), None, &[]).is_empty());
    assert!(build_actual_intervals(None, Some(at(2026, 7, 6, 18, 0)), &[]).is_empty());
}

#[test]
fn build_intervals_returns_empty_when_clock_out_is_not_after_clock_in() {
    assert!(
        build_actual_intervals(Some(at(2026, 7, 6, 18, 0)), Some(at(2026, 7, 6, 9, 0)), &[])
            .is_empty()
    );
}

// --- night overlay (Decision 3) ----------------------------------------------

#[test]
fn night_minutes_start_exactly_at_night_start_and_end_exactly_at_night_end() {
    // 22:00 丁度開始 → 22:00-24:00 + 0:00-5:00 の全帯が深夜。5:00 丁度終了。
    let days = vec![workday(
        date(2026, 7, 6),
        480,
        vec![interval(at(2026, 7, 6, 22, 0), at(2026, 7, 7, 5, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].night_minutes, 7 * 60);
    assert_eq!(result[0].actual_minutes, 7 * 60);
}

#[test]
fn night_minutes_exclude_breaks_taken_inside_the_night_band() {
    let days = vec![workday(
        date(2026, 7, 6),
        480,
        build_actual_intervals(
            Some(at(2026, 7, 6, 21, 0)),
            Some(at(2026, 7, 7, 1, 0)),
            &[(at(2026, 7, 6, 23, 0), Some(at(2026, 7, 6, 23, 30)))],
        ),
    )];

    let result = classify_days(&days);
    // 22:00-1:00 = 180分のうち休憩30分を除いた150分が深夜。
    assert_eq!(result[0].night_minutes, 150);
    assert_eq!(result[0].actual_minutes, 210);
}

#[test]
fn night_minutes_stop_at_five_am_for_work_continuing_past_the_band() {
    let days = vec![workday(
        date(2026, 7, 6),
        480,
        vec![interval(at(2026, 7, 7, 4, 0), at(2026, 7, 7, 7, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].night_minutes, 60);
}

// --- daily statutory judgment (Decision 4) -----------------------------------

#[test]
fn daily_exactly_at_statutory_limit_has_no_excess() {
    let days = vec![workday(
        date(2026, 7, 6),
        420,
        vec![interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 17, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].actual_minutes, 480);
    assert_eq!(result[0].statutory_excess_minutes, 0);
    assert_eq!(result[0].scheduled_minutes, 420);
    assert_eq!(result[0].statutory_within_minutes, 60);
}

#[test]
fn daily_one_minute_over_statutory_limit_is_excess() {
    let days = vec![workday(
        date(2026, 7, 6),
        480,
        vec![interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 17, 1))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].actual_minutes, 481);
    assert_eq!(result[0].statutory_excess_minutes, 1);
    assert_eq!(result[0].scheduled_minutes, 480);
    assert_eq!(result[0].statutory_within_minutes, 0);
}

#[test]
fn overnight_shift_is_judged_as_one_continuous_workday() {
    // 夜勤: 20:00-翌7:00（休憩1h）= 実労働 600分。暦日で切らず一体で判定する。
    let days = vec![workday(
        date(2026, 7, 31),
        480,
        build_actual_intervals(
            Some(at(2026, 7, 31, 20, 0)),
            Some(at(2026, 8, 1, 7, 0)),
            &[(at(2026, 8, 1, 0, 0), Some(at(2026, 8, 1, 1, 0)))],
        ),
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].actual_minutes, 600);
    assert_eq!(result[0].statutory_excess_minutes, 120);
    assert_eq!(result[0].scheduled_minutes, 480);
    // 深夜帯 22:00-5:00 のうち 0:00-1:00 は休憩 → 6時間。
    assert_eq!(result[0].night_minutes, 360);
    // 帰属は work_date = 7/31（区分値は当日行にすべて載る）。
    assert_eq!(result[0].work_date, date(2026, 7, 31));
}

// --- weekly statutory judgment (Decision 5) ----------------------------------

fn full_week_of_480(days_of_week: &[NaiveDate]) -> Vec<ClassificationDayInput> {
    days_of_week
        .iter()
        .map(|&d| {
            workday(
                d,
                420,
                vec![interval(
                    d.and_hms_opt(9, 0, 0).expect("time"),
                    d.and_hms_opt(17, 0, 0).expect("time"),
                )],
            )
        })
        .collect()
}

#[test]
fn weekly_exactly_2400_minutes_has_no_weekly_excess() {
    // 2026-07-05 は日曜（週起算）。月-金 480分 × 5 = 2400 丁度。
    let mut days = full_week_of_480(&[
        date(2026, 7, 6),
        date(2026, 7, 7),
        date(2026, 7, 8),
        date(2026, 7, 9),
        date(2026, 7, 10),
    ]);
    days.sort_by_key(|d| d.work_date);

    let result = classify_days(&days);
    assert!(result.iter().all(|d| d.statutory_excess_minutes == 0));
}

#[test]
fn weekly_excess_lands_on_the_day_where_cumulative_crosses_the_limit() {
    // 月-金 2400分 + 土曜（所定休日）240分 → 土曜の240分は全量週次法定外。
    let mut days = full_week_of_480(&[
        date(2026, 7, 6),
        date(2026, 7, 7),
        date(2026, 7, 8),
        date(2026, 7, 9),
        date(2026, 7, 10),
    ]);
    days.push(non_working_day(
        date(2026, 7, 11),
        vec![interval(at(2026, 7, 11, 9, 0), at(2026, 7, 11, 13, 0))],
    ));

    let result = classify_days(&days);
    let saturday = result.last().expect("saturday");
    assert_eq!(saturday.work_date, date(2026, 7, 11));
    assert_eq!(saturday.actual_minutes, 240);
    assert_eq!(saturday.statutory_excess_minutes, 240);
    assert_eq!(saturday.scheduled_minutes, 0);
    assert_eq!(saturday.legal_holiday_minutes, 0);
}

#[test]
fn daily_excess_does_not_count_toward_weekly_accumulation() {
    // 月曜 500分（20分は日次法定外）→ 週累積には480分のみ。
    // 火-金 480×4 = 1920 → 累積 2400 丁度で金曜に週次法定外なし。
    let mut days = vec![workday(
        date(2026, 7, 6),
        420,
        vec![interval(at(2026, 7, 6, 9, 0), at(2026, 7, 6, 17, 20))],
    )];
    days.extend(full_week_of_480(&[
        date(2026, 7, 7),
        date(2026, 7, 8),
        date(2026, 7, 9),
        date(2026, 7, 10),
    ]));

    let result = classify_days(&days);
    assert_eq!(result[0].statutory_excess_minutes, 20);
    let friday = result.last().expect("friday");
    assert_eq!(friday.statutory_excess_minutes, 0);
}

#[test]
fn legal_holiday_work_is_excluded_from_weekly_accumulation() {
    // 日曜（法定休日）120分 + 月-金 2400分 → 金曜に週次法定外が出ないこと。
    let mut days = vec![non_working_day(
        date(2026, 7, 5),
        vec![interval(at(2026, 7, 5, 10, 0), at(2026, 7, 5, 12, 0))],
    )];
    days.extend(full_week_of_480(&[
        date(2026, 7, 6),
        date(2026, 7, 7),
        date(2026, 7, 8),
        date(2026, 7, 9),
        date(2026, 7, 10),
    ]));

    let result = classify_days(&days);
    assert_eq!(result[0].legal_holiday_minutes, 120);
    assert_eq!(result[0].statutory_excess_minutes, 0);
    let friday = result.last().expect("friday");
    assert_eq!(friday.statutory_excess_minutes, 0);
}

#[test]
fn weekly_accumulation_resets_even_when_the_week_start_day_is_absent() {
    // 前週金曜 2400分相当を積んだ後、日曜（週起算日）のデータが無くても
    // 翌週月曜で累積がリセットされること。
    let days = vec![
        workday(
            date(2026, 7, 10),
            420,
            // 金曜に 2400 分相当の極端な勤務（週累積を使い切る）
            vec![interval(at(2026, 7, 10, 0, 0), at(2026, 7, 11, 16, 0))],
        ),
        workday(
            date(2026, 7, 13),
            420,
            vec![interval(at(2026, 7, 13, 9, 0), at(2026, 7, 13, 17, 0))],
        ),
    ];

    let result = classify_days(&days);
    // 月曜 480分は新しい週として週次法定外なし（日次480以内）。
    let monday = &result[1];
    assert_eq!(monday.work_date, date(2026, 7, 13));
    assert_eq!(monday.statutory_excess_minutes, 0);
}

#[test]
fn week_start_date_honours_configured_start_weekday() {
    // 2026-07-08 は水曜。
    assert_eq!(week_start_date(date(2026, 7, 8), 7), date(2026, 7, 5)); // 日曜起算
    assert_eq!(week_start_date(date(2026, 7, 8), 1), date(2026, 7, 6)); // 月曜起算
    assert_eq!(week_start_date(date(2026, 7, 8), 3), date(2026, 7, 8)); // 水曜起算（当日）
}

#[test]
fn changing_week_start_weekday_resets_accumulation_at_the_new_boundary() {
    // 月曜起算の規則で月火 960分、水曜から水曜起算の規則 → 水曜で週リセット。
    let monday_rules = WorkRuleParameters {
        week_start_weekday: 1,
        ..default_rules()
    };
    let wednesday_rules = WorkRuleParameters {
        week_start_weekday: 3,
        ..default_rules()
    };
    let mut monday = workday(
        date(2026, 7, 6),
        420,
        vec![interval(at(2026, 7, 6, 0, 0), at(2026, 7, 7, 0, 0))],
    );
    monday.rules = monday_rules;
    let mut tuesday = workday(
        date(2026, 7, 7),
        420,
        vec![interval(at(2026, 7, 7, 0, 0), at(2026, 7, 8, 0, 0))],
    );
    tuesday.rules = monday_rules;
    let mut wednesday = workday(
        date(2026, 7, 8),
        420,
        vec![interval(at(2026, 7, 8, 9, 0), at(2026, 7, 8, 17, 0))],
    );
    wednesday.rules = wednesday_rules;

    let result = classify_days(&[monday, tuesday, wednesday]);
    // 水曜は新しい週の初日 → 週次法定外 0（日次480以内）。
    assert_eq!(result[2].statutory_excess_minutes, 0);
}

// --- legal holiday (Decision 6) ------------------------------------------------

#[test]
fn work_on_designated_weekday_non_working_day_is_legal_holiday() {
    let days = vec![non_working_day(
        date(2026, 7, 5), // 日曜
        vec![interval(at(2026, 7, 5, 9, 0), at(2026, 7, 5, 19, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].legal_holiday_minutes, 600);
    assert_eq!(result[0].statutory_excess_minutes, 0);
    assert_eq!(result[0].scheduled_minutes, 0);
    assert_eq!(result[0].statutory_within_minutes, 0);
}

#[test]
fn work_on_designated_weekday_that_is_scheduled_workday_is_classified_normally() {
    // 指定曜日（日曜）が勤務予定日の場合は通常判定（Decision 6 第一増分）。
    let days = vec![workday(
        date(2026, 7, 5),
        420,
        vec![interval(at(2026, 7, 5, 9, 0), at(2026, 7, 5, 17, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].legal_holiday_minutes, 0);
    assert_eq!(result[0].scheduled_minutes, 420);
    assert_eq!(result[0].statutory_within_minutes, 60);
}

#[test]
fn public_holiday_on_designated_weekday_counts_as_legal_holiday() {
    let mut day = non_working_day(
        date(2026, 7, 5),
        vec![interval(at(2026, 7, 5, 9, 0), at(2026, 7, 5, 12, 0))],
    );
    day.day_kind = ResolvedDayKind::PublicHoliday;

    let result = classify_days(&[day]);
    assert_eq!(result[0].legal_holiday_minutes, 180);
}

#[test]
fn work_on_non_designated_non_working_day_is_judged_normally_with_zero_expected() {
    // 所定休日（土曜）の休日出勤は legal_holiday にせず通常判定。
    let days = vec![non_working_day(
        date(2026, 7, 11),
        vec![interval(at(2026, 7, 11, 9, 0), at(2026, 7, 11, 13, 0))],
    )];

    let result = classify_days(&days);
    assert_eq!(result[0].legal_holiday_minutes, 0);
    assert_eq!(result[0].scheduled_minutes, 0);
    assert_eq!(result[0].statutory_within_minutes, 240);
    assert_eq!(result[0].statutory_excess_minutes, 0);
}

// --- flex (Decision 8) ----------------------------------------------------------

#[test]
fn flex_days_have_no_daily_partition_but_keep_actual_and_night() {
    let mut day = workday(
        date(2026, 7, 6),
        540,
        vec![interval(at(2026, 7, 6, 15, 0), at(2026, 7, 6, 23, 0))],
    );
    day.schedule_type = ScheduleType::Flex;

    let result = classify_days(&[day]);
    assert_eq!(result[0].actual_minutes, 480);
    assert_eq!(result[0].night_minutes, 60);
    assert_eq!(result[0].scheduled_minutes, 0);
    assert_eq!(result[0].statutory_within_minutes, 0);
    assert_eq!(result[0].statutory_excess_minutes, 0);
}

#[test]
fn flex_day_on_legal_holiday_is_still_legal_holiday() {
    let mut day = non_working_day(
        date(2026, 7, 5),
        vec![interval(at(2026, 7, 5, 9, 0), at(2026, 7, 5, 12, 0))],
    );
    day.schedule_type = ScheduleType::Flex;

    let result = classify_days(&[day]);
    assert_eq!(result[0].legal_holiday_minutes, 180);
}

#[test]
fn statutory_period_frame_is_floored() {
    // 2400 × 31 / 7 = 10628.57… → 10628
    assert_eq!(statutory_period_frame_minutes(2400, 31), 10628);
    // 2400 × 28 / 7 = 9600 丁度
    assert_eq!(statutory_period_frame_minutes(2400, 28), 9600);
}

#[test]
fn flex_period_exactly_at_frame_has_no_excess() {
    let result = classify_flex_period(9600, 9600, 9600);
    assert_eq!(result.statutory_excess_minutes, 0);
    assert_eq!(result.scheduled_minutes, 9600);
    assert_eq!(result.statutory_within_minutes, 0);
}

#[test]
fn flex_period_over_frame_splits_scheduled_within_and_excess() {
    let result = classify_flex_period(10800, 9600, 10628);
    assert_eq!(result.statutory_excess_minutes, 172);
    assert_eq!(result.scheduled_minutes, 9600);
    assert_eq!(result.statutory_within_minutes, 1028);
    assert_eq!(
        result.scheduled_minutes
            + result.statutory_within_minutes
            + result.statutory_excess_minutes,
        10800
    );
}

#[test]
fn flex_period_with_contract_above_frame_never_returns_negative_values() {
    // 誤設定: 契約所定 > 法定総枠。
    let result = classify_flex_period(10700, 11000, 10628);
    assert_eq!(result.statutory_excess_minutes, 72);
    assert_eq!(result.scheduled_minutes, 10628);
    assert_eq!(result.statutory_within_minutes, 0);
}

// --- consistency invariant (Decision 2) ------------------------------------------

#[test]
fn partition_sums_equal_actual_minutes_for_every_day() {
    let mut days = full_week_of_480(&[
        date(2026, 7, 6),
        date(2026, 7, 7),
        date(2026, 7, 8),
        date(2026, 7, 9),
        date(2026, 7, 10),
    ]);
    days.push(non_working_day(
        date(2026, 7, 5),
        vec![interval(at(2026, 7, 5, 10, 0), at(2026, 7, 5, 14, 0))],
    ));
    days.push(non_working_day(
        date(2026, 7, 11),
        vec![interval(at(2026, 7, 11, 20, 0), at(2026, 7, 12, 4, 0))],
    ));

    for day in classify_days(&days) {
        assert_eq!(
            day.scheduled_minutes
                + day.statutory_within_minutes
                + day.statutory_excess_minutes
                + day.legal_holiday_minutes,
            day.actual_minutes,
            "partition must sum to actual for {}",
            day.work_date
        );
    }
}
