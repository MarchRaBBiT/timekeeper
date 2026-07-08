use std::fmt;

use chrono::NaiveDate;

pub mod attendance_classification;
pub mod work_schedules;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkDate(NaiveDate);

impl WorkDate {
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, WorkDateError> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Self)
            .ok_or(WorkDateError { year, month, day })
    }

    pub fn from_naive_date(date: NaiveDate) -> Self {
        Self(date)
    }

    pub fn as_naive_date(self) -> NaiveDate {
        self.0
    }
}

impl fmt::Display for WorkDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid work date: {year:04}-{month:02}-{day:02}")]
pub struct WorkDateError {
    year: i32,
    month: u32,
    day: u32,
}
