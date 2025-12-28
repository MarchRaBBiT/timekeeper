# 監査ログ: 記録パス（サービス/ミドルウェア）

## 目的
- 監査ログの記録処理をサービスに集約し、API 呼出を起点に成功/失敗を記録する。

## 依存
- `documents/issues/001-audit-log-event-catalog.md`
- `documents/issues/002-audit-log-schema-model.md`
- `documents/issues/005-audit-log-retention.md`

## 作業タスク
- [ ] `AuditLogService` を追加し、`record_event` API を提供する
- [ ] API 呼出を捕捉するミドルウェアを実装し、actor/ip/user_agent/request_id/result を付与する
- [ ] issue 001 の除外リストを反映し、不要な API 呼出は記録しない
- [ ] 保存期間 0 の場合は記録をスキップする

## 受入条件
- 成功/失敗の両方が監査ログに記録される
- 除外対象 API は記録されない
- 保存期間 0 の場合は一切記録されない

## テスト
- 監査ログサービスの unit テスト
- ミドルウェア経由での記録を確認する統合テスト

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
- 記録失敗はログのみで継続するため、監査ログ欠損の監視方法を別途検討すること
