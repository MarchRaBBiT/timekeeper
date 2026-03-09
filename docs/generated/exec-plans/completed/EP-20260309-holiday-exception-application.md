# EP-20260309-holiday-exception-application

## Goal
- `holiday_exceptions` handler から権限確認・ID 解析・service 呼び出し・エラーマッピングを application 層へ移し、HTTP adapter を薄くする

## Scope
- In: `backend/src/holiday/application/*`, `backend/src/holiday/mod.rs`, `backend/src/handlers/holiday_exceptions.rs`, `.agent/PLANS.md`
- Out: API path/method/payload の変更、DB schema 変更

## Done Criteria (Observable)
- [x] `holiday_exceptions` handler が application helper を呼ぶだけの構成になる
- [x] holiday exception のエラーマッピングと admin/system-admin gate が application 層に移る
- [x] 既存 API path / method / JSON shape が変わらない

## Constraints / Non-goals
- 既存 service / repository の振る舞いは変えない
- `BREAKING_CHANGES.md` は API 互換が崩れた場合のみ更新する

## Task Breakdown
1. [x] `backend/src/holiday/application/exceptions.rs` を追加する
2. [x] handler から holiday exception の orchestration を application 層へ移す
3. [x] unit test を application 側へ移し、handler 側は adapter として成立する最小確認に寄せる

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib holiday::application::exceptions::tests::`
- [x] `cargo test -p timekeeper-backend --lib handlers::holiday_exceptions::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [x] holiday exception refactor tests pass
- [x] `jj commit -m "refactor(holiday): extract holiday exception application helpers"`

## Progress Notes
- 2026-03-09: holiday query 抽出の次段として、holiday exception handler の adapter 化に着手。
- 2026-03-09: `cargo fmt --all`、`cargo test -p timekeeper-backend --lib holiday::application::exceptions::tests::`、`cargo test -p timekeeper-backend --lib handlers::holiday_exceptions::tests::`、`cargo test -p timekeeper-backend --lib test_app_router_builds` が成功。API 互換は維持したため `BREAKING_CHANGES.md` は更新なし。
- 2026-03-09: `jj commit -m "refactor(holiday): extract holiday exception application helpers"` を実行し、`76716ea0` に保存。working copy はクリーン。

