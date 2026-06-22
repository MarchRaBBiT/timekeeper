# EP-20260621-workday-override-and-schedule-read-api

## Goal

- 勤務体系マスタ Phase 1 の残API4本を実装し、従業員の予定閲覧・マネージャーの予定閲覧・日別例外のupsert/削除を契約境界で提供して Phase 1 をクローズする

## Scope

- In:
  - `GET /api/work-schedules/me`（従業員が自分の解決済み予定を `from`/`to` で閲覧）
  - `GET /api/admin/users/{user_id}/resolved-workdays`（scoped manager+ が配下の予定を閲覧）
  - `PUT /api/admin/users/{user_id}/workday-overrides/{date}`（authorized manager が日別例外を upsert）
  - `DELETE /api/admin/users/{user_id}/workday-overrides/{date}`（authorized manager が未ロック例外を削除）
  - app use case（`ListUserWorkdays`, `SetWorkdayOverride`, `DeleteWorkdayOverride`）
  - contract DTO、infra-postgres repository メソッド、handler、route、error mapping、API catalog更新
  - unit / integration tests
- Out: anomaly生成、未来projection worker、管理者カレンダーUI、月次締め、frontend表示、flex/変形労働（Phase 2/3）

## Done Criteria (Observable)

- [x] `GET /api/work-schedules/me` が認証ユーザー本人の解決済み予定を `from`/`to` 範囲で返す
- [x] `from`/`to` 逆転・範囲過大は `400 INVALID_WORK_SCHEDULE` を返す（必須queryのため欠落はAxumが400相当で拒否）
- [x] `GET /api/admin/users/{id}/resolved-workdays` は system admin で任意ユーザー、manager で配下ユーザーのみ閲覧でき、スコープ外は `403`
- [x] `PUT .../workday-overrides/{date}` が `non_working_day` / `use_schedule` を upsert し、`200` で確定状態を返す
- [x] `use_schedule` で `work_schedule_id` 欠落、`non_working_day` で `work_schedule_id` 指定、`reason` 空/501字以上は `400`
- [x] 対象日に locked な resolved workday がある override の upsert/削除は `409 RESOLVED_WORKDAY_LOCKED`
- [x] manager がスコープ外ユーザーへ override すると `403`
- [x] `DELETE .../workday-overrides/{date}` は未ロック例外を削除し `204`、存在しなければ `404`
- [x] resolver読み取り規則は変更せず、既存 resolve/clock-in/out 挙動が不変（attendance_work_schedule_integration 7 passed）
- [~] 新規・変更モジュールの line coverage 80%以上: 全分岐をtestで網羅。cargo-llvm-cov 未導入で数値計測は保留

## Constraints / Non-goals

- 解決規則・SQL・優先順位は `crates/app` / `crates/infra-postgres` に維持し、handler へ置かない
- migration は追加しない（044の `workday_overrides` / `resolved_workdays` を再利用）
- locked検証は DB trigger を source of truth とし、app側は事前チェックで先行409を返す
- SQL入力はすべて bind parameter
- 認証・CSRF・rate limit は既存 route 境界を維持する
- 部署スコープ認可は既存 `department::can_manager_approve` を再利用する

## Task Breakdown

1. [x] app unit tests と API integration tests を RED にする
2. [x] app: `ListUserWorkdays` / `SetWorkdayOverride` / `DeleteWorkdayOverride` use case と repository trait 拡張を実装する
3. [x] infra-postgres: `list_resolved_in_range` / `upsert_override` / `delete_override` と locked 事前チェックを実装する
4. [x] contract: `ResolvedWorkday*` / `SetWorkdayOverrideRequest` / `WorkdayOverrideResponse` DTO を追加する
5. [x] backend: handler 4本、route 登録、error mapping（forbidden/locked/not_found/bad_request）を実装する
6. [x] API catalog と勤務体系設計の Phase 1 ステータスを更新する
7. [x] focused tests、fmt、clippy、harness、security review を実行する（review agentは529過負荷で実行不能、手動レビューで代替・CRITICAL/HIGHなし）
8. [x] git snapshot を作成する

## Validation Plan

- [x] `cargo test -p timekeeper-app`（user_workdays 4 + workday_overrides 8 = 新規12 passed）
- [x] `cargo test -p timekeeper-backend --test work_schedule_read_api` — 6 passed
- [x] `cargo test -p timekeeper-backend --test workday_override_api` — 8 passed
- [x] 回帰: admin_work_schedules_api 11 / attendance_work_schedule_integration 7 / docs_api 3 passed
- [x] `bash scripts/harness.sh backend-unit` — 369 passed
- [x] `bash scripts/harness.sh docs-check`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [~] changed-module line coverage: cargo-llvm-cov 未導入。新規ロジックの全分岐（範囲検証・kind/schedule整合・reason・locked・スコープ・not-found）を unit/integration test で網羅

## Git Snapshot Log

- [x] `git status --short`
- [x] focused tests pass
- [x] `git commit -m "feat: add workday override and schedule read api"`

## Progress Notes

- 2026-06-21: connect-punches完了後の Phase 1 残タスクとして、予定閲覧2本＋日別例外2本を1計画に集約。スキーマ(044)・resolver型・`can_manager_approve` 既存を確認し、migration不要で着手可能と判断した。
- 2026-06-22: app use case 3種を新規モジュール(`user_workdays`/`workday_overrides`)で実装しmock testで分岐固定。infra-postgres `management.rs` で範囲読み取り・override upsert/delete・locked事前チェックを実装。contract DTO追加、handler 4本とroute登録、OpenAPI doc.rs登録、API catalog/設計docのPhase 1完了反映まで実施。
- 2026-06-22: 認可は既存パターン踏襲で system_admin バイパス + manager は `can_manager_approve` 部署スコープ。override は admin_routes(auth_admin=manager+) に置き、handler内でスコープ検証。locked判定は resolved_workdays.locked_at を gate にし DB trigger と二重防御。
- 2026-06-22: app 12 / read API 6 / override API 8 / 回帰(admin 11, integration 7, docs 3) すべて green。fmt・workspace clippy・docs-check green。
