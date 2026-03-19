# CSV 一括インポート ガイド

**対象バージョン:** 本機能は 2026-03-18 のリリースで追加されました。
**必要権限:** システム管理者 (`is_system_admin = true`) のみ

---

## 概要

CSV ファイルを使って **部署** と **ユーザー** を一括登録できます。大規模な初期セットアップや組織変更時に、フォームでの 1 件ずつの登録作業を不要にします。

**動作方針: 全-or-なし**
1 行でもバリデーションエラーがあれば、一切データを書き込みません。すべての行が正常と確認されてはじめてトランザクション内で一括 INSERT されます。

---

## 1. 部署の一括インポート

### 1.1 アクセス方法

管理画面 → **部署管理** タブの下部に「部署 CSV 一括インポート」セクションが表示されます。
（システム管理者以外には表示されません）

### 1.2 CSV フォーマット

```csv
name,parent_name
Engineering,
Frontend,Engineering
Backend,Engineering
QA,Engineering
HR,
```

| 列 | 必須 | 説明 |
|---|---|---|
| `name` | ○ | 部署名。空文字列は不可。CSV 内で一意でなければなりません |
| `parent_name` | △ | 親部署の名前。ルート部署にする場合は空にする |

**`parent_name` の解決順序:**
1. 同一 CSV 内の `name` と一致する行を探す
2. 見つからなければデータベース内の既存部署を検索する
3. どちらにも存在しない場合はエラー

### 1.3 操作手順

1. ヘッダー行を含む CSV ファイルを用意します
2. 「ファイルを選択」でファイルを選ぶと先頭 5 行がプレビュー表示されます
3. 内容を確認して **インポート実行** を押します
4. 完了後、成功件数またはエラー一覧が表示されます

### 1.4 エラー一覧の見方

| エラーメッセージ例 | 原因 | 対処 |
|---|---|---|
| `name must not be empty` | name 列が空 | 行を削除するか name を記入 |
| `Duplicate name 'X' in CSV` | CSV 内に同名の行が複数ある | 重複行を削除 |
| `Parent department 'X' not found in CSV or database` | parent_name が CSV にも DB にも存在しない | 親部署を先に作成するか CSV に含める |
| `Circular dependency detected involving 'X'` | 部署間の親子関係が循環している（A→B→A など） | 親子関係を見直す |

### 1.5 サンプル CSV

```csv
name,parent_name
本社,
東日本営業部,本社
西日本営業部,本社
技術部,本社
フロントエンドチーム,技術部
バックエンドチーム,技術部
```

---

## 2. ユーザーの一括インポート

### 2.1 アクセス方法

管理画面 → **ユーザー管理** → **アクティブ** タブの「ユーザー招待」フォームの下に「ユーザー CSV 一括インポート」セクションが表示されます。

### 2.2 CSV フォーマット

```csv
username,password,full_name,email,role,is_system_admin,department_name
alice,SecurePass1!,Alice Smith,alice@example.com,employee,false,Engineering
bob,SecurePass2!,Bob Jones,bob@example.com,manager,false,
carol,SecurePass3!,Carol Admin,carol@example.com,employee,true,HR
```

| 列 | 必須 | 説明 |
|---|---|---|
| `username` | ○ | ログイン名。空文字列不可・CSV 内で一意・DB 内未登録 |
| `password` | ○ | 初期パスワード。パスワードポリシー（大文字・小文字・数字・記号・最低 12 文字）を満たすこと |
| `full_name` | ○ | 氏名。空文字列不可 |
| `email` | ○ | メールアドレス。`@` を含む形式。CSV 内で一意・DB 内未登録 |
| `role` | ○ | `employee` または `manager` のいずれか |
| `is_system_admin` | ○ | `true` または `false` |
| `department_name` | △ | 所属部署名。省略可（空文字列で部署なし）。指定した場合は DB に存在する部署名でなければならない |

> **パスワードポリシー** はシステム設定 (`password_min_length` など) に従います。デフォルトは 12 文字以上、大文字・小文字・数字・記号をそれぞれ 1 文字以上含むこと。

### 2.3 操作手順

1. 上記フォーマットの CSV ファイルを用意します
2. 「ファイルを選択」でファイルを選ぶと先頭 5 行がプレビュー表示されます
3. 内容を確認して **インポート実行** を押します
4. 完了後、成功件数またはエラー一覧が表示されます

### 2.4 エラー一覧の見方

