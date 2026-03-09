# EP-20260309-attendance-report-application

## Goal
- attendance の summary/export と補正適用ロジックを application 層へ移し、read 系 handler の集計責務を減らす

## Scope
- In: `backend/src/attendance/application/reports.rs`, `backend/src/attendance/application/mod.rs`, `backend/src/handlers/attendance.rs`, `.agent/PLANS.md`
- Out: clock in/out・break 更新系の再設計、API path/method/payload 変更、DB schema 変更

## Done Criteria (Observable)
- [x] `get_my_attendance` の補正適用と response 組み立てが application 層で共通化されている
- [x] `get_my_summary` の集計が application 層へ移っている
- [x] `export_my_attendance` の CSV 組み立てが application 層へ移っている
- [x] 既存 attendance read/export API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- `build_attendance_response` と更新系 helper は handler 側に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `attendance::application::reports` を追加し、補正適用・summary・export を移設
2. [x] `handlers::attendance` から read/export 系の集計責務を除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::reports::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(attendance): move report application helpers"`

## Progress Notes
- 2026-03-09: attendance の補正適用・summary 集計・export CSV 生成を `attendance::application::reports` へ移し、handler 側は query 解釈済み引数の受け渡しに集中させる。

