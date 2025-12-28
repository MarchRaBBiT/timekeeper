# 監査ログ: DB スキーマとモデル追加

## 目的
- 監査ログ保存用のテーブルと Rust モデル/リポジトリを追加する。

## 依存
- `documents/issues/001-audit-log-event-catalog.md`

## 作業タスク
- [ ] `backend/migrations` に `audit_logs` テーブル作成 SQL を追加する
- [ ] 主要カラム: id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, error_code, metadata(JSON), ip, user_agent, request_id
- [ ] 検索向けインデックス（occurred_at, actor_id, event_type, target_id, request_id）を付与する
- [ ] `backend/src/models` に AuditLog モデルを追加する
- [ ] リポジトリ（INSERT/SELECT）を追加し、SQLx で型を固定する

## 受入条件
- migration を適用すると `audit_logs` が作成される
- INSERT/SELECT が SQLx で型安全に動作する

## テスト
- SQLx を使ったリポジトリの insert/select テストを追加する

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
- event_type は初期は TEXT とし、イベントが安定した段階で DB enum 化を検討する
