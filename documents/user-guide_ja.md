# Timekeeper ユーザーガイド

本ガイドは、従業員・管理者・システム管理者それぞれの利用フローをまとめたものです。`README.md` や `API_DOCS.md`、最新の管理画面仕様をもとにしています。

## 1. 役割と権限

| ロール | 主な権限 |
| --- | --- |
| 従業員 (`role=employee`) | 出退勤・休憩の記録、休暇/残業/修正申請の作成、履歴閲覧、MFA 登録/解除、パスワード変更、自己データのエクスポート |
| 管理者 (`role=admin`, `is_system_admin=false`) | 従業員同等の機能に加え、残業・休暇・修正申請の承認/却下、休日管理、CSV エクスポート |
| システム管理者 (`is_system_admin=true`) | 管理者機能に加え、ユーザー管理、勤怠データの直接編集、強制休憩終了、MFA リセット、休日管理、CSV エクスポート、パスワードリセットなど全権限 |

> MFA リセットおよびパスワードリセットは必ずシステム管理者が実施してください。

## 2. ログインと MFA

1. フロントエンド（既定では `http://localhost:8000`）にアクセスし、`/login` を開きます。
2. ユーザー名とパスワードを入力します。MFA が有効な場合は 6 桁の TOTP コードも求められます。
3. ログイン成功後、ブラウザの `localStorage` にアクセストークンとリフレッシュトークンが保存されます。

### MFA の登録手順

1. `/mfa/register` に移動し、**MFA設定** ボタンを押して QR コードとシークレットを発行します。
2. 認証アプリで QR コードを読み取り、表示された確認コードを入力します。
3. 無効化したい場合は同ページで現在の TOTP コードを送信します。
4. 端末を紛失した場合はシステム管理者に **Admin -> MFA Reset** を依頼し、再度登録してください。

## 3. 従業員向けの基本操作

### 勤怠アクション

| 操作 | UI | API |
| --- | --- | --- |
| 出勤 | ダッシュボード -> **勤務開始 (Clock In)** | `POST /api/attendance/clock-in` |
| 休憩開始/終了 | Attendance ページのボタン | `POST /api/attendance/break-start` / `POST /api/attendance/break-end` |
| 退勤 | ダッシュボード -> **勤務終了 (Clock Out)** | `POST /api/attendance/clock-out` |
| ステータス/履歴の確認 | Attendance -> 「本日の状況 / 履歴」 | `GET /api/attendance/status`, `GET /api/attendance/me` |
| CSV エクスポート | Attendance -> 「CSV エクスポート」カード | `GET /api/attendance/export` |

### 申請フロー（休暇/残業/修正）

1. `/requests` で申請種別を選択し、期間や理由を入力して送信します。
2. ステータス（`pending` / `approved` / `rejected` / `cancelled`）は同ページで確認できます。
3. `pending` の間は編集・取消が可能です。

## 4. 管理者（承認担当）

`role=admin` のユーザーはヘッダーに **Admin** が表示され、`/admin` ダッシュボードへアクセスできます。

1. **申請一覧** - ステータスやユーザーでフィルタし、詳細モーダルからコメント付きで承認/却下します。  
   - 承認: `PUT /api/admin/requests/:id/approve`  
   - 却下: `PUT /api/admin/requests/:id/reject`
2. **詳細モーダル** - 承認前に JSON ペイロードを確認できます。

3. **休日管理とCSVエクスポート** - ページ下部のカードから休日の追加/削除や Google カレンダー取り込み、勤怠 CSV のダウンロードが可能です。

システム管理者でない管理者は上記機能を利用できますが、ユーザー管理や勤怠手動修正、MFA リセットは引き続きシステム管理者専用です。

## 5. システム管理者コンソール

システム管理者はナビゲーションに **ユーザー管理** が追加され、`/admin` で以下の追加ツールを利用できます（休日管理と CSV エクスポートは通常の管理者も利用可能です）。

### 5.1 ユーザー管理 (`/admin/users`)

- `POST /api/admin/users` で従業員・管理者を作成。初期パスワードもここで設定します。
- **システム管理者** チェックボックスで最上位権限を付与。
- 一覧テーブルでユーザー名/氏名/ロール/システム管理者フラグを確認。

### 5.2 勤怠データの手動編集

- 「勤務の手動登録（アップサート）」フォームから `PUT /api/admin/attendance` を呼び出し、ユーザーID・日付・出退勤・休憩区間を直接登録/上書きできます。
- 「休憩の強制終了」ではブレーク ID を指定し、`PUT /api/admin/breaks/:id/force-end` で未終了休憩を即時終了させます。

### 5.3 MFA リセット

- `/api/admin/users` で取得したユーザー一覧から対象を選び、**Reset MFA** ボタンを押すと `POST /api/admin/mfa/reset` が実行されます。
- リセット後は該当ユーザーのリフレッシュトークンも全て破棄され、再ログインが必要になります。

### 5.4 休日管理

- （管理者/システム管理者共通）手動追加（`POST /api/admin/holidays`）に加え、Google Calendar ICS から `GET /api/admin/holidays/google` で休日候補を取り込み可能。
- 既存の休日は `DELETE /api/admin/holidays/:id` で削除できます。

### 5.5 CSV エクスポート

- （管理者/システム管理者共通）`/admin/export` ではユーザー名・日付範囲で絞り込んだ勤怠 CSV をダウンロードできます（`GET /api/admin/export`）。
- 取得したファイルの冒頭は画面上でもプレビューされます。

## 6. ログアウトとパスワード

- ヘッダーの **ログアウト** ボタンから `POST /api/auth/logout` を呼び出します。`{"all": true}` を送ると全端末のリフレッシュトークンを一括失効できます。
- パスワード変更は `/api/auth/change-password`（最低 8 文字・現在のパスワードと異なる必要あり）で実行します。

## 7. トラブルシューティング

1. **401 Unauthorized** - トークンの期限切れ/欠落。再ログインまたはリフレッシュを試してください。
2. **403 Forbidden** - システム管理者限定機能を一般管理者で実行している可能性があります。
3. **マイグレーション不整合** - `cargo run` または `cargo sqlx migrate run` で最新スキーマを適用してください。
4. **フロントが表示されない** - `wasm-pack build --dev` を再実行し、Python サーバを再起動。Playwright 実行時は `FRONTEND_BASE_URL` を適切に設定してください。

API の詳細は `API_DOCS.md`、環境構築の詳細は `documents/environment-setup.md` を参照してください。
