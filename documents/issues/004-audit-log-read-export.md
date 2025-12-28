# 監査ログ: Read API と JSON エクスポート

## 目的
- 監査ログの一覧/詳細/JSON エクスポートをシステム管理者向けに提供する。

## 依存
- `documents/issues/001-audit-log-event-catalog.md`
- `documents/issues/002-audit-log-schema-model.md`

## 作業タスク
- [ ] `GET /api/admin/audit-logs` で一覧を取得できるようにする（期間/actor/event_type/target/result で絞り込み + ページング）
- [ ] `GET /api/admin/audit-logs/{id}` で詳細取得を追加する
- [ ] `GET /api/admin/audit-logs/export` で JSON エクスポートを追加する
- [ ] システム管理者のみアクセス可能な認可チェックを共通化する
- [ ] `API_DOCS.md` にエンドポイント仕様を記載する

## 受入条件
- 管理者のみ一覧/詳細/エクスポートにアクセスできる
- フィルタとページングが指定通り動作する
- JSON エクスポートは `Content-Disposition` でファイルとして取得できる

## テスト
- 認可/フィルタ/エクスポートの API テストを追加する

## メモ
- 実装開始時は本 issue を読み込んだ後にトピックブランチを作成すること
