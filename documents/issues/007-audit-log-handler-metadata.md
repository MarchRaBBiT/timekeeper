# 監査ログ: ハンドラ別 metadata 付与

## 目的
- 各ハンドラの文脈に応じた metadata を監査ログへ付与する。

## 依存
- `documents/issues/001-audit-log-event-catalog.md`
- `documents/issues/003-audit-log-write-path.md`

## 作業タスク
- [x] 打刻系 API で metadata（例: clock_type, timezone, source）を追加する
- [x] 申請系 API で metadata（例: request_type, payload_summary）を追加する
- [x] 承認系 API で metadata（例: approval_step, decision）を追加する
- [x] パスワード変更系 API で metadata（例: method, mfa_enabled）を追加する
- [x] 失敗時の metadata 記録方針を統一する

## 受入条件
- 対象 API の監査ログに metadata が付与される
- 失敗イベントでも metadata の形式が揃っている

## テスト
- 対象 API ごとの監査ログ内容を確認する統合テスト

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
