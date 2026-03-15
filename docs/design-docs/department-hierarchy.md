# 部署階層と部署マネージャー承認モデル

**作成:** 2026-03-15
**対応 PR:** #456
**対応 Issue:** #452–455
**ステータス:** 実装済み（2026-03-15 main マージ）

---

## 概要

Timekeeper の権限モデルを「全 Admin が全申請を承認できるフラット構造」から
「マネージャーが担当部署および下位部署のメンバーの申請のみ承認できる階層スコープ構造」へ移行した。

---

## 背景と動機

- 旧モデルでは `admin` ロールを持つ全ユーザーが全従業員の申請を承認・閲覧できた
- 組織のコンプライアンス要件として「承認者は担当部署のメンバーの申請のみ処理できる」制約が必要になった
- 部署を可変深さの自己参照ツリーとして表現することで、任意の組織階層に対応できるようにした

---

## データモデル

### テーブル定義

#### `departments`（migration 039）

```sql
CREATE TABLE departments (
    id        TEXT PRIMARY KEY,           -- DepartmentId (UUID)
    name      TEXT NOT NULL,
    parent_id TEXT REFERENCES departments(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_departments_parent_id ON departments(parent_id);
```

- `parent_id = NULL` の行がルートノード（最上位部署）
- `ON DELETE RESTRICT` により、子部署が存在する部署の削除を DB レベルで防止
- 可変深さのツリー構造。深さに制限は設けていないが、再帰 CTE の計算量がツリー深さに比例する点に注意

#### `department_managers`（migration 040）

```sql
CREATE TABLE department_managers (
    department_id TEXT NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL REFERENCES users(id)       ON DELETE CASCADE,
    assigned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (department_id, user_id)
);
CREATE INDEX idx_department_managers_user_id ON department_managers(user_id);
```

- 1 人のマネージャーは複数の部署を担当できる（多対多）
- 部署が削除されると担当割り当ても自動削除（CASCADE）

#### `users` への追加（migration 041）

```sql
ALTER TABLE users ADD COLUMN department_id TEXT REFERENCES departments(id) ON DELETE SET NULL;
CREATE INDEX idx_users_department_id ON users(department_id);
```

- 従業員は 0 または 1 部署に所属（`NULL` = 未所属）
- 部署が削除されると `department_id = NULL` になる（SET NULL）

#### ロール移行（migration 042）

```sql
UPDATE users        SET role = 'manager' WHERE role = 'admin';
UPDATE archived_users SET role = 'manager' WHERE role = 'admin';
```

---

## ロール体系

| ロール | 説明 |
|--------|------|
| `employee` | 一般従業員。自分の申請のみ操作可能 |
| `manager` | 担当部署および下位部署メンバーの申請を承認・閲覧可能 |
| `is_system_admin: bool` | ロールと独立したフラグ。全社操作・ユーザー管理・subject_requests を担当 |

### 後方互換 Deserialize

既存 DB に `role = 'admin'` が残っていた場合でも正しく動作するよう、
`"admin"` / `"Admin"` / `"ADMIN"` → `UserRole::Manager` にマップする後方互換デシリアライザを実装した。

```rust
// backend/src/models/user.rs
fn deserialize_role<'de, D>(deserializer: D) -> Result<UserRole, D::Error>
```

---

## 承認・閲覧の認可モデル

### 判定優先順位

```
1. is_system_admin == true  → 無条件 OK
2. role == Manager          → 部署スコープ判定（下記 SQL）
3. それ以外                 → 403 Forbidden
```

### コア SQL — `can_manager_approve(manager_id, applicant_id)`

```sql
WITH RECURSIVE subordinate_depts AS (
    -- マネージャーの直属担当部署
    SELECT dm.department_id
    FROM department_managers dm
    WHERE dm.user_id = $1
    UNION ALL
    -- 再帰的に子部署を展開
    SELECT d.id FROM departments d
    INNER JOIN subordinate_depts sd ON d.parent_id = sd.department_id
)
SELECT EXISTS (
    SELECT 1 FROM users u
    WHERE u.id = $2
      AND u.department_id IN (SELECT department_id FROM subordinate_depts)
) AS can_approve
```

- `WITH RECURSIVE` を使ったツリー探索により、任意の深さの部署ツリーに対応
- `EXISTS` で最初に一致した時点で探索を打ち切るため効率的

### 閲覧スコープ — `list_subordinate_user_ids(manager_id)`

承認だけでなく **一覧閲覧・詳細表示・CSV エクスポート** も同じ部署スコープで制限する。
`can_manager_approve` と同じ再帰 CTE で配下ユーザーの ID リストを取得し、
各クエリの `WHERE user_id = ANY($ids)` フィルターとして注入する。

```
Handler (list_requests, list_attendance_corrections, export_data)
  ↓ if manager && !system_admin
  list_subordinate_user_ids(manager_id)
  ↓
  RequestListFilters { allowed_user_ids: Some(ids) }
  ↓
  apply_request_filters → WHERE user_id = ANY($ids)
```

### サイクル防止 — `would_create_cycle(dept_id, new_parent_id)`

部署ツリーの `parent_id` 更新時にサイクルを防ぐ。

```sql
WITH RECURSIVE descendants AS (
    SELECT id FROM departments WHERE id = $1  -- dept_id
    UNION ALL
    SELECT d.id FROM departments d
    INNER JOIN descendants desc ON d.parent_id = desc.id
)
SELECT EXISTS (SELECT 1 FROM descendants WHERE id = $2) AS creates_cycle
```

`new_parent_id` が `dept_id` 自身または子孫に含まれる場合 `400 BadRequest` を返す。

