# EP-20260309-final-handler-cleanup

## Goal
- handler 配下に残っていた read/access helper と admin utility を application/use-case 側へ寄せ、handler 層を adapter と互換 re-export 中心にする

## Scope
- In: `backend/src/attendance/application/*`, `backend/src/admin/application/*`, `backend/src/handlers/attendance.rs`, `backend/src/handlers/subject_requests.rs`, `backend/src/handlers/admin/*`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `attendance` handler に残っていた status/break lookup が application query を経由する
- [x] `attendance_utils` と `admin/common` の実装本体が application 側へ移る
- [x] `subject_requests` handler の error mapping / details test helper が整理される
- [x] 既存 API path / method / JSON shape が変わらない

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::helpers::tests::ensure_authorized_access_rejects_other_user -- --exact`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::queries::tests::resolve_attendance_range_uses_explicit_range -- --exact`
- [x] `cargo test -p timekeeper-backend --lib requests::application::user_subject_requests::tests::validate_details_accepts_valid_details -- --exact`
- [x] `cargo test -p timekeeper-backend --lib admin::application::common::tests::push_clause_switches_between_where_and_and -- --exact`
- [x] `cargo test -p timekeeper-backend --lib platform::app::tests::test_app_router_builds -- --exact`

## JJ Snapshot Log
- [x] `jj status`
- [x] final handler cleanup tests pass
- [x] `jj commit -m "refactor(handler): finish handler-layer cleanup"`

## Progress Notes
- 2026-03-09: `attendance` の status/break lookup を `attendance/application/queries.rs` へ移し、`subject_requests` の local error mapping を共通 helper 利用に変更。
- 2026-03-09: `admin/application/common.rs` と `attendance/application/helpers.rs` を追加し、`handlers/admin/common.rs` と `handlers/attendance_utils.rs` は互換 re-export に差し替え。API 互換は維持し、`BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: fmt / clippy / 代表ユニットテスト 5 本が成功。
- 2026-03-09: `jj commit -m "refactor(handler): finish handler-layer cleanup"` を実行し、今回の handler cleanup 差分を保存。

