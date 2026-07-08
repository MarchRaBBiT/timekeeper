//! 日次労働時間区分の純ロジック（attendance-calculation-policy.md Decision 1–8, 10 の実装）。
//!
//! 入力は resolved workday snapshot（予定側）と effective 打刻から導出した
//! 実労働区間の列（Decision 2）。出力は基本区分 4 種（所定内 / 法定内残業 /
//! 法定外残業 / 法定休日）の partition と深夜 overlay の分値（整数分・丸めなし）。

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::work_schedules::{ResolvedDayKind, ScheduleType};

/// 就業規則マスタの 1 行分のパラメータ（Decision 11）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkRuleParameters {
    pub statutory_daily_minutes: i64,
    pub statutory_weekly_minutes: i64,
    pub night_start: NaiveTime,
    pub night_end: NaiveTime,
    /// ISO weekday (1 = Monday .. 7 = Sunday)
    pub week_start_weekday: u8,
    /// ISO weekday (1 = Monday .. 7 = Sunday)
    pub legal_holiday_weekday: u8,
}

/// 実労働区間（閉開区間 `[start, end)`、Decision 2 の唯一の中間表現）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActualWorkInterval {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl ActualWorkInterval {
    pub fn minutes(&self) -> i64 {
        (self.end - self.start).num_minutes().max(0)
    }
}

/// 区分計算への日次入力。`intervals` は当該 `work_date` に帰属する実労働区間列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationDayInput {
    pub work_date: NaiveDate,
    pub day_kind: ResolvedDayKind,
    pub schedule_type: ScheduleType,
    pub expected_work_minutes: i64,
    pub intervals: Vec<ActualWorkInterval>,
    /// その日時点で有効な就業規則パラメータ。
    pub rules: WorkRuleParameters,
}

/// 日次区分の出力（すべて丸めなしの整数分、Decision 10）。
///
/// 不変条件: `scheduled + statutory_within + statutory_excess + legal_holiday == actual`
/// （fixed 日）。flex 日は partition を持たず `actual` / `night` のみ（Decision 8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyClassification {
    pub work_date: NaiveDate,
    pub schedule_type: ScheduleType,
    pub actual_minutes: i64,
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
    pub legal_holiday_minutes: i64,
    pub night_minutes: i64,
}

/// effective 打刻（clock/break）から実労働区間列を導出する（Decision 2）。
///
/// - clock_in / clock_out のどちらかが欠ける進行中 attendance は区間なし（全区分 0 分）
/// - 休憩は完了済み（end あり）のもののみ、実際に発生した時刻で除去する
/// - 勤務範囲外にはみ出した休憩は勤務範囲へクリップする
pub fn build_actual_intervals(
    clock_in: Option<NaiveDateTime>,
    clock_out: Option<NaiveDateTime>,
    breaks: &[(NaiveDateTime, Option<NaiveDateTime>)],
) -> Vec<ActualWorkInterval> {
    let (Some(start), Some(end)) = (clock_in, clock_out) else {
        return Vec::new();
    };
    if end <= start {
        return Vec::new();
    }

    let mut clipped: Vec<(NaiveDateTime, NaiveDateTime)> = breaks
        .iter()
        .filter_map(|(break_start, break_end)| break_end.map(|break_end| (*break_start, break_end)))
        .filter_map(|(break_start, break_end)| {
            let clipped_start = break_start.max(start);
            let clipped_end = break_end.min(end);
            (clipped_start < clipped_end).then_some((clipped_start, clipped_end))
        })
        .collect();
    clipped.sort();

    let mut merged: Vec<(NaiveDateTime, NaiveDateTime)> = Vec::new();
    for (break_start, break_end) in clipped {
        match merged.last_mut() {
            Some(last) if break_start <= last.1 => {
                if break_end > last.1 {
                    last.1 = break_end;
                }
            }
            _ => merged.push((break_start, break_end)),
        }
    }

    let mut intervals = Vec::new();
    let mut cursor = start;
    for (break_start, break_end) in merged {
        if cursor < break_start {
            intervals.push(ActualWorkInterval {
                start: cursor,
                end: break_start,
            });
        }
        cursor = cursor.max(break_end);
    }
    if cursor < end {
        intervals.push(ActualWorkInterval { start: cursor, end });
    }
    intervals
}