---

## 実装ファイルマップ

### バックエンド

| ファイル | 役割 |
|---------|------|
| `backend/migrations/039–042` | DB スキーマ変更（departments, department_managers, users.department_id, role 移行） |
| `backend/src/types/id.rs` | `DepartmentId` typed ID |
| `backend/src/models/department.rs` | `Department`, `DepartmentManager`, payload 型 |
| `backend/src/models/user.rs` | `UserRole::Manager`（旧 Admin）、後方互換 Deserialize、`User.department_id` |
| `backend/src/repositories/department.rs` | CRUD + `can_manager_approve` + `list_subordinate_user_ids` + `would_create_cycle` |
| `backend/src/repositories/user.rs` | 全 SELECT/INSERT/RETURNING に `department_id` 追加、`update_user` に `department_id` 引数 |
| `backend/src/repositories/request.rs` | `RequestListFilters.allowed_user_ids` 追加 |
| `backend/src/repositories/attendance_correction_request.rs` | `list_paginated` に `allowed_user_ids` 追加 |
| `backend/src/handlers/admin/common.rs` | `check_approval_authorization()` 共通ヘルパー |
| `backend/src/handlers/admin/departments.rs` | 部署管理 CRUD + マネージャー割り当て API |
| `backend/src/handlers/admin/requests.rs` | approve/reject に `check_approval_authorization`、list/detail に部署スコープ |
| `backend/src/handlers/admin/attendance_correction_requests.rs` | 同上 |
| `backend/src/handlers/admin/export.rs` | マネージャー時に部署スコープでフィルタ |
| `backend/src/handlers/admin/subject_requests.rs` | `is_system_admin()` 専用に変更 |
| `backend/src/main.rs` | 部署管理ルート（`admin_routes` / `system_admin_routes`）追加 |

### フロントエンド

| ファイル | 役割 |
|---------|------|
| `frontend/src/api/types.rs` | `DepartmentResponse`, `CreateDepartmentRequest` 等の型 |
| `frontend/src/api/client.rs` | `admin_list_departments`, `admin_create_department`, `admin_delete_department` 等 |
| `frontend/src/components/guard.rs` | `RequireAdmin` が `"manager"` ロールを受け入れるよう更新 |
| `frontend/src/components/layout.rs` | ナビの管理メニュー表示を `"manager"` に対応 |
| `frontend/src/pages/admin/components/departments.rs` | `DepartmentsPanel`（一覧・作成・削除） |
| `frontend/src/pages/admin_departments/` | `AdminDepartmentsPage`（`/admin/departments` ルート） |
| `frontend/src/pages/admin/panel.rs` | `AdminSubjectRequestsSection` を `system_admin_allowed` でガード |
| `frontend/src/pages/admin/view_model.rs` | `subject_requests_resource` を `system_admin_allowed` でトリガー |
| `frontend/src/pages/admin_users/components/invite_form.rs` | ロール選択 `"admin"` → `"manager"` |

---

## API エンドポイント

詳細は [backend-api-catalog.md](./backend-api-catalog.md) の「Admin / Department Management」セクションを参照。

| Method | Path | 認可 |
|--------|------|------|
| GET | `/api/admin/departments` | manager+ |
| GET | `/api/admin/departments/:id` | manager+ |
| POST | `/api/admin/departments` | system_admin |
| PUT | `/api/admin/departments/:id` | system_admin |
| DELETE | `/api/admin/departments/:id` | system_admin |
| POST | `/api/admin/departments/:id/managers` | system_admin |
| DELETE | `/api/admin/departments/:id/managers/:uid` | system_admin |

---

## 既知の制約と将来の課題

### 設計上の決定事項

| 項目 | 決定 |
|------|------|
| 最上位マネージャーの自己申請 | `pending` のまま残る。`is_system_admin` による手動承認が必要（運用対応） |
| 従業員の複数部署所属 | 非対応（1人1部署）。変更する場合は `users.department_id` を `user_departments` 多対多テーブルに置き換える必要がある |
| 部署削除時の承認スコープ影響 | 部署が削除されると `users.department_id = NULL` になる。既存の pending 申請は影響を受けないが、承認者が変わる可能性がある |
| `list_subordinate_user_ids` のタイミング | 各リクエスト時に再計算。部署ツリーが大きくなるとクエリコストが増加するが、HR システムのスケールでは許容範囲内 |

### 未実装（`#[allow(dead_code)]` で保留中）

- 部署更新 UI（`admin_update_department`, フロントエンドの更新フォーム）
- マネージャー割り当て UI（`admin_assign_manager` / `admin_remove_manager`、フロントエンドの割り当てパネル）
- ユーザー招待・編集フォームへの部署選択ドロップダウン

これらは backend API・クライアントメソッド・型定義まで実装済みであり、UI のみ未実装。

---

## テストカバレッジ

### 統合テスト

| ファイル | カバー範囲 |
|---------|-----------|
| `backend/tests/admin_departments_api.rs` | 部署 CRUD 権限テスト（8 ケース） |
| `backend/tests/department_approval_api.rs` | 承認スコープ境界値テスト（8 ケース） |

### 承認スコープ境界値

```
✅ manager が直属部署メンバーを承認できる
✅ manager が 3 段階下位部署メンバーを承認できる
✅ is_system_admin が全員を承認できる（後方互換）
❌ manager が担当外部署メンバーを承認できない（403）
❌ manager が兄弟部署メンバーを承認できない（403）
❌ manager が自分の申請を承認できない（403）
❌ employee が誰の申請も承認できない（403）
✅ manager が直属部署メンバーを却下できる
```
