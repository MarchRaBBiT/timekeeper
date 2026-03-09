# 監査ログテーブル マイグレーションガイド

## 概要

マイグレーション `018_create_audit_logs.sql` は、監査ログの新スキーマを導入し、既存テーブルがある場合はリネームして保持します。

## マイグレーション内容

```sql
-- 既存テーブルをリネーム（既存データを保持）
ALTER TABLE IF EXISTS audit_logs RENAME TO audit_logs_legacy;

-- 新スキーマでテーブル作成
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL,
    actor_id TEXT,
    actor_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    result TEXT NOT NULL,
    error_code TEXT,
    metadata JSONB,
    ip TEXT,
    user_agent TEXT,
    request_id TEXT
);

-- インデックス作成
CREATE INDEX idx_audit_logs_occurred_at ON audit_logs(occurred_at);
CREATE INDEX idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX idx_audit_logs_target_id ON audit_logs(target_id);
CREATE INDEX idx_audit_logs_request_id ON audit_logs(request_id);
```

## 既存データの扱い

### ケース1: 新規環境（既存テーブルなし）

マイグレーションはそのまま実行され、新テーブルが作成されます。

### ケース2: 旧スキーマからの移行

1. 既存の `audit_logs` テーブルは `audit_logs_legacy` にリネームされます
2. 新しい `audit_logs` テーブルが作成されます
3. 既存データは `audit_logs_legacy` に保持されます

### レガシーデータの移行（必要な場合）

旧スキーマのデータを新テーブルに移行する場合は、以下のSQLを参考にしてください：

```sql
-- 例: 旧スキーマのカラムに応じて調整してください
INSERT INTO audit_logs (id, occurred_at, actor_id, actor_type, event_type, target_type, target_id, result, error_code, metadata, ip, user_agent, request_id)
SELECT 
    id,
    occurred_at,
    actor_id,
    actor_type,
    event_type,
    target_type,
    target_id,
    result,
    error_code,
    metadata,
    ip,
    user_agent,
    request_id
FROM audit_logs_legacy;
```

### レガシーテーブルの削除

移行が完了し、レガシーデータが不要になった場合：

```sql
DROP TABLE IF EXISTS audit_logs_legacy;
```

## 実行手順

### 開発環境

```bash
cd backend
cargo sqlx migrate run
```

### 本番環境

1. データベースのバックアップを取得
2. マイグレーションを実行
3. アプリケーションを再起動
4. 動作確認後、必要に応じてレガシーテーブルを削除

## ロールバック

マイグレーションをロールバックする場合（手動）：

```sql
-- 新テーブルを削除
DROP TABLE IF EXISTS audit_logs;

-- レガシーテーブルを元に戻す
ALTER TABLE IF EXISTS audit_logs_legacy RENAME TO audit_logs;
```

## 注意事項

- `audit_logs_legacy` はアプリケーションから参照されないため、移行期間後に削除を検討してください
- 本番環境での実行前に、ステージング環境でテストすることを推奨します