| エラーメッセージ例 | 原因 | 対処 |
|---|---|---|
| `username must not be empty` | username 列が空 | username を記入 |
| `Duplicate username 'X' in CSV` | CSV 内に同じ username が複数ある | 重複行を修正 |
| `Username 'X' already exists` | DB にすでに同名ユーザーが存在する | 別の username を使用 |
| `Invalid email format: 'X'` | `@` がないなどメール形式が不正 | メールアドレスを修正 |
| `Email 'X' already exists` | 同じメールアドレスが DB に登録済み | 別のメールアドレスを使用 |
| `Invalid role 'X': must be 'employee' or 'manager'` | role 列の値が不正 | `employee` または `manager` に修正 |
| `Invalid is_system_admin 'X': must be 'true' or 'false'` | is_system_admin の値が不正 | `true` または `false` に修正 |
| `Password policy violation: ...` | パスワードがポリシーを満たしていない | パスワードを変更（大文字・小文字・数字・記号・最低 12 文字） |
| `Department 'X' not found` | 指定した部署名が DB に存在しない | 部署名を確認するか、先に部署を作成する |

### 2.5 サンプル CSV

```csv
username,password,full_name,email,role,is_system_admin,department_name
yamada.taro,Passw0rd!Tokyo,山田太郎,yamada@example.com,employee,false,東日本営業部
suzuki.hanako,Passw0rd!Osaka,鈴木花子,suzuki@example.com,manager,false,西日本営業部
tanaka.jiro,Passw0rd!Tech,田中次郎,tanaka@example.com,employee,false,バックエンドチーム
sato.admin,Passw0rd!Admin,佐藤管理者,sato@example.com,employee,true,
```

---

## 3. インポート後の作業

### 部署インポート後

- インポート成功後、部署一覧が自動更新されます
- 親子関係はツリー順に正しく設定されています
- マネージャーの割り当ては個別に **部署管理** → 各部署の「マネージャー割り当て」から行ってください

### ユーザーインポート後

- インポートされたユーザーは **アクティブ** タブに表示されます
- 初期パスワードは CSV に記載したものです。ユーザーへの安全な通知方法（別経路）で伝えてください
- MFA はユーザー自身が `/mfa/register` から登録します

---

## 4. よくある質問

**Q. 既存の部署を上書きできますか？**
A. できません。一括インポートは新規作成のみです。既存部署の変更は管理画面の編集フォームから行ってください。

**Q. 1 回のインポートで登録できる件数に上限はありますか？**
A. API 側のハード上限はありませんが、パスワードハッシュの計算が CPU-bound のため、数百件以上は数秒かかる場合があります。分割インポートを推奨します。

**Q. 部署インポートとユーザーインポートの順番はありますか？**
A. ユーザー CSV で `department_name` を指定する場合は、先に部署を登録（既存 or 部署 CSV インポート）しておく必要があります。

**Q. インポート失敗後、再試行できますか？**
A. はい。全-or-なし方針のため、エラーが出ても DB は変更されていません。CSV を修正してそのまま再インポートできます。

**Q. 監査ログに記録されますか？**
A. はい。インポート実行は `admin_bulk_import_departments` / `admin_bulk_import_users` のイベントタイプで監査ログに残ります（`/admin/audit-logs` で確認可）。

---

## 5. API リファレンス（直接呼び出し）

システム管理者の認証トークンがあれば API を直接呼び出すことも可能です。

### 部署インポート

```http
POST /api/admin/bulk-import/departments
Authorization: Bearer <token>
Content-Type: application/json

{
  "csv_data": "name,parent_name\nEngineering,\nFrontend,Engineering\n"
}
```

### ユーザーインポート

```http
POST /api/admin/bulk-import/users
Authorization: Bearer <token>
Content-Type: application/json

{
  "csv_data": "username,password,full_name,email,role,is_system_admin,department_name\nalice,SecurePass1!,Alice,alice@example.com,employee,false,\n"
}
```

### レスポンス形式（共通）

**成功（全行正常）:**
```json
{
  "imported": 3,
  "failed": 0,
  "errors": []
}
```

**バリデーションエラーあり（全行ロールバック）:**
```json
{
  "imported": 0,
  "failed": 2,
  "errors": [
    { "row": 2, "message": "Username 'alice' already exists" },
    { "row": 4, "message": "Invalid role 'superuser': must be 'employee' or 'manager'" }
  ]
}
```

- HTTP ステータスは成功時も失敗時も **200 OK**
- `imported > 0` であれば DB への書き込みが完了
- `errors` が空でない場合は何も書き込まれていない
- 認証エラーは `401`、権限エラーは `403` を返す