/// `week_start_weekday` を起算曜日として、`date` の属する週の開始日を返す（Decision 5）。
pub fn week_start_date(date: NaiveDate, week_start_weekday: u8) -> NaiveDate {
    let weekday = i64::from(date.weekday().number_from_monday());
    let start = i64::from(week_start_weekday);
    date - Duration::days((weekday - start).rem_euclid(7))
}

/// 窓内の日次入力を時系列で区分計算する。
///
/// 呼び出し側は週次判定（Decision 5）のため、対象月に重なる週の全日
/// （前月末・翌月頭を含む）を渡すこと。欠測日（入力に無い日）は 0 分として扱われる。
pub fn classify_days(days: &[ClassificationDayInput]) -> Vec<DailyClassification> {
    let mut ordered: Vec<&ClassificationDayInput> = days.iter().collect();
    ordered.sort_by_key(|day| day.work_date);

    let mut results = Vec::with_capacity(ordered.len());
    let mut weekly_cumulative: i64 = 0;
    let mut current_week_start: Option<NaiveDate> = None;

    for day in ordered {
        let week_start = week_start_date(day.work_date, day.rules.week_start_weekday);
        if current_week_start != Some(week_start) {
            current_week_start = Some(week_start);
            weekly_cumulative = 0;
        }

        let actual: i64 = day.intervals.iter().map(ActualWorkInterval::minutes).sum();
        let night = night_minutes(&day.intervals, day.rules.night_start, day.rules.night_end);
        let mut classification = DailyClassification {
            work_date: day.work_date,
            schedule_type: day.schedule_type,
            actual_minutes: actual,
            scheduled_minutes: 0,
            statutory_within_minutes: 0,
            statutory_excess_minutes: 0,
            legal_holiday_minutes: 0,
            night_minutes: night,
        };

        if is_legal_holiday(day) {
            // Decision 6: 法定休日労働は全量。日次・週次の法定判定から除外する。
            classification.legal_holiday_minutes = actual;
        } else if day.schedule_type == ScheduleType::Flex {
            // Decision 8: flex 日は日次・週次判定の対象外。清算期間単位で別途区分する。
        } else {
            // Decision 4: 日次法定判定（時系列累積で超過分を特定）。
            let (within_daily, excess_daily) =
                split_after_cumulative(&day.intervals, day.rules.statutory_daily_minutes);
            let daily_excess: i64 = excess_daily.iter().map(ActualWorkInterval::minutes).sum();
            let countable: i64 = within_daily.iter().map(ActualWorkInterval::minutes).sum();

            // Decision 5: 週次累積は「日次法定外にならなかった分」のみを対象とする。
            let available = (day.rules.statutory_weekly_minutes - weekly_cumulative).max(0);
            let weekly_excess = (countable - available).max(0);
            weekly_cumulative += countable - weekly_excess;

            let excess = daily_excess + weekly_excess;
            let remaining = actual - excess;
            // Decision 7: 所定内 / 法定内残業は日次の分量比較。
            let scheduled = remaining.min(day.expected_work_minutes.max(0));
            classification.statutory_excess_minutes = excess;
            classification.scheduled_minutes = scheduled;
            classification.statutory_within_minutes = remaining - scheduled;
        }

        results.push(classification);
    }
    results
}

/// flex 清算期間の法定総枠（週法定 × 清算期間暦日数 ÷ 7、分値切り捨て。Decision 8）。
pub fn statutory_period_frame_minutes(
    statutory_weekly_minutes: i64,
    period_calendar_days: i64,
) -> i64 {
    (statutory_weekly_minutes.max(0) * period_calendar_days.max(0)) / 7
}

/// flex 清算期間単位の区分（Decision 8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexPeriodClassification {
    pub scheduled_minutes: i64,
    pub statutory_within_minutes: i64,
    pub statutory_excess_minutes: i64,
}

