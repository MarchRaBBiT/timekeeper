# 監査ログ: 保持期間バッチ処理

## 目的
- 監査ログの保持期間に基づき、古いログを外部バッチで定期削除する。

## 依存
- `documents/issues/002-audit-log-schema-model.md`
- `documents/issues/003-audit-log-write-path.md`

## 作業タスク
- [ ] 削除対象の監査ログを取得するSQLクエリを作成する
- [ ] バッチスクリプト（シェルまたはCLI）を作成する
- [ ] 環境変数 `AUDIT_LOG_RETENTION_DAYS` / `AUDIT_LOG_RETENTION_FOREVER` を参照する
- [ ] ドライラン機能を追加する（削除件数を表示のみ）
- [ ] 実行ログをファイルまたは標準出力に記録する
- [ ] cron / タスクスケジューラでの実行手順をドキュメント化する

## 受入条件
- `AUDIT_LOG_RETENTION_FOREVER=true` の場合は削除しない
- `AUDIT_LOG_RETENTION_DAYS=N` 日より古いログを削除できる
- ドライランで影響件数を確認できる
- 削除実行後にログに記録が残る

## テスト
- ドライラン動作確認
- 保持期間設定に基づく削除動作テスト

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
- API サーバーとは別プロセスで動作（外部バッチ）
