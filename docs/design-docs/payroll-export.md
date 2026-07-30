# Payroll Export Contract

**Status:** Phase 3 implemented
**Updated:** 2026-07-30

`GET /api/admin/payroll-export?year=YYYY&month=M` is a System Admin-only bulk export.
It returns HTTP 200 when individual users fail; `exported` and `failed` identify each result.
Only a `monthly_closing_workflows.status = closed` row is exportable. A legacy lock is not
sufficient, and `open`, `self_confirmed`, `approved`, and `reopened` produce
`monthly_not_closed`.

Closing creates an immutable payroll snapshot in the same database transaction as the workflow
transition and resolved-workday lock. The first close has revision 1. Reopen blocks export.
Re-close appends the next revision; it never updates an earlier snapshot. Consequently corrections
made after a close cannot change that revision's bytes.

## CSV

The response's `csv` string is UTF-8 with a BOM (`U+FEFF`) and CRLF line endings. RFC 4180
double-quote escaping is used. Rows are ordered by `employee_id`. Numeric values are integer
minutes; there is no decimal-hour conversion or rounding.

Fixed column order:

```text
employee_id,year,month,revision,worked_minutes,scheduled_minutes,statutory_within_minutes,statutory_excess_minutes,legal_holiday_minutes,night_minutes,absent_days,paid_leave_days,paid_leave_half_days,paid_leave_minutes,holiday_work_minutes,substitute_holiday_days,compensatory_leave_minutes
```

`worked_minutes` and breaks use approved effective correction values when present.
The five classification columns freeze the exact T-03 pure-classification totals. Approved annual leave is
classified by T-11 acquisition unit: `day`, `half_am`/`half_pm`, or `hour` (requested minutes).
Approved T-13 substitution contributes holiday work minutes and a substitute-holiday day;
compensatory benefit contributes its granted minutes.
