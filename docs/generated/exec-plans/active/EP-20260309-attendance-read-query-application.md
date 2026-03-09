# EP-20260309-attendance-read-query-application

## Goal
- attendance read 系の期間解釈とステータス整形を application 層へ移し、handler の分岐を減らす

## Scope
- In: `backend/src/attendance/application/*`, `backend/src/attendance/mod.rs`, `backend/src/handlers/attendance.rs`, `.agent/PLANS.md`
- Out: clock in/out・break 更新系の再設計、API path/method/payload 変更、DB schema 変更

## Done Criteria (Observable)
- [x] `get_my_attendance` の期間解釈が application 層に移っている
- [x] `get_attendance_status` のステータス組み立てが application 層に移っている
- [x] `get_my_summary` の月次期間解釈が application 層に移っている
- [x] 既存 attendance read API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- 既存 docs 用の query/response 型名は維持する
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `attendance::application::queries` を追加し、期間解釈とステータス整形を移設
2. [x] `handlers::attendance` から read 系の重い条件分岐を除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib attendance::application::queries::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::attendance::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(attendance): extract read query application helpers"`

## Progress Notes
- 2026-03-09: attendance read 系の期間解釈とステータス整形を `attendance::application::queries` へ移し、handler では repository 呼び出しと HTTP 変換に集中させる。

