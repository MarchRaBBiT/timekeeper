# EP-20260308-attendance-http-boundary

## Goal
- 勤怠・勤怠修正申請の HTTP 入口を `attendance` モジュールへ移し、`platform` はモジュール合成責務へ寄せる

## Scope
- In: 新規 `backend/src/attendance/*`, `backend/src/platform/app.rs`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: attendance handler 本体の再実装、holiday ルート分離、API contract 変更

## Done Criteria (Observable)
- [x] `/api/attendance*` と `/api/admin/attendance*` の route 定義が `attendance/interface/http.rs` に集約されている
- [x] admin と system-admin の勤怠系権限制御が従来どおり分離されている
- [x] attendance / platform の対象テストが成功する

## Constraints / Non-goals
- 既存 API path / method / payload は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] `attendance` モジュール追加
2. [x] attendance / attendance-correction admin route を `attendance/interface/http.rs` へ移設
3. [x] `platform::app` から attendance ルータを合成するよう変更
4. [x] attendance / platform の対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib attendance::interface::http::tests::`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(attendance): extract attendance http routes into module"`

## Progress Notes
- 2026-03-08: `attendance/interface/http.rs` を追加し、user/admin/system-admin の勤怠系 route 定義を `platform` から切り離した。既存 API 変更はなし。