/// flex 清算期間の実績計（法定休日労働分を除外済み）を区分する。
///
/// 法定外を先に確定してから所定内を clamp することで、契約所定 > 法定総枠の
/// 誤設定でも負値を返さない（`scheduled + within + excess == actual` を常に満たす）。
pub fn classify_flex_period(
    actual_minutes: i64,
    contracted_minutes: i64,
    statutory_frame_minutes: i64,
) -> FlexPeriodClassification {
    let actual = actual_minutes.max(0);
    let excess = (actual - statutory_frame_minutes.max(0)).max(0);
    let scheduled = (actual - excess).min(contracted_minutes.max(0));
    FlexPeriodClassification {
        scheduled_minutes: scheduled,
        statutory_within_minutes: actual - excess - scheduled,
        statutory_excess_minutes: excess,
    }
}

fn is_legal_holiday(day: &ClassificationDayInput) -> bool {
    u8::try_from(day.work_date.weekday().number_from_monday())
        .map(|weekday| weekday == day.rules.legal_holiday_weekday)
        .unwrap_or(false)
        && matches!(
            day.day_kind,
            ResolvedDayKind::ScheduledNonWorkingDay | ResolvedDayKind::PublicHoliday
        )
}

/// 区間列を時系列に累積し、累積 `limit` 分を超えた時点で区間を分割する（Decision 4）。
fn split_after_cumulative(
    intervals: &[ActualWorkInterval],
    limit: i64,
) -> (Vec<ActualWorkInterval>, Vec<ActualWorkInterval>) {
    let limit = limit.max(0);
    let mut within = Vec::new();
    let mut excess = Vec::new();
    let mut cumulative: i64 = 0;
    let mut ordered: Vec<ActualWorkInterval> = intervals.to_vec();
    ordered.sort_by_key(|interval| interval.start);

    for interval in ordered {
        let length = interval.minutes();
        if cumulative >= limit {
            excess.push(interval);
        } else if cumulative + length <= limit {
            within.push(interval);
        } else {
            let split_at = interval.start + Duration::minutes(limit - cumulative);
            within.push(ActualWorkInterval {
                start: interval.start,
                end: split_at,
            });
            excess.push(ActualWorkInterval {
                start: split_at,
                end: interval.end,
            });
        }
        cumulative += length;
    }
    (within, excess)
}

/// 実労働区間列と暦日ごとの深夜帯の交差分（Decision 3）。
///
/// `night_start > night_end`（例: 22:00–翌 5:00）は日跨ぎ帯として扱う。
/// `night_start == night_end` は空帯として扱う。
fn night_minutes(
    intervals: &[ActualWorkInterval],
    night_start: NaiveTime,
    night_end: NaiveTime,
) -> i64 {
    intervals
        .iter()
        .map(|interval| night_overlap(interval, night_start, night_end))
        .sum()
}

fn night_overlap(
    interval: &ActualWorkInterval,
    night_start: NaiveTime,
    night_end: NaiveTime,
) -> i64 {
    if night_start == night_end {
        return 0;
    }
    let mut total = 0;
    let mut day = interval.start.date();
    let last_day = interval.end.date();
    while day <= last_day {
        for (segment_start, segment_end) in night_segments(day, night_start, night_end) {
            let overlap_start = interval.start.max(segment_start);
            let overlap_end = interval.end.min(segment_end);
            if overlap_start < overlap_end {
                total += (overlap_end - overlap_start).num_minutes();
            }
        }
        let Some(next) = day.succ_opt() else {
            break;
        };
        day = next;
    }
    total
}

fn night_segments(
    day: NaiveDate,
    night_start: NaiveTime,
    night_end: NaiveTime,
) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    let midnight = NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is a valid time");
    if night_start > night_end {
        let mut segments = Vec::with_capacity(2);
        if let Some(next_day) = day.succ_opt() {
            segments.push((day.and_time(night_start), next_day.and_time(midnight)));
        }
        segments.push((day.and_time(midnight), day.and_time(night_end)));
        segments
    } else {
        vec![(day.and_time(night_start), day.and_time(night_end))]
    }
}
