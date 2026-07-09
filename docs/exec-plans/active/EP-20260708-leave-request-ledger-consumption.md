# EP-20260708-leave-request-ledger-consumption

**親タスク:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-05（G2 実装 後半）
**前提:** [EP-20260705-leave-entitlement-ledger](./EP-20260705-leave-entitlement-ledger.md) / [docs/design-docs/leave-entitlement.md](../../design-docs/leave-entitlement.md)

## Goal

- 年次有給申請を leave ledger と接続し、残高不足の入口 reject、承認時 consume、取消時 release を成立させる
- pending は第一増分では引当せず、承認時に残高を再検証して FIFO consume する

## Scope

- In:
  - `POST /api/requests/leave` の `annual` 申請で残高不足を `LEAVE_BALANCE_INSUFFICIENT` として reject
  - `annual` 承認時に同一 DB transaction で `leave_requests` 更新と `leave_ledger_entries(kind='consume')` 追記
  - 承認済み `annual` 取消時に同一 DB transaction で `leave_requests` 取消と `leave_ledger_entries(kind='release')` 追記
  - `annual` 以外は残高非連動のまま既存挙動を維持
  - API catalog / OpenAPI / contract error code / integration test の同期
- Out:
  - pending 申請時点の reservation ledger
  - 半休・時間単位・休暇種別マスタ（T-11）
  - 承認済み休暇の勤怠表示反映（T-06）

## Done Criteria

- [x] 残高不足の `annual` 申請が `400 LEAVE_BALANCE_INSUFFICIENT` で reject される
- [x] 残高ありの `annual` 申請は pending では残高を減らさず、承認時に FIFO consume で残高が減る
- [x] 承認済み `annual` 取消で release が記録され、残高が復帰する
- [x] `sick` / `personal` / `other` は残高非連動のまま作成・承認できる
- [x] `cargo fmt --all --check`、focused integration、backend tests、clippy、docs-check が green
- [x] conventional commit 作成済み、本 EP に実測値を記録済み

## Task Breakdown

1. [x] 申請/承認/取消の現在経路と T-04 ledger repository を確認する
2. [x] T-05 integration test を先に追加する
3. [x] contract に残高不足 error code を固定する
4. [x] app/domain 境界に annual 申請日数の consume/release entry 作成ロジックを追加する
5. [x] infra-postgres に transaction 内 ledger read/append を追加する
6. [x] backend request repository / handler を同一 transaction で接続する
7. [x] 既存 tests の annual fixture を T-05 後の前提へ更新する
8. [x] docs/catalog/OpenAPI/タスク表を同期する
9. [x] validation ladder を実行する

## Validation Plan

- [x] `cargo test -p timekeeper-backend --test leave_request_ledger_api`
- [x] `cargo test -p timekeeper-app --test leave_ledger`
- [x] `cargo test -p timekeeper-contract --test leave_contract`
- [x] `cargo test -p timekeeper-backend --test leave_request_api`
- [x] `cargo test -p timekeeper-backend --test admin_requests_api`
- [x] `cargo test -p timekeeper-backend --test department_approval_api`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --tests`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh fmt-check`
- [x] `bash scripts/harness.sh docs-check`

## Progress Notes

- 2026-07-08: T-05 着手。T-02 design doc の pending 方針（pending は引当せず承認時 consume + 再検証）と T-04 の ledger 実装を確認。
- 2026-07-08: 実装完了。annual 申請入口の残高検証、承認時 FIFO consume、承認済み annual 取消時 release を integration test で固定。pending は ledger を書かず、承認時に同一 DB transaction で再検証・consume する。
- 2026-07-08: 検証実測: `cargo test -p timekeeper-backend --test leave_request_ledger_api`、`cargo test -p timekeeper-app --test leave_ledger`、`cargo test -p timekeeper-contract --test leave_contract`、`cargo test -p timekeeper-backend --test leave_request_api`、`cargo test -p timekeeper-backend --test admin_requests_api`、`cargo test -p timekeeper-backend --test department_approval_api`、`cargo test -p timekeeper-backend --lib`、`cargo test -p timekeeper-backend --tests`、`cargo clippy --workspace --all-targets -- -D warnings`、`bash scripts/harness.sh fmt-check`、`bash scripts/harness.sh docs-check` は green。
- 2026-07-09: Phase 1 多観点レビューで「消化分数が暦日ベース（土日も残高消費）」を HIGH として検出・修正（144ec32）。消化対象日を resolved workday の稼働日のみに変更し（leave-entitlement.md の Consumption Target Days に決定を追記）、稼働日 0 / 勤務予定未解決 / アクティブロット無しは専用エラーコードで fail-closed。申請作成時と承認時は同一ヘルパー（count_working_days_for_annual_leave）を共用し、承認時はロック前後の期間不一致を 409 で拒否。PUT /api/requests/{id} にも同じ残高再検証を追加（c7116c5）。検証: leave_request_ledger_api 9 件ほか leave 系 integration green
