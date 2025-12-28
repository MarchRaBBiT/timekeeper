# 監査ログ: 保存期間設定とクリーンアップ

## 目的
- 保存期間を 0（記録しない）〜無限（削除しない）まで設定可能にし、既定は 365 日とする。

## 依存
- `documents/issues/002-audit-log-schema-model.md`

## 作業タスク
- [ ] 設定項目を追加する（例: `AUDIT_LOG_RETENTION_DAYS` と `AUDIT_LOG_RETENTION_FOREVER`）
- [ ] `AUDIT_LOG_RETENTION_DAYS=0` の場合は記録を無効化する
- [ ] `AUDIT_LOG_RETENTION_FOREVER=true` の場合は削除処理を実行しない
- [ ] 競合設定時の優先順位を定義する（例: FOREVER を優先）
- [ ] バックグラウンドで日次クリーンアップを実行する
- [ ] `env.example` と設定ドキュメントを更新する

## 受入条件
- 既定値は 365 日として読み込まれる
- 保存期間 0 の場合は記録が行われない
- FOREVER 有効時は削除処理が走らない

## テスト
- 設定読み込みと保持期間判定の unit テスト
- クリーンアップの削除対象判定テスト

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
