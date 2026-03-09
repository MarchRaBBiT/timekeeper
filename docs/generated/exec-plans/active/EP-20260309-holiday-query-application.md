# EP-20260309-holiday-query-application

## Goal
- holiday の user/admin read 系と Google ICS 解析を application 層へ移し、handler を HTTP adapter に寄せる

## Scope
- In: `backend/src/holiday/application/*`, `backend/src/holiday/mod.rs`, `backend/src/handlers/holidays.rs`, `.agent/PLANS.md`
- Out: admin holiday create/delete の再設計、API path/method/payload 変更、DB schema 変更

## Done Criteria (Observable)
- [x] public holiday 一覧取得が application 層へ移っている
- [x] holiday check/month query が application 層へ移っている
- [x] Google ICS fetch/parse が application 層へ移っている
- [x] 既存 holiday API の JSON 形状を維持したまま関連テストが成功する

## Constraints / Non-goals
- admin 権限チェックは handler に残す
- API 互換が崩れない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `holiday::application::queries` を追加し、一覧・判定・月間取得・ICS fetch/parse を移設
2. [x] `handlers::holidays` から read/query 系ロジックを除去
3. [x] fmt + 関連ユニットテスト + app router build で回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib holiday::application::queries::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::holidays::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [ ] `jj status`
- [x] validation commands pass
- [ ] `jj commit -m "refactor(holiday): extract holiday query application helpers"`

## Progress Notes
- 2026-03-09: holiday 一覧・判定・月間取得・Google ICS 解析を `holiday::application::queries` へ移し、handler 側は auth と HTTP 変換に限定。

