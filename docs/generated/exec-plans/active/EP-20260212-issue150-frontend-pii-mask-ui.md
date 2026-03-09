# EP-20260212-issue150-frontend-pii-mask-ui

## Goal
- issue #150 の未実装差分として、PIIマスキング状態をフロントエンドで明示表示する

## Scope
- In: `frontend/src/api/*`, `frontend/src/pages/admin_users/*`, `frontend/src/pages/admin_export/*`, `frontend/src/pages/admin_audit_logs/*`
- Out: 新規バックエンドAPI追加、DBマイグレーション変更

## Done Criteria (Observable)
- [x] `X-PII-Masked` ヘッダをAPIクライアントで受け取れる
- [x] 管理画面でマスキング適用中のバナー/注記が表示される
- [x] 既存関連テストが成功する

## Constraints / Non-goals
- 既存 API レスポンスの JSON 互換性は壊さない
- 権限判定ロジックはバックエンド仕様に追従し、フロントは表示のみ追加

## Task Breakdown
1. [x] API型に `pii_masked` 付きレスポンス型を追加
2. [x] Users/Export/Audit API にヘッダ読み取りメソッドを追加
3. [x] 各 ViewModel に `pii_masked` 状態を追加
4. [x] 各パネルに注意表示を追加
5. [x] fmt + frontend対象テストで回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-frontend --lib admin_users`
- [x] `cargo test -p timekeeper-frontend --lib admin_export`
- [x] `cargo test -p timekeeper-frontend --lib admin_audit_logs`

## JJ Snapshot Log
- [x] `jj status`
- [x] frontend対象テスト pass
- [ ] `jj commit -m "feat(frontend): surface pii masking state in admin views"`

## Progress Notes
- 2026-02-12: 実装開始
- 2026-02-12: APIクライアントで `X-PII-Masked` を取り込み、admin users/export/audit logs でマスキング表示バナーを実装。frontend関連テスト成功。

