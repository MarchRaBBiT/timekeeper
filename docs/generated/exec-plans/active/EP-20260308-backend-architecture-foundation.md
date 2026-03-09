# EP-20260308-backend-architecture-foundation

## Goal
- バックエンド全面再構築に向けた最初の土台として、起動/bootstrap とルーティング構成を `platform` 層へ分離し、`main.rs` を薄くする

## Scope
- In: `backend/src/main.rs`, 新規 `backend/src/platform/*`, `backend/src/lib.rs`, 関連ユニットテスト
- Out: handler の業務ロジック再設計、DB schema 再設計、API 互換性変更

## Done Criteria (Observable)
- [x] `main.rs` がアプリ組み立ての委譲のみを行う
- [x] 既存の public/user/admin/system-admin ルーティング組み立てが `platform` モジュールに移動している
- [x] CORS/cleanup/bootstrap の既存挙動をユニットテストで維持確認できる

## Constraints / Non-goals
- 既存 handler / middleware / state の外部挙動は変えない
- 今回は新規アーキテクチャの受け皿作成までに留め、業務モジュール分解は別タスクとする

## Task Breakdown
1. [x] `platform` モジュール追加と `main.rs` の薄型化
2. [x] ルーティング・共通 layer・cleanup 起動処理の移設
3. [x] 既存 `main.rs` テストの移設と回帰確認
4. [x] fmt + 対象テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib platform::`
- [x] `cargo test -p timekeeper-backend --bin timekeeper-backend`

## JJ Snapshot Log
- [x] `jj status`
- [x] `cargo test -p timekeeper-backend --lib platform::` pass
- [x] `cargo test -p timekeeper-backend --bin timekeeper-backend` pass
- [ ] `jj commit -m "refactor(backend): move app bootstrap into platform module"`

## Progress Notes
- 2026-03-08: 専用ブランチ `topic/backend-rebuild-architecture` を作成。
- 2026-03-08: `platform::{app,runtime}` を追加し、`main.rs` の起動・ルーティング・cleanup 初期化を移設。platform ユニットテストと bin テストビルド成功。

