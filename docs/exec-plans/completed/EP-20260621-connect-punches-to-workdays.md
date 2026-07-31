# EP-20260621-connect-punches-to-workdays

## Goal

- 出退勤打刻を解決済み勤務日へ接続し、最初の出勤でprojectionを固定し、休日・予定外勤務も実績として保存する

## Scope

- In: punch用work date解決、ClockIn/ClockOut use case更新、attendance FK、projection lock、予定外勤務flag、PostgreSQL transaction、既存API adapter、unit/integration tests、API catalog更新
- Out: anomaly管理API、打刻漏れ検知、日別例外管理API、月次締め、frontend表示変更

## Done Criteria (Observable)

- [x] 出勤前に`ResolvedWorkday`を解決し、attendanceへ`resolved_workday_id`を保存する
- [x] 最初の出勤とprojection lockを同一DB transactionで確定する
- [x] 祝日・予定非勤務日でも出勤を拒否せず、`is_unscheduled_work = true`で保存する
- [x] 通常勤務日は`is_unscheduled_work = false`で保存する
- [x] 退勤は祝日判定や再解決をせず、出勤済みattendanceを更新する
- [x] 日界時刻より前の打刻は前日の勤務日へ帰属する
- [x] 明示date付き打刻は指定勤務日を使用する
- [x] 勤務体系未設定の出勤は`422 WORK_SCHEDULE_NOT_CONFIGURED`を返す
- [x] attendanceとresolved projectionのuser/date不一致をDBが拒否する
- [x] 新規・変更モジュールのline coverageが80%以上である

## Constraints / Non-goals

- resolver規則は`crates/app`に維持し、handlerへSQLや優先順位を置かない
- attendance FKは既存データ互換のためnullableとし、新規出勤だけ必須にする
- raw punch時刻は丸めず、現行のlocal `NaiveDateTime`保存を維持する
- SQL入力はすべてbind parameterを使用する
- 認証、CSRF、rate limitは既存attendance routeの境界を維持する

## Task Breakdown

1. [x] app unit testsとAPI integration testsをREDにする
2. [x] punch work date解決とClockIn/ClockOut use caseを実装する
3. [x] migration 045とattendance transaction adapterを実装する
4. [x] handlerをresolverへ接続し、holiday拒否adapterを削除する
5. [x] API catalogと勤務体系設計を更新する
6. [x] focused tests、coverage、fmt、clippy、harness、security reviewを実行する
7. [x] git snapshotを作成する

## Validation Plan

- [x] `cargo test -p timekeeper-app --test clock_in --test clock_out --test resolve_workday` — 21 passed
- [x] `cargo test -p timekeeper-backend --test attendance_api -- --nocapture` — 20 passed
- [x] `cargo test -p timekeeper-backend --test attendance_work_schedule_integration -- --nocapture` — 7 passed
- [x] `cargo test -p timekeeper-infra-postgres --test attendance_workflow_repository` — 2 passed
- [x] `bash scripts/harness.sh backend-unit` — 369 passed
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] changed-module line coverage >= 80% — app変更ロジック83.97%、DB/API変更ロジック93.18%、全変更ロジック87.67%

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit -m "feat: connect attendance punches to workdays"`

## Progress Notes

- 2026-06-21: resolver接続、attendance FK、projection lock、予定外勤務flag、夜勤日界、退勤のsnapshot再利用を今回の実装範囲に固定した。
- 2026-06-21: app testsをRED→GREENとし、resolved ID伝播、祝日打刻許可、日界前の前日帰属、退勤時の再解決廃止を固定した。
- 2026-06-21: migration 045で複合FK、locked検証trigger、`is_unscheduled_work`導出を追加した。attendance作成とlockは同一transactionで行い、insert失敗時のrollbackを実DBで確認した。
- 2026-06-21: attendance API 20 tests、打刻・勤務日integration 6 tests、app 21 focused tests、backend unit 369 tests、workspace clippy、fmt、docs-checkがgreen。
- 2026-06-21: security reviewで認証・CSRF・rate limit維持、bind parameter、typed input、複合FK、locked検証、generic errorを確認した。
- 2026-06-21: app変更ロジックline coverageは83.97%。DB/API統合coverage再計測はCodex approval usage limitにより実行開始前に拒否された。
- 2026-06-21: `git add`も同じapproval usage limitで拒否されたため、commit snapshotは未作成。検証済み差分はworktreeに保持している。
- 2026-06-21: 継続実行でalready-locked workday再利用testを追加し、打刻・勤務日integrationは7 tests green。DB/API変更ロジック93.18%、全変更ロジック87.67%を確認した。
- 2026-06-21: 最終検証後に`feat: connect attendance punches to workdays`としてgit snapshotを作成した。
