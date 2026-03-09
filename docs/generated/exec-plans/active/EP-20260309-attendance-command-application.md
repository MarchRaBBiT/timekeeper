# EP-20260309-attendance-command-application

## Goal
- attendance の更新系 use-case を application 層へ移し、handler から clock/break の状態遷移責務を除去する

## Scope
- In: `backend/src/attendance/application/commands.rs`, `backend/src/attendance/application/mod.rs`, `backend/src/handlers/attendance.rs`, `.agent/PLANS.md`
- Out: API path/method/payload 変更、attendance repository 再設計、DB schema 変更

## Done Criteria (Observable)
- [x] `clock_in` / `clock_out` の状態遷移と holiday 判定が application 層に移っている
- [x] `break_start` / `break_end` の状態遷移が application 層に移っている
- [x] 既存 attendance update API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- 時刻取得自体は handler に残し、application には確定値を渡す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `attendance::application::commands` を追加し、clock/break use-case と holiday 判定を移設
2. [x] `handlers::attendance` から更新系の状態遷移ロジックを除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::commands::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(attendance): move command application helpers"`

## Progress Notes
- 2026-03-09: attendance の clock/break 更新系と holiday 判定を `attendance::application::commands` へ移し、handler 側は時刻決定と HTTP 変換に集中させる。

