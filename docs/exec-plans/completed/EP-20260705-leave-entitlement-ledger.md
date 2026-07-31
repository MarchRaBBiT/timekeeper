# EP-20260705-leave-entitlement-ledger

**親タスク:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-04（G2 実装 前半）
**親 EP:** [EP-20260704-attendance-domain-gap-backlog](../active/EP-20260704-attendance-domain-gap-backlog.md)
**設計 source of truth:** [docs/design-docs/leave-entitlement.md](../../design-docs/leave-entitlement.md)（T-02 成果物）

## Goal

- 有給の付与と残高参照をシステム内で成立させる（消化引当は T-05）
- append-only の `leave_ledger_entries` と付与ルールマスタを migration で追加し、残高・時効予定・年5日義務を read API として導出する

## Scope

- In:
  - migration: `052_create_leave_grant_rules.sql`（付与ルール + 年5日義務ルールのマスタ + 既定シード）、`053_create_leave_ledger_entries.sql`（append-only 台帳）、`054_add_users_hire_date.sql`（付与基準日の起点となる入社日）
  - `crates/domain`: ロット導出・FIFO 消化順・時効控除（遅延評価）・年5日義務判定・付与ルール選択の純ロジック + unit test
  - `crates/app`: `RunLeaveGrants` / `GetLeaveBalance` / `AdjustLeaveLedger` / `SetHireDate` use case + fake repository test
  - `crates/contract`: 残高・付与実行・adjust・hire-date の DTO + round-trip test
  - `crates/infra-postgres`: ledger / rules / grant-candidate repository（append は単一トランザクション、expire の冪等補記は partial unique index で保証）
  - `backend`: 薄い handler 配線、route 追加、OpenAPI（`docs.rs`）、`backend-api-catalog.md` 同期、integration test
  - API: `GET /api/leave-balances/me`（User）、`GET /api/admin/users/{user_id}/leave-balances`（Scoped Manager+）、`POST /api/admin/leave-grants/run`（System Admin、dry-run 付き）、`POST /api/admin/leave-ledger/adjust`（System Admin、dry-run 照合付き）、`PUT /api/admin/users/{user_id}/hire-date`（System Admin）
- Out（本 EP では行わない）:
  - 申請時の残高検証・consume / release の引当（T-05）
  - 半休・時間単位・種別マスタ（T-11）、代休（T-13）
  - cron 常駐の自動付与、比例付与・出勤率 8 割判定・一律基準日（design doc の拡張点のまま）
  - 承認済み休暇の勤怠反映（T-06）

## Design Notes / Doc との整合

- design doc は基準日を「入社日 + 6 ヶ月」と決定しているが、既存 `users` に入社日が存在しなかったため、`users.hire_date DATE NULL` を migration 054 で追加し、System Admin が `PUT /api/admin/users/{user_id}/hire-date` で投入する。`hire_date` 未設定ユーザーは付与実行で `hire_date_not_set` として skip される（design doc の「対象者選定は運用入力」の具体化）。design doc にも同内容を追記済み
- 年5日義務の法定値（10 日以上付与 / 5 日 / 1 年 window）も共通規約 8 に従い `leave_obligation_rules` マスタとして外部化した
- `expire` の冪等補記は付与実行 API が行う（`uq_leave_ledger_expire` partial unique index + 事前重複チェック）。残高導出は遅延評価（`expires_at <= as_of` のロットを 0 扱い）なので、補記遅延で残高が過大に見えることはない

## Done Criteria (Observable)

- [x] 付与実行（dry-run / 実行 / 冪等 skip）→ 残高参照（本人 / 管理者）の一連が integration test で green
- [x] `crates/domain` の時効境界・複数付与ロット FIFO・年5日義務の unit test が green
- [x] `backend-api-catalog.md` と `backend/src/docs.rs` が同一 change set で同期済み
- [x] `cargo fmt --all --check` / `cargo test -p timekeeper-backend --lib` / `cargo test -p timekeeper-backend --tests` / workspace clippy `-D warnings` / `bash scripts/harness.sh docs-check` green
- [x] conventional commit 作成対象の change set として、本 EP に実測値を記録済み

## Task Breakdown

1. [x] 既存パターン調査（settlement/read-model 型 use case、workday_overrides の scoped 認可、contract round-trip、testcontainers integration）
2. [x] migration 052/053/054 追加（T-04 予約帯。050/051 は T-03、055 は T-06 予約のため使用しない）
3. [x] `crates/domain::leave_ledger` 純ロジック + unit test（時効境界 / FIFO / 複数付与 / 義務判定 / 付与ルール選択 / 月末入社の基準日）
4. [x] `crates/contract::leave` DTO + round-trip test
5. [x] `crates/app::leave_ledger` use case + fake repo test
6. [x] `crates/infra-postgres::leave_ledger` repository
7. [x] backend handler / route / docs.rs / api-catalog 同期
8. [x] `backend/tests/leave_ledger_api.rs` integration test
9. [x] Validation Ladder（fmt → unit → integration → clippy → docs-check）

## Validation Plan

- [x] `bash scripts/harness.sh fmt-check`
- [x] `cargo test -p timekeeper-domain --test leave_ledger`
- [x] `cargo test -p timekeeper-contract --test leave_contract`
- [x] `cargo test -p timekeeper-app --test leave_ledger`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test leave_ledger_api`
- [x] `cargo test -p timekeeper-backend --test request_repositories`
- [x] `cargo test -p timekeeper-backend --tests`（testcontainers Postgres）
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh docs-check`
- live 前提ステージ（api-smoke / worker-once）は並列作業衝突防止のため実行しない

## Progress Notes

- 2026-07-05: T-04 着手。worktree を main（13fa58b）へ fast-forward。既存 read-model / scoped 認可 / contract round-trip パターンを調査し、migration 052–054・domain 純ロジック・use case・API 5 本の構成を確定。
- 2026-07-08: T-03 後の main に T-04 worktree 差分を取り込み、共有 routing / docs.rs / API catalog / request repository cleanup を解決。T-04 の付与実行・本人/管理者残高参照・adjust・hire-date API と domain/app/contract/infra-postgres を実装し、focused tests、backend integration 全体、workspace clippy、docs-check を green 確認。消化 / release 引当は T-05 スコープとして未実装のまま維持。
- 2026-07-09: Phase 1 多観点レビューの指摘を修正。RunLeaveGrants / AdjustLeaveLedger のロックなし read-then-write（TOCTOU）を `with_user_lock`（users 行 FOR UPDATE + 台帳 FOR UPDATE + 同一 tx 追記、承認経路とロック順序統一）へ是正し、付与バッチをユーザー単位 tx に分離（1 人の失敗が全体をロールバックしない）・`leave_grant_batch_lock`（migration 055、stale claim 600 秒自動失効）で二重起動を排他（457281d）。adjust 入力の i32 範囲 validate + checked_add、adjust 新規ロットの CHECK（migration 056）、エラー変換の `error::leave_ledger` への一本化、system-admin handler の defense-in-depth、付与/adjust/hire-date の監査ログ登録（10b60d0 / c7116c5）。`crates/app/src/leave_ledger.rs`（1,151 行）は use case 単位に分割（42c2074）。検証: unit + leave 系 integration + `cargo clippy --workspace --all-targets -- -D warnings` + docs-check green
