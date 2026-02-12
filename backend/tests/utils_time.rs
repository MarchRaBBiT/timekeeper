use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use timekeeper_backend::utils::time::{now_in_timezone, now_utc, today_local};

#[test]
fn time_now_in_jst_returns_jst_datetime() {
    let tz: Tz = "Asia/Tokyo".parse().unwrap();
    let result = now_in_timezone(&tz);
    assert_eq!(result.timezone(), tz);
}

#[test]
fn time_now_in_est_returns_est_datetime() {
    let tz: Tz = "America/New_York".parse().unwrap();
    let result = now_in_timezone(&tz);
    assert_eq!(result.timezone(), tz);
}

#[test]
fn time_now_utc_returns_utc() {
    let tz: Tz = chrono_tz::UTC;
    let result = now_utc(&tz);
    assert_eq!(result.timezone(), Utc);
}

#[test]
fn time_today_local_returns_date() {
    let tz: Tz = chrono_tz::UTC;
    let result = today_local(&tz);
    assert_eq!(result, Utc::now().date_naive());
}

#[test]
fn time_now_in_timezone_returns_different_times() {
    let jst: Tz = "Asia/Tokyo".parse().unwrap();
    let utc: Tz = chrono_tz::UTC;

    let jst_time = now_in_timezone(&jst);
    let utc_time = now_in_timezone(&utc);

    assert_eq!(jst_time.timezone(), jst);
    assert_eq!(utc_time.timezone(), utc);
}

#[test]
fn time_now_utc_is_consistent() {
    let tz1: Tz = chrono_tz::UTC;
    let tz2: Tz = "Asia/Tokyo".parse().unwrap();

    let utc_from_utc = now_utc(&tz1);
    let utc_from_jst = now_utc(&tz2);

    let diff = (utc_from_utc - utc_from_jst).num_seconds().abs();
    assert!(diff < 2);
}

#[test]
fn time_today_local_different_timezones() {
    let jst: Tz = "Asia/Tokyo".parse().unwrap();
    let pst: Tz = "America/Los_Angeles".parse().unwrap();

    let jst_date = today_local(&jst);
    let pst_date = today_local(&pst);

    let diff_days = (jst_date - pst_date).num_days().abs();
    assert!(diff_days <= 1);
}

#[test]
fn time_now_in_timezone_not_utc() {
    let jst: Tz = "Asia/Tokyo".parse().unwrap();
    let result = now_in_timezone(&jst);
    assert_eq!(result.timezone(), jst);
    assert!(jst.to_string().contains("Tokyo"));
}

#[test]
fn time_jst_and_utc_produce_different_datetimes() {
    let jst: Tz = "Asia/Tokyo".parse().unwrap();
    let utc: Tz = chrono_tz::UTC;

    let jst_time = now_in_timezone(&jst);
    let utc_time = now_in_timezone(&utc);

    assert_ne!(jst_time.to_rfc3339(), utc_time.to_rfc3339());
}

#[test]
fn time_today_local_is_naive() {
    let tz: Tz = chrono_tz::UTC;
    let _: NaiveDate = today_local(&tz);
}

#[test]
fn time_now_utc_timestamp_increases() {
    let tz: Tz = chrono_tz::UTC;
    let time1 = now_utc(&tz);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let time2 = now_utc(&tz);

    assert!(time2 >= time1);
}
