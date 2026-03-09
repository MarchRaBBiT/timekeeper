# EP-20260212-issue150-gap-closure

## Goal
- issue #150 の残差分（平文フォールバック除去・平文列廃止・関連テスト整備）を完了する

## Scope
- In: `backend/src/repositories/*`, `backend/src/middleware/auth.rs`, `backend/src/handlers/*`, `backend/migrations/*`, `backend/tests/*`
- Out: インフラ資格情報、本番データの実移行作業

## Done Criteria (Observable)
- [x] DBクエリから平文PIIカラム参照/fallbackが除去されている
- [x] `users`/`archived_users` の平文PIIカラム削除migrationが追加されている
- [x] 影響範囲の統合テスト/ユニットテストが成功する

## Constraints / Non-goals
- 既存API仕様は維持（JSON項目名は互換維持）
- SQLx migrationは新規追加のみで対応

## Task Breakdown
1. [x] repository/handler/middleware の平文fallbackクエリを削除
2. [x] 平文PIIカラム削除migrationを追加
3. [x] テストデータ投入を `*_enc` 前提に更新
4. [x] 旧データ由来の `*_enc IS NULL` を吸収するmigrationを追加
5. [x] 影響テストを再実行して回帰なしを確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test admin_users_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test password_reset_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test transaction_repository -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test user_update_api -- --nocapture`
- [x] `cargo test -p timekeeper-backend --test user_repository -- --nocapture`

## JJ Snapshot Log
- [x] `jj status`
- [x] backend関連テスト pass
- [ ] `jj commit -m "fix(security): close issue150 pii plaintext gap"`

## Progress Notes
- 2026-02-12: `*_enc` への一本化と平文列削除migration（032）を追加。
- 2026-02-12: 既存seed由来の `NULL` を補正するmigration（033）を追加し、統合テスト群のデータ投入を暗号化前提に更新。

