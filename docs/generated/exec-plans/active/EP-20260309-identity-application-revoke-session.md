# EP-20260309-identity-application-revoke-session

## Goal
- `identity` の session revoke 業務ロジックを application/use-case 化し、handler から削除・整合性チェック責務を外す

## Scope
- In: `backend/src/identity/application/sessions.rs`, `backend/src/handlers/sessions.rs`, 関連ユニットテスト
- Out: 監査ログ組み立ての完全移設、admin session revoke の共通化、API contract 変更

## Done Criteria (Observable)
- [x] session revoke のバリデーション・所有者確認・トークン削除が `identity/application/sessions.rs` に移動している
- [x] `handlers/sessions.rs` は application 層の結果を使って監査イベントと HTTP レスポンスを返すだけになっている
- [x] 新旧対象テストが成功する

## Constraints / Non-goals
- 既存 API response 形状は変更しない
- 破壊的変更が出ない限り `BREAKING_CHANGES.md` は更新しない

## Task Breakdown
1. [x] revoke 用の application 関数と戻り値型を追加
2. [x] token/session 削除処理を handler から application 層へ移設
3. [x] `handlers/sessions.rs` を application 呼び出しへ差し替え
4. [x] application / handler / app build テスト実行

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib identity::application::sessions::tests::`
- [x] `cargo test -p timekeeper-backend --lib extract_ip_`
- [x] `cargo test -p timekeeper-backend --lib test_app_router_builds`

## JJ Snapshot Log
- [x] `jj status`
- [ ] `jj commit -m "refactor(identity): move session revoke into application service"`

## Progress Notes
- 2026-03-09: session revoke の整合性チェックと削除処理を `identity/application/sessions.rs` に移し、handler には監査と HTTP 応答だけを残した。既存 API 変更はなし。

