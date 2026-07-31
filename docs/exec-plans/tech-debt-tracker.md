# Technical Debt Tracker

## Purpose

このファイルは、現行コードと設計に存在する技術的負債を「後で読むメモ」ではなく、優先順位付きの実行バックログとして残すための tracker です。  
初版は 2026-03-10 時点の `main` 相当コードベースを対象とし、最終トリアージは 2026-07-04 です。

## Summary

最優先は次の 3 点でした。2026-03-11 時点で P0 build health debt は返済済みです。

1. backend / frontend の中核モジュールが肥大化し、PR の clean reapply と review コストを押し上げている
2. repository / handler / test support / docs の source-of-truth が分散し、変更 1 件あたりの追従コストが高い
3. harness / fixture の fragile point が残っている

## Triage 2026-07-04

前回更新（2026-03-16）以降の最大の環境変化は、2026-06-10 に rebuild mode
（[docs/design-docs/rebuild-architecture.md](../design-docs/rebuild-architecture.md)、
`crates/` + `apps/` workspace、多数の active ExecPlan）が始動したことです。
これを踏まえた再優先順位付けの原則:

- **rebuild が実質的な返済経路になる項目**（#2, #3, #4, #12）は、現行 `backend/` / `frontend/`
  での大規模分割リファクタを凍結し、「これ以上肥大化させない」ガードに切り替える。
  現行側での分割作業は rebuild と semantic conflict を起こすため優先度を P2 に下げる。
- **rebuild 期間中も現行 harness が gate であり続ける項目**（#5）は P1 を維持する。2026-07-04 に本体を返済済み（詳細はセクション #5 参照）。
- **運用実害がある quick win**（#11 runbook 追記、#6 の stale line-count 削除）を先に処理し、いずれも返済済み。

各項目の実測ステータス（詳細は各セクションの Status 参照）:

| # | Debt | Current status |
|---|---|---|
| 1 | P0 Build health | 返済済み（2026-03-11） |
| 2 | Backend god modules | 未着手・悪化（auth.rs 2059 / main.rs 1139 / audit_log.rs 1060 行）→ rebuild へ委譲、P2 降格 |
| 3 | Frontend god modules | 未着手・悪化（client.rs 1121 / holidays.rs 1659 行）→ rebuild へ委譲、P2 降格 |
| 4 | Repository / handler duplication | 未着手 → rebuild の use case 分割 EP 群が返済経路、P2 降格 |
| 5 | Test harness fragility | 返済済み（2026-07-04）。fixture profile / EnvVarGuard / 共有 integration_guard / backend-security-smoke stage を追加。follow-up は残るが P1 as-was の懸念は解消 |
| 6 | Docs source-of-truth drift | 返済済み（plan 配置を統一し、変化しやすい AGENTS.md の line count を削除） |
| 7 | Queue / worker operational debt | 返済済み（2026-07-04）。Recommended Fix 1–3 を T-16（汎用通知サービス基盤）と統合して返済: RUNBOOK 追記・`worker-once` stage 追加・queue メッセージ型の一般化 |
| 8 | Frontend i18n follow-up | 一部返済済み（EP-20260311 の移行・検証は完了。`rust-i18n 4.0.0-preview1` から stable 系への更新だけを独立した残債として維持） |
| 9 | 部署管理 UI 未完成 | 未返済（`#[allow(dead_code)]` 3 メソッド残存） |
| 10 | ユーザー管理 department_id 選択 | 部分解消（招待フォームは実装済み: f3677c6。編集フォームは未実装） |
| 11 | 最上位マネージャー自己申請 pending | 一次返済済み（RUNBOOK 追記・本コミット）。中長期対応（代理承認者/自動エスカレーション）は P2 で残存 |
| 12 | allowed_user_ids 関心漏れ | 未返済 → rebuild の use case 層で解消見込み、rebuild へ委譲 |
| 13 | CreateUser serde 非対称 | 未返済（現状維持を確認） |
| 14 | departments_resource リフレッシュ | 未返済（key は `bool` のまま） |
| 15 | 既存 failing integration test 6 件 | 返済済み。PR #456 後の認可・schema に test を追従し、対象 2 suite は 15 passed / 0 failed（2026-07-30 再実測） |
| 16 | 勤怠修正承認の管理 UI 未配線 | 新規（2026-07-04 登録）。approve/reject が API 直叩きでしか実行できない |

## Priority Queue

| Priority | Debt | Scope | Why now |
|---|---|---|---|
| ~~P1~~ 返済済み | 既存 failing integration test 6 件（#15） | `backend/tests/admin_requests_api.rs`, `backend/tests/user_update_api.rs` | 現行認可・schema に test を追従し、対象 2 suite は 15 passed / 0 failed |
| P2 | 勤怠修正承認の管理 UI 未配線（#16） | `/admin` 画面 + `frontend/src/api/client.rs` | 勤怠修正の承認/却下が API 直叩きでしか実行できず、RUNBOOK が curl 手順に依存している |
| P2 | 最上位マネージャー自己申請 pending 中長期対応（#11 残） | 代理承認者 / system_admin 自動エスカレーション | RUNBOOK 追記（quick win）は返済済み。残るのは仕様検討を要する中長期対応のみ |
| ~~P2~~ 返済済み | Docs source-of-truth drift（#6） | `backend/AGENTS.md`, `frontend/AGENTS.md` | 変化しやすい line count / file size metadata を削除 |
| P2 | ユーザー編集フォームの department 選択（#10 残） | `admin_users/components/detail.rs` | 招待フォーム側は返済済みで、残り半分だけ |
| P2 | 部署管理 UI 未完成（#9） | department 編集 / manager 割当 UI | dead_code 3 件の温床 |
| P2 | Frontend i18n preview 依存（#8 残） | `rust-i18n 4.0.0-preview1` の stable 系への更新 | EP-20260311 の移行・検証は完了済み。依存更新は本体完了と分離して扱う |
| ~~P2~~ 返済済み | Queue / worker operational debt（#7） | RUNBOOK, harness stage | T-16 と統合して 2026-07-04 に返済（RUNBOOK 追記 / `worker-once` stage / queue メッセージ型一般化） |
| P2 | Backend / Frontend god modules（#2, #3） | 現行側は肥大化ガードのみ | 分割は rebuild（`crates/`, `apps/web`）で実現 |
| P2 | Repository / handler duplication（#4）, allowed_user_ids（#12） | rebuild use case 層 | rebuild EP 群が実質的な返済経路 |
| P2 | CreateUser serde 非対称（#13）, departments_resource リフレッシュ（#14） | frontend 小物 | 実害軽微。関連画面を触る PR に同乗させる |
| P2 | Test harness fragility 残 follow-up（#5 残） | `rate_limit_redis_integration.rs` の docker-cli 重複、`admin_holiday_list.rs` の sync guard、~40 ファイルの `profile::db_only()` 統一 | 本体（Fix 1-4）は 2026-07-04 に返済済み。残りは実害軽微な追従作業 |

## Detailed Items

### 1. P0: Build Health Debt

**Status (2026-03-11)**

- 返済済み
- `cargo fmt --all --check` green
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` green
- `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` green
- `cargo clippy --all-targets -- -D warnings` green
- `scripts/harness.sh` に `fmt-check`, `clippy-backend`, `clippy-frontend`, `lint` を追加済み
- [AGENTS.md](../../AGENTS.md) と [docs/manual/HARNESS.md](../manual/HARNESS.md) の受け入れ条件を更新済み

**Symptoms**

- `cargo clippy --all-targets -- -D warnings` を repo 標準として要求しているが、実際には pre-existing violations が残っており、変更ごとの gate として使えない
- `#[allow(dead_code)]` が repository / service / worker 周辺に散在している
- warning が恒常化しているため、新しい負債の混入検知が遅れる

**Evidence**

- [docs/manual/HARNESS.md](../manual/HARNESS.md)
- [backend/src/repositories/attendance_repository.rs](../../backend/src/repositories/attendance_repository.rs)
- [backend/src/repositories/leave_request_repository.rs](../../backend/src/repositories/leave_request_repository.rs)
- [backend/src/repositories/overtime_request_repository.rs](../../backend/src/repositories/overtime_request_repository.rs)
- [backend/src/repositories/repository.rs](../../backend/src/repositories/repository.rs)
- [backend/src/repositories/auth.rs](../../backend/src/repositories/auth.rs)
- [backend/src/services/lockout_notification_queue.rs](../../backend/src/services/lockout_notification_queue.rs)

**Impact**

- harness の `doctor -> unit -> integration -> clippy` で最後の gate が壊れている
- PR ごとに「今回差分ではない失敗」の説明が必要になり、レビューがノイズ化する
- dead code を許容する文化が広がり、境界整理が進みにくい

**Likely Root Cause**

- issue 対応を高速に進める過程で、一時的な `allow` や warning bypass が恒久化した
- 既存違反をまとめて返済する dedicated phase がない

**Recommended Fix**

1. `lint` stage を継続的に PR / ExecPlan の標準 gate として使う
2. 新しい warning 回避のための広域 `allow` 追加をレビューで拒否する
3. 将来の環境差異が出た場合も「lint debt」と「build prerequisite debt」を分けて記録する

---

### 2. P1→P2: Backend God Modules

**Status (2026-07-04)**

- 未着手・悪化。実測: `auth.rs` 1993→2059 行、`main.rs` 992→1139 行、`audit_log.rs` 874→1060 行。`attendance.rs` のみ 869→721 行に改善
- rebuild mode（`crates/app` / `crates/domain` への use case 分割）が実質的な返済経路になったため、現行 `backend/` 側での大規模分割は凍結し、P2 に降格
- 現行側の方針は「これ以上肥大化させない」レビューガードのみ

**Symptoms**

- 認証、監査ログ、勤怠、ルーティングの中核が巨大単位で残っている
- 1 つの修正で unrelated diff が混ざりやすく、stacked PR の clean reapply コストが高い
- 行数が大きく、責務境界より「ファイルの都合」でコードが集約されている

**Evidence**

- [backend/src/handlers/auth.rs](../../backend/src/handlers/auth.rs) `1993` lines
- [backend/src/main.rs](../../backend/src/main.rs) `992` lines
- [backend/src/middleware/audit_log.rs](../../backend/src/middleware/audit_log.rs) `874` lines
- [backend/src/handlers/attendance.rs](../../backend/src/handlers/attendance.rs) `869` lines
- [backend/AGENTS.md](../../backend/AGENTS.md)

**Impact**

- review で「本当に変更したかった責務」が見えにくい
- semantic conflict resolution のたびに current owner を探し直す必要がある
- テスト対象の seam が太く、focused validation でも compile / setup cost が高い

**Likely Root Cause**

- issue 対応を継ぎ足しで進め、抽出の節目を設けていない
- route / audit / auth が cross-cutting concern のため、自然に集中している

**Recommended Fix**

1. `auth.rs` を login / refresh / mfa / password-reset / sessions / profile に分割
2. `audit_log.rs` を `classify`, `metadata`, `serialization`, `path rules` に分割
3. `main.rs` の route 定義を feature module ごとの router builder へ移す
4. `attendance.rs` の export / admin helper / request handling を service/repository 側に抽出する

---

### 3. P1→P2: Frontend God Modules

**Status (2026-07-04)**

- 未着手・悪化。実測: `api/client.rs` 942→1121 行、`holidays.rs` 1574→1659 行
- rebuild target の `apps/web/src/features/<feature>/api.rs` 分割が返済経路。現行側は P2 に降格し、肥大化ガードのみ

**Symptoms**

- API client が auth / attendance / requests / admin を一手に持っている
- 管理画面コンポーネントが巨大で、UI 変更と API 変更が一緒に見えやすい
- current `main` への clean cherry-pick / reapply で unrelated frontend diff が混ざりやすい

**Evidence**

- [frontend/src/api/client.rs](../../frontend/src/api/client.rs) `942` lines
- [frontend/src/pages/admin/components/holidays.rs](../../frontend/src/pages/admin/components/holidays.rs) `1574` lines
- [frontend/AGENTS.md](../../frontend/AGENTS.md)

**Impact**

- frontend PR が「本来 1 画面の変更」でも branch 汚染を起こしやすい
- host test の前提差分が広く、router/context 周りの incidental failure を起こしやすい
- API path / auth redirect / device label など cross-cutting change が `client.rs` へ集中する

**Likely Root Cause**

- MVVM は採用しているが、API client と大型 admin component の分割が追いついていない
- 短期 issue を積み上げる一方で、feature package 化をしていない

**Recommended Fix**

1. `api/client.rs` を domain client へ分割
2. holiday admin UI を filter / list / editor / external sync に分解
3. host test helper を router-aware / auth-aware に共通化する

---

### 4. P1→P2: Repository / Handler Duplication

**Status (2026-07-04)**

- 未着手。rebuild の attendance / correction / request 系 use case EP 群（EP-20260612〜EP-20260613）が実質的な返済経路
- 現行側での service 抽出は rebuild と二重投資になるため P2 に降格

**Symptoms**

- 同じ domain の validation / snapshot / query shape が handler と repository に分散している
- `SELECT ... RETURNING ...` や response conversion の繰り返しが多い
- trait abstraction と関数ベース実装が混在し、repo の標準パターンが揺れている

**Evidence**

- [backend/src/handlers/attendance_correction_requests.rs](../../backend/src/handlers/attendance_correction_requests.rs)
- [backend/src/repositories/attendance_correction_request.rs](../../backend/src/repositories/attendance_correction_request.rs)
- [backend/src/repositories/attendance_repository.rs](../../backend/src/repositories/attendance_repository.rs)
- [backend/src/repositories/leave_request_repository.rs](../../backend/src/repositories/leave_request_repository.rs)
- [backend/src/repositories/overtime_request_repository.rs](../../backend/src/repositories/overtime_request_repository.rs)
- [backend/AGENTS.md](../../backend/AGENTS.md)

**Impact**

- validation change を入れる場所が一意でない
- clippy / dead_code / unused constant が出やすい
- repo 単位で挙動を追いづらく、service 抽出の候補が見えにくい

**Likely Root Cause**

- repo 標準化の途中で issue 対応が先行した
- trait-based pattern と handler-specific helper の境界が曖昧

**Recommended Fix**

1. correction request を service 化し、snapshot build / validation / persistence を 1 箇所へ寄せる
2. repeated SELECT column list を macro or query helper で共有する
3. `RepositoryTrait` を実際に mock されるものへ絞り、 dead trait method を減らす

---

### 5. P1: Test Harness Fragility

**Status (2026-07-04, 返済実施)**

ExecPlan: [EP-20260704-test-harness-fragility](./completed/EP-20260704-test-harness-fragility.md)（完了）

Recommended Fix 1-4 を次のとおり返済した。

1. **Fix 1（fixture profile）— 部分返済**: `backend/tests/support/mod.rs` に `pub mod profile` を新設し、
   `db_only()` / `db_and_redis()` / `db_and_smtp_skip()` / `db_and_smtp_failure()` の named profile を追加した。
   `auth_lockout_redis_integration.rs` の単純な db+redis テスト 4 件と `password_reset_api.rs` の全 7 テストを
   実際にこの profile へ移行した。残る ~40 ファイルは `support::test_pool()` 直接呼び出しのままで、暗黙に
   `db-only` profile に従っている（強制リネームはしていない）。**follow-up**: 残りのファイルを
   `profile::db_only()` へ機械的に統一するかどうかは実害が出た時点で判断する
2. **Fix 2（env mutation helper）— 返済**: `support::EnvVarGuard`（RAII、snapshot/restore 保証）を追加し、
   `auth_lockout_redis_integration.rs` の自前 `EnvGuard`（44 行）と `password_reset_api.rs` の
   `configure_email_skip()`（一度 set したら restore しない直書き `env::set_var`）を置き換えた。
   `support/mod.rs` 内部の `#[cfg(test)] mod tests` も手動 `restore_env` から `EnvVarGuard` へ置換した。
   `ensure_docker_cli()` / `allocate_ephemeral_port()` も `support::` の pub 関数へ統合し、
   `auth_lockout_redis_integration.rs` の重複コピーを削除した。**例外**: `rate_limit_redis_integration.rs` は
   意図的に `mod support;` を追加していない（`support` は `#[ctor]` で Postgres testcontainers を起動するため、
   Postgres を必要としないこのテストに起動コストを強制することになるため）。ここでは重複を残し follow-up 化した
3. **Fix 3（cross-file / cross-invocation 対策）— 返済**: 47 ファイルに重複していた file-local
   `async fn integration_guard()`（ほぼ同一ボイラープレートが 2 variant + 1 個別実装）を
   `support::integration_guard()` への `use support::integration_guard;` 参照に統一した
   （`admin_holiday_list.rs` の独自 sync 版は対象外、follow-up）。
   その上で **実行モデルを実測**した: 2 本の probe test binary（3 秒 sleep）を
   `cargo test -p timekeeper-backend --test A --test B` で実行し、test binary は**逐次実行**され重ならないことを
   確認した。したがって単一の `cargo test --tests` invocation 内では file-local mutex で十分であり、
   実際のリスクは「別々の `cargo test` invocation が `scripts/test_backend_integrated.sh` 経由で同じ共有 Postgres
   （127.0.0.1:55432）を同時に指す」場合に限られる。この cross-invocation 競合に対して、
   `scripts/harness.sh` の `backend-integration` / `backend-security-smoke` に `flock` ベースの
   `BACKEND_INTEGRATION_LOCK`（既定 `target/harness-locks/backend-integration.lock`）を追加し、
   同じロックファイルを共有させることで直列化した。方針は `docs/manual/HARNESS.md` の
   "Suite Execution Model" に明文化した
4. **Fix 4（focused harness stage）— 返済**: `scripts/harness.sh` に `backend-security-smoke` stage を追加した。
   `auth_flow_api`, `auth_lockout_redis_integration`, `rate_limit_redis_integration`, `password_api`,
   `password_reset_api`, `mfa_api`, `session_api`, `active_session_repo` の 8 ファイル・88 test を対象にする。
   `--list` / `docs/manual/HARNESS.md` / ルート `AGENTS.md` の Validation Ladder を整合させた

**実行した検証（実測、2026-07-04）**

- `bash scripts/harness.sh doctor`: pass（podman あり、podman.socket active）
- `cargo fmt --all --check`: pass
- `cargo test -p timekeeper-backend --lib`: pass（379 passed; 0 failed）
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`: pass（0 warnings）
- `bash scripts/harness.sh docs-check`: pass
- `bash scripts/harness.sh backend-security-smoke`: pass（8 ファイル・88 test すべて green。testcontainers 経由で
  Postgres/Redis を実際に起動して実行した実測結果）
- `bash scripts/harness.sh backend-integration`（live Postgres/Redis, 全 test file, `--no-fail-fast`）:
  **1314 passed / 6 failed**。failed 6 件は 2 binary に集中し、いずれも本返済と無関係の既存 failure:
  - `admin_requests_api.rs` 3 件（`test_admin_can_approve_leave_request` 等、403 vs 200/404）。
    本 diff 適用前の HEAD 版ファイルに一時的に戻して再実行しても同一 failure を再現（既存問題と確定）
  - `user_update_api.rs` 3 件（`invalid value \"admin\" for enum UserRole` — テスト自身の seed helper の decode 不具合）。
    このファイルは本返済で一切変更していない（`integration_guard` を持たず移行対象外）
  それ以外の binary（本返済で変更した 48 ファイルを含む）はすべて green

**未着手・follow-up として tracker に残す項目**

- 残り ~40 ファイルの `support::test_pool()` 直呼び出しを `profile::db_only()` へ統一するか（実害なし、優先度低）
- `rate_limit_redis_integration.rs` の docker-cli-ensure 重複解消（`support` の ctor 副作用を避けるため未着手）
- `admin_holiday_list.rs` の独自 sync `integration_guard()` の統合可否検討
- Fix 1 で言及されていた `db+smtp-failure` 実運用テストは `auth_lockout_redis_integration.rs` 内で
  引き続き手動の `EnvVarGuard` 直接操作を使っている（プロファイル化していない）。動的に SMTP 状態を
  トグルする必要があるテストのため、固定形状の `profile::db_and_smtp_failure()` にそのまま当てはめられなかった

**Symptoms**

- integration test が global env mutation と global mutex に依存している
- SMTP / Redis / Postgres の可用性が test ごとに暗黙前提になっている
- local timing acceptance は固定できたが、まだ「なぜ安定するか」が harness profile に組み込まれていない
- file-local な `integration_guard()` では test file をまたぐ DB 競合を防げず、`cargo test` の並列実行で cross-file contention が再発しうる

**Evidence**

- [backend/tests/support/mod.rs](../../backend/tests/support/mod.rs)
- [backend/tests/auth_flow_api.rs](../../backend/tests/auth_flow_api.rs)
- [backend/tests/auth_lockout_redis_integration.rs](../../backend/tests/auth_lockout_redis_integration.rs)
- [scripts/harness.sh](../../scripts/harness.sh)
- [docs/manual/HARNESS.md](../manual/HARNESS.md)

**Impact**

- test の増加に比例して runtime と flake risk が上がる
- env-dependent failure と product regression の切り分けに時間がかかる
- `doctor` が存在しても test fixture 自体の health indicator はまだ弱い

**Likely Root Cause**

- issue 解決ごとに focused test を足してきたが、fixture の集約設計をしていない
- SMTP / Redis worker など background seam が最近増えた

**Recommended Fix**

1. backend integration fixture を `db-only`, `db+redis`, `db+smtp-failure` の profile に分ける
2. env mutation helper を共通 service に寄せ、 direct `set_var/remove_var` を減らす
3. cross-file 競合が出る integration suite には DB 分離、または少なくとも suite-level serial 実行方針を定義する
4. `scripts/harness.sh` に `backend-security-smoke` のような focused stage を追加する

---

### 6. P2→返済済み: Docs Source-Of-Truth Drift

**Status (2026-07-30)**

- 返済済み。exec plan の配置は `docs/exec-plans/{active,completed}` に統一済み
- `backend/AGENTS.md` / `frontend/AGENTS.md` から、変化しやすく陳腐化していた
  line count / file size metadata を削除した

**Symptoms**

- `AGENTS.md` / subdirectory `AGENTS.md` のサイズ・記述が current code とずれている
- file size / line count の記述が stale
- plan 配置の運用が `docs/generated/exec-plans` と今回要求された `docs/exec-plans` に分かれている

**Evidence**

- [frontend/AGENTS.md](../../frontend/AGENTS.md)
  - hotspot は行数ではなく分割方針で記載
- [backend/AGENTS.md](../../backend/AGENTS.md)
  - hotspot は行数ではなく分割方針で記載
- [AGENTS.md](../../AGENTS.md)
- [docs/exec-plans/tech-debt-tracker.md](./tech-debt-tracker.md)

**Impact**

- agent / reviewer が stale metadata を信用して誤った見積もりをしやすい
- docs の配置規約が揺れると、次の文書追加時に迷う

**Recommended Fix**

1. `AGENTS.md` の line-count のような変化しやすい metadata を削除する
2. exec plan の保存先を 1 箇所に統一する
3. tracker / runbook / design-doc の役割を README で明記する

---

### 7. P2→返済済み: Queue / Worker Operational Debt

**Status (2026-07-04, 返済実施)**

ExecPlan: [EP-20260704-notification-service-generalization](./active/EP-20260704-notification-service-generalization.md)（T-16 と統合して実施）

Recommended Fix 1–3 を次のとおり返済した。

1. **Fix 1（worker runbook）— 返済**: `docs/manual/RUNBOOK.md` に "Notification Worker Operations"
   節を追加した。アーキテクチャ概要、Redis key 一覧（queue/retry/DLQ + idempotency marker）、
   retry backoff（2s/4s/8s/16s → DLQ）の実際の計算根拠、worker の起動方法（loop / `--once`）、
   DLQ の手動 replay/破棄手順を明文化した
2. **Fix 2（queue depth 等の観測項目）— 返済**: 同節に `redis-cli LLEN` /
   `redis-cli ZCARD` / `redis-cli ZCOUNT` による queue depth / retry depth / 期限超過 retry 件数 /
   DLQ depth の具体コマンドを記載した
3. **Fix 3（`worker-once` harness stage）— 返済**: `scripts/harness.sh` に `worker-once` stage を
   追加した。`lockout_notification_worker --once` を live Postgres/Redis（testcontainers ではなく
   `DATABASE_URL` / `REDIS_URL` / `JWT_SECRET` で指定された実環境）に対して 1 回実行し、
   起動・終了できることを確認する。`--list` / `docs/manual/HARNESS.md` / ルート `AGENTS.md` の
   Validation Ladder（`backend-security-smoke` の次、item 7）を同期した

これに加えて、tech-debt-tracker には明示されていなかったが親タスク T-16 の指示 1 として、
queue のメッセージ型を `notification_kind` + payload の internally-tagged enum
（`backend/src/services/notification_queue.rs::NotificationJob`）へ一般化した。
既存 lockout 通知の Redis key 名・関数シグネチャ・JSON 直接デコード互換は変更していない
（`notification_kind` フィールドは追加されるが、`serde` は未知フィールドを無視するため
`LockoutNotificationJob` への直接デコードは影響を受けない。unit test で固定済み）。

**実行した検証（実測、2026-07-04）**

- `cargo fmt --all --check`: pass
- `cargo test -p timekeeper-backend --lib`: pass（384 passed; 0 failed。うち 5 件が本タスクで
  追加した `notification_queue` / `lockout_notification_queue` の新規 unit test）
- `cargo test -p timekeeper-backend --test auth_lockout_redis_integration`（live Postgres/Redis,
  testcontainers）: pass（10 passed; 0 failed）
- `cargo test -p timekeeper-backend --test auth_flow_api`（live Postgres, testcontainers）:
  pass（24 passed; 0 failed）
- `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`: pass（0 warnings）
- `bash scripts/harness.sh worker-once`（live Postgres/Redis を podman で用意し、
  `DATABASE_URL` / `REDIS_URL` / `JWT_SECRET` を実際に設定して実行）: pass（exit 0）。
  必須環境変数未設定時に `die` で即座に fail することも確認済み
- `bash scripts/harness.sh docs-check`: pass

**Symptoms（返済前）**

- lockout notification worker は実装されたが、運用境界が doc と harness にまだ十分現れていない
- queue semantics は implicit で、FIFO/LIFO や drain strategy が operator 向けに明文化されていない
- worker binary の health / lag / DLQ monitor が manual 化されていない

**Evidence**

- [backend/src/bin/lockout_notification_worker.rs](../../backend/src/bin/lockout_notification_worker.rs)
- [backend/src/services/lockout_notification_queue.rs](../../backend/src/services/lockout_notification_queue.rs)
- [backend/src/services/lockout_notification_worker.rs](../../backend/src/services/lockout_notification_worker.rs)
- [backend/src/services/notification_queue.rs](../../backend/src/services/notification_queue.rs)（新規、本返済で追加）
- [docs/manual/RUNBOOK.md](../manual/RUNBOOK.md)
- [docs/manual/HARNESS.md](../manual/HARNESS.md)
- [scripts/harness.sh](../../scripts/harness.sh)

**Impact（返済前）**

- 本番障害時に「worker が落ちているのか、queue が溜まっているのか、SMTP が失敗しているのか」を素早く分けにくい
- security feature は実装済みでも運用 readiness がまだ薄い

**Constraints / Non-goals（本返済のスコープ外）**

- T-17（申請提出/承認/却下・打刻漏れ通知の実配線）は行っていない。`notification_queue.rs` は
  拡張点のみを用意した
- rebuild target の `apps/worker` 設計そのものへの反映は別途判断する（現行 `backend/` 側の
  運用ドキュメント・harness stage の整備を先に済ませた）

---

### 8. P2: Frontend I18n Preview Dependency Debt

**Status (2026-07-30)**

- [EP-20260311-frontend-rust-i18n-migration](./completed/EP-20260311-frontend-rust-i18n-migration.md)
  の翻訳移行・テスト・文書同期は完了済み
- 未返済なのは `frontend/Cargo.toml` の `rust-i18n = "4.0.0-preview1"` を stable 系へ
  更新する作業のみ。本体 EP の完了状態とは分離し、依存更新 debt として維持する

**Status (2026-03-12)**

- PR #430 / #431 の merge-blocking review comment は解消済み
- PR #435 では merge-blocking ではない follow-up として、翻訳キー解決を直接検証しない監査ログ event test と、一部 host test / view model test の hardcoded assertion が指摘された
- ただし reviewer が low / optional とした follow-up は別件化前の debt として残す

**Symptoms**

- locale foundation まわりに low-priority の設計・運用 debt が残っている
- frontend と backend の password change error code が二重定義で、将来の drift 耐性が弱い
- host test の一部が翻訳済み日本語テキストに直接依存しており、翻訳変更時に UI 構造と無関係な failure を起こしうる
- `AUDIT_EVENT_TYPES` の test が翻訳キー文字列の非空確認に留まり、`t!()` で実際に locale 解決できるかは保証していない

**Evidence**

- [frontend/Cargo.toml](../../frontend/Cargo.toml)
  - `rust-i18n = "4.0.0-preview1"` のまま
- [frontend/src/state/locale.rs](../../frontend/src/state/locale.rs)
  - `use_locale()` fallback は production でも in-memory fallback を生成する
  - `persist_locale()` failure はコメント付きで無視しているが、UX 上は未通知
- [frontend/src/components/layout.rs](../../frontend/src/components/layout.rs)
  - `LocaleSwitcher` に move closure 制約由来の clone が残っている
- [frontend/src/components/confirm_dialog.rs](../../frontend/src/components/confirm_dialog.rs)
- [frontend/src/components/cards.rs](../../frontend/src/components/cards.rs)
  - host test が翻訳済み文言に直接依存する箇所が残る
- [frontend/src/pages/settings/panel.rs](../../frontend/src/pages/settings/panel.rs)
- [frontend/src/pages/admin_audit_logs/view_model.rs](../../frontend/src/pages/admin_audit_logs/view_model.rs)
  - `audit_event_types_keys_are_unique_and_labels_not_empty` は key string の形だけを見ており、翻訳解決を直接検証していない
- [frontend/src/pages/admin_export/panel.rs](../../frontend/src/pages/admin_export/panel.rs)
  - host test に `Data Export` など hardcoded 文字列 assertion が残っている
- [frontend/src/pages/admin/view_model.rs](../../frontend/src/pages/admin/view_model.rs)
  - validation message の test が翻訳キー経由ではなく最終文字列を直接比較している
- [backend/src/handlers/auth.rs](../../backend/src/handlers/auth.rs)
  - `PASSWORD_CHANGE_*` code を frontend/backend で別管理している

**Impact**

- `rust-i18n` preview 依存が長引くと、将来の stable 移行時にまとめて差分が大きくなる
- locale context の fallback / storage failure が本番で silent degradation として残る
- 翻訳変更だけで host test が落ち、review と reapply のノイズになる
- 監査ログ event key の typo や locale file 側の欠落が、現在の test だと未検知で混入しうる
- error code の片側変更時に frontend/backend の対応がずれる余地がある

**Recommended Fix**

1. `rust-i18n` stable 系への移行計画を別 issue 化し、`Cargo.lock` ごと更新する
2. `use_locale()` fallback の production diagnostics 方針を決める
3. `persist_locale()` failure を UX / telemetry のどちらで拾うか決める
4. `LocaleSwitcher` の clone 群は意図が明確な小さな構造へ寄せる
5. text 直書き assertion が必要な host test と、DOM 構造 assertion で十分な host test を整理する
6. password change error code は共有定数または schema 生成で一元化する
7. `AUDIT_EVENT_TYPES` については、`en` / `ja` の両 locale で `rust_i18n::t!(*key) != key` を確認する translation-resolution test を追加する
8. review comment で指摘された hardcoded assertion は `rust_i18n::t!(...)` ベースへ寄せ、翻訳文面変更と UI 回帰を切り分けやすくする

---

### 9. P2: 部署管理 UI の未完成（PR #456 由来）

**発生時期:** 2026-03-15（部署階層 & マネージャー承認 PR #456）

**Status (2026-07-04)**

- 未返済。`admin_update_department` / `admin_assign_manager` / `admin_remove_manager` は依然 `#[allow(dead_code)]` 付きで `frontend/src/api/client.rs` に残存

**Symptoms**

- `admin_update_department`、`admin_assign_manager`、`admin_remove_manager` の 3 メソッドが `frontend/src/api/client.rs` に `#[allow(dead_code)]` で存在する
- 対応する `UpdateDepartmentRequest`、`AssignManagerRequest` 型も同様
- バックエンド API は全エンドポイント実装済みだが、フロントエンド UI は一覧・作成・削除のみ
- 管理画面から部署名の変更やマネージャーの割り当て・解除が操作できない

**Evidence**

- [frontend/src/api/client.rs](../../frontend/src/api/client.rs) — `admin_update_department`, `admin_assign_manager`, `admin_remove_manager`
- [frontend/src/api/types.rs](../../frontend/src/api/types.rs) — `UpdateDepartmentRequest`, `AssignManagerRequest`, `DepartmentManagerEntry`
- [docs/design-docs/department-hierarchy.md](../design-docs/department-hierarchy.md)

**Impact**

- system_admin が部署名の変更やマネージャー割り当てを行うには API を直接叩く必要がある
- `#[allow(dead_code)]` が放置されると新規の dead_code 混入を見逃しやすくなる

**Recommended Fix**

1. `/admin/departments/:id` に部署編集フォーム（名前変更・親部署変更）を実装
2. 部署詳細ページにマネージャー割り当て/解除 UI を実装
3. `#[allow(dead_code)]` を除去

---

### 10. P2: ユーザー管理 UI に department_id 選択が未実装（PR #456 由来）

**発生時期:** 2026-03-15（部署階層 & マネージャー承認 PR #456）

**Status (2026-07-04)**

- 部分解消。招待フォームは `AdminDepartmentSelect` で部署選択を実装済み（commit f3677c6、EP-20260316-frontend-invite-department）
- 未解消: ユーザー編集フォーム（`admin_users/components/detail.rs`）は `department_id: None` 固定のままで、既存ユーザーの部署変更は依然 UI からできない

**Symptoms**

- バックエンドの `create_user` / `update_user` は `department_id` を受け入れるが、
  フロントエンドの招待フォーム・ユーザー編集フォームに部署選択ドロップダウンが存在しない
- 従業員の部署所属を変更するには API を直接叩く必要がある

**Evidence**

- [frontend/src/pages/admin_users/components/invite_form.rs](../../frontend/src/pages/admin_users/components/invite_form.rs)
- [backend/src/handlers/admin/users.rs](../../backend/src/handlers/admin/users.rs)

**Impact**

- 部署階層の実用性が制限される。管理者が UI から従業員を部署に配属できない
- 「承認スコープが効かない」というバグ報告が来た際に原因を特定しにくい（所属が設定されていないだけ）

**Recommended Fix**

1. 招待フォームに部署選択ドロップダウンを追加（`admin_list_departments` を利用）
2. ユーザー編集フォームにも同様に追加

---

### 11. P1: 最上位マネージャーの自己申請が永久 pending（PR #456 由来）

**発生時期:** 2026-03-15（部署階層 & マネージャー承認 PR #456）

**Status (2026-07-04)**

- 一次返済済み（本コミットで返済）。Recommended Fix 1（RUNBOOK 追記）を実施した
- `docs/manual/RUNBOOK.md` に「Top-Level Manager Self-Approval (Requests Stuck in `pending`)」節を追加。
  実装を確認した上で、system_admin が `/admin`（申請承認ページ、`AdminRequestsSection`）から
  `GET /api/admin/requests` でユーザー/ステータス絞り込みを行い、
  `PUT /api/admin/requests/{id}/approve` または `/reject` で承認・却下する具体手順と、
  system_admin の request 一覧は `allowed_user_ids` によるスコープ制限を受けない（`list_requests`）ため
  最上位マネージャー自身の申請も見える、という実装上の根拠を明記した
- 勤怠修正申請（`/api/admin/attendance-corrections/{id}/approve|reject`）にも同様の
  `is_system_admin` 救済経路があるが、対応する admin UI セクションが未配線であるため、
  現状は API 直接呼び出しでの対応が必要である旨も runbook に注記した
- Recommended Fix 2（代理承認者の指定 / system_admin への自動エスカレーション）は未着手のため **P2 に降格**して残す。
  Priority Queue と Suggested Execution Order を実測に合わせて更新済み

**Symptoms**

- 最上位部署（`parent_id = NULL`）のマネージャーが有給・残業等を申請すると、
  承認できる上位マネージャーが存在しないため申請が `pending` のまま残る
- `is_system_admin` が手動で承認する運用が必要だったが、その手順は runbook 未記載だった（本コミットで追記済み）

**Evidence**

- [docs/design-docs/department-hierarchy.md](../design-docs/department-hierarchy.md) — 「既知の制約」セクション
- [backend/src/handlers/admin/common.rs](../../backend/src/handlers/admin/common.rs) — `check_approval_authorization`

**Impact**

- 経営層・部門長など上位マネージャーの申請がシステム上処理できず、手動フローへの依存が生まれる
- 申請が溜まると運用負債になる

**Recommended Fix**

1. ~~`docs/manual/RUNBOOK.md` に「最上位マネージャーの申請承認手順」を追記~~（完了・本コミット）
2. （未着手・P2）中長期的には「代理承認者の指定」または「system_admin への自動エスカレーション」を検討

---

### 12. P2: `allowed_user_ids` によるハンドラ → リポジトリへの関心漏れ（PR #456 由来）

**発生時期:** 2026-03-15（部署階層 & マネージャー承認 PR #456）

**Status (2026-07-04)**

- 未返済。`allowed_user_ids` は `repositories/request.rs` と `handlers/admin/requests.rs` に残存
- rebuild の use case 層（`crates/app`）で認可スコープ組み立てが handler から分離される見込みのため、現行側での service 層新設はせず rebuild へ委譲

**Symptoms**

- `RequestListFilters.allowed_user_ids` と `AttendanceCorrectionRequestRepository::list_paginated` の `allowed_user_ids` パラメータは、「誰がアクセスしているか」というハンドラ層の関心事をリポジトリ層に注入している
- リポジトリが認可スコープを知ってしまうため、将来のリポジトリ単体テストで認可状態のモックが必要になる

**Evidence**

- [backend/src/repositories/request.rs](../../backend/src/repositories/request.rs) — `RequestListFilters`
- [backend/src/repositories/attendance_correction_request.rs](../../backend/src/repositories/attendance_correction_request.rs) — `list_paginated`
- [backend/src/handlers/admin/requests.rs](../../backend/src/handlers/admin/requests.rs)

**Impact**

- リポジトリと認可ロジックの境界が曖昧になる
- サービス層が存在しないため、ハンドラが直接リポジトリの認可スコープを組み立てている
- 「#3 P1: Repository / Handler Duplication」の症状と連動する

**Recommended Fix**

1. サービス層（`services/request_service.rs` など）を設け、認可スコープの組み立てをハンドラから移動する
2. `RequestListFilters` から `allowed_user_ids` を除去し、サービス層で SQL を構築するか、専用クエリメソッドを用意する

---

### 13. P2: `CreateUser` の serde 属性非対称性（招待フォーム部署選択追加由来）

**発生時期:** 2026-03-16（EP-20260316-frontend-invite-department）

**Status (2026-07-04)**

- 未返済。`is_system_admin` は `#[serde(default)]`、`department_id` は `skip_serializing_if` のまま非対称。実害は未発生のため、`types.rs` を触る PR に同乗させる

**Symptoms**

- `CreateUser` struct 内で `is_system_admin` と `department_id` の serde シリアライズ戦略が異なる
  - `is_system_admin`: `#[serde(default)]` — `false` でも JSON に含まれる (`"is_system_admin": false`)
  - `department_id`: `#[serde(skip_serializing_if = "Option::is_none")]` — `None` の場合 JSON から省略される
- 同一 struct 内でフィールドによってシリアライズ挙動が異なることは、将来の API 呼び出しデバッグを難しくする

**Evidence**

- [frontend/src/api/types.rs](../../frontend/src/api/types.rs) — `CreateUser` struct

**Impact**

- 機能上のバグは現時点ではないが、backend が `serde(default)` を持たないフィールドを追加した場合に差異が顕在化する
- struct の serde 戦略を把握するためにフィールドごとに属性を追跡する必要がある

**Recommended Fix**

1. `CreateUser` の `is_system_admin` に `#[serde(skip_serializing_if)]` を追加して `department_id` と統一する、または
2. 方針を「必須フィールドは常に送る、オプショナルフィールドは None 省略」として `CODING_AGENT.md` に明記する

---

### 14. P2: `departments_resource` に手動リフレッシュ機構がない（招待フォーム部署選択追加由来）

**発生時期:** 2026-03-16（EP-20260316-frontend-invite-department）

**Status (2026-07-04)**

- 未返済。`departments_resource` の key は `bool` 単体のまま。#9 / #10 の UI 実装時に同乗させる

**Symptoms**

- `departments_resource` のキーが `bool` 単体のため、他リソースのような手動リフレッシュ（タプルの第2要素を変化させる）ができない
- 他のリソース (`users_resource`, `archived_users_resource`) は `(is_system_admin.get(), 0u32)` で手動 refetch トリガーを持つ

**Evidence**

- [frontend/src/pages/admin_users/view_model.rs](../../frontend/src/pages/admin_users/view_model.rs) — `departments_resource` vs `users_resource`

**Impact**

- 別タブで部署を追加・削除した後に招待フォームへ戻っても部署一覧がキャッシュされたまま
- 現状の招待フォームユースケースでは操作頻度が低く実害は軽微だが、部署管理 UI が拡充されると問題が顕在化しやすい

**Recommended Fix**

1. `departments_resource` のキーを `(is_system_admin.get(), 0u32)` 形式へ変更し、他リソースと統一する
2. ViewModel に `reload_departments()` 相当のメソッドを追加することを検討する

---

### 15. P1→返済済み: 既存 failing integration test 6 件（PR #456 への test 未追従）

**発生時期:** 発見は 2026-07-04（item #5 返済の検証中）。混入は PR #456（部署階層 & マネージャー承認）と推定

**Status (2026-07-30)**

- 返済済み。`admin_requests_api.rs` は現行の部署スコープ認可へ、
  `user_update_api.rs` は現行 schema / 共有 fixture へ追従済み
- `cargo test -p timekeeper-backend --test admin_requests_api --test user_update_api -- --nocapture`:
  15 passed / 0 failed（2026-07-30 再実測）
- 以下の Symptoms / Root Cause は返済前の記録として保持する

**Symptoms**

- `admin_requests_api.rs` の 3 件が 403 で fail:
  - `test_admin_can_approve_leave_request`（403 vs 期待 200）
  - `test_admin_can_reject_leave_request`（403 vs 期待 200）
  - `test_approve_already_processed_request_fails`（403 vs 期待 404）
- `user_update_api.rs` の 3 件が seed helper 内で panic:
  - `test_admin_update_user_email` — `invalid value "admin" for enum UserRole`
  - `test_email_uniqueness_check` — `ColumnNotFound("department_id")`
  - `test_user_update_own_profile` — `ColumnNotFound("department_id")`

**Evidence**

- [backend/tests/admin_requests_api.rs](../../backend/tests/admin_requests_api.rs) — 承認者を `seed_user(&pool, UserRole::Manager, false)`（`is_system_admin = false`、申請者の部署と無関係）で作成している
- [backend/tests/user_update_api.rs](../../backend/tests/user_update_api.rs) — file 独自の seed helper（151 行目付近）が現行スキーマとずれている
- [backend/src/handlers/admin/common.rs](../../backend/src/handlers/admin/common.rs) — `check_approval_authorization`

**Likely Root Cause**

- PR #456 で承認認可が「system_admin または申請者部署チェーンの manager」に厳格化されたが、`admin_requests_api.rs` は旧モデル（Manager role なら誰でも承認可）の前提のまま。部署スコープ外の Manager が 403 になるのは現行仕様どおり
- 同 PR で `users` テーブルに `department_id` が追加されたが、`user_update_api.rs` の file 独自 seed helper は共有 `support::seed_user` を使わず生 SQL のままで、スキーマ変更に追従していない

**Impact**

- `backend-integration` が gate として機能しない。「今回差分による失敗」と「既知の失敗」の切り分け説明が PR ごとに必要になる
- item #1（Build Health）で返済したはずの「red が恒常化して新規負債の混入検知が遅れる」状態が integration 層で再発している

**Recommended Fix**

1. `admin_requests_api.rs` の 3 test を現行認可モデルに追従させる（承認者を system_admin にするか、申請者部署の manager として配属したうえで期待値を検証する。スコープ外 Manager が 403 になるケースは別 test として明示的に固定する）
2. `user_update_api.rs` の file 独自 seed helper を廃止し、共有 `support` の seed に統一する（`department_id` / `UserRole` の decode を現行スキーマに合わせる）
3. 返済後、`bash scripts/harness.sh backend-integration` が 0 failed であることを確認し、以降は red を「既知」として放置しない

---

### 16. P2: 勤怠修正承認の管理 UI 未配線（API 直叩きのみ）

**発生時期:** 発見は 2026-07-04（item #11 返済の実装確認中）

**Status (2026-07-04)**

- 新規登録。backend の承認/却下エンドポイントは実装済みだが、それを呼ぶ管理画面 UI が存在しない

**Symptoms**

- `/api/admin/attendance-corrections/{id}/approve` / `/reject` は `ApproveCorrectionUseCase` / `RejectCorrectionUseCase` 経由で実装済み（system_admin オーバーライド含む）
- しかし `/admin` 画面のどのセクションからも勤怠修正申請の一覧・承認・却下に到達できない（route 未配線）
- 有給・残業申請（`AdminRequestsSection`）には UI があるため、申請種別によって運用手順が非対称

**Evidence**

- [backend/src/handlers/admin/attendance_correction_requests.rs](../../backend/src/handlers/admin/attendance_correction_requests.rs)
- [frontend/src/pages/admin/panel.rs](../../frontend/src/pages/admin/panel.rs) — 勤怠修正セクションなし
- [docs/manual/RUNBOOK.md](../manual/RUNBOOK.md) — "Top-Level Manager Self-Approval" の Notes が「API を直接呼ぶ」手順に依存している

**Impact**

- 勤怠修正申請の承認運用が API 直叩き（認証済み session での curl 等）に依存し、操作ミス・監査可能性の面で弱い
- item #9（部署管理 UI 未完成）と同型の「backend 先行・frontend 未配線」debt で、放置パターンが定着しつつある

**Recommended Fix**

1. `/admin` 画面に勤怠修正申請の承認セクション（一覧 + 承認/却下）を追加する（`AdminRequestsSection` の構成を踏襲）
2. `frontend/src/api/client.rs` に対応する client メソッドを追加する（rebuild 進行中のため、現行側は最小限の配線に留める）
3. UI 配線後、RUNBOOK の該当 Notes を UI 手順へ書き換える

---

## Suggested Execution Order

2026-07-04 トリアージ後の実行順:

1. ~~P0 Build health debt~~（返済済み 2026-03-11）
2. ~~P1 最上位マネージャーの自己申請 pending（#11）— RUNBOOK 追記~~（一次返済済み・本コミット。中長期対応は P2 #11 残 へ降格）
3. ~~P1 Test harness fragility（#5）~~（返済済み 2026-07-04。fixture profile / EnvVarGuard / 共有 integration_guard / backend-security-smoke stage / suite execution model を追加。follow-up は P2 #5 残 へ）
4. ~~P1 既存 failing integration test 6 件（#15）~~（返済済み。対象 2 suite は 15 passed / 0 failed）
5. P2 最上位マネージャー自己申請 pending 中長期対応（#11 残）— 代理承認者 / 自動エスカレーション検討
6. ~~P2 Docs source-of-truth drift 残作業（#6）~~（返済済み。AGENTS.md の stale line-count を削除）
7. P2 ユーザー編集フォームの department 選択（#10 残り半分）
8. P2 部署管理 UI 未完成（#9）
9. P2 勤怠修正承認の管理 UI 未配線（#16）— #9 と同型のため合わせて計画してよい
10. P2 Frontend i18n preview 依存（#8 残）— 完了済み EP-20260311 とは分離し、stable 系への依存更新を計画
11. ~~P2 Queue / worker operational debt（#7）~~（返済済み 2026-07-04。T-16 と統合。RUNBOOK 追記 / `worker-once` stage / queue メッセージ型一般化）
12. P2 Backend / Frontend god modules（#2, #3）— rebuild へ委譲、現行側は肥大化ガードのみ
13. P2 Repository / handler duplication（#4）+ allowed_user_ids 関心漏れ（#12）— rebuild use case EP 群へ委譲
14. P2 CreateUser serde 非対称（#13）+ departments_resource リフレッシュ（#14）— 関連 PR に同乗
15. P2 Test harness fragility 残 follow-up（#5 残）— docker-cli 重複 / sync guard 統合 / profile 統一の残作業

## Notes

- この tracker は「今すぐ全部直す」ためではなく、issue 化・分割計画の起点として使う
- まずは P0 を返済しないと harness gate の信頼性が上がらない
- **2026-03-15 追加**: PR #456（部署階層 & マネージャー承認）により items 9–12 を追加
- **2026-03-16 追加**: EP-20260316-frontend-invite-department レビューにより items 13–14 を追加
- **2026-07-04 トリアージ**: rebuild mode 始動（2026-06-10）を反映。#2/#3/#4/#12 は rebuild へ委譲して P2 降格、#10 は招待フォーム側を返済済みに更新、god module の実測行数を更新
- **2026-07-04 追加返済**: #11 の Recommended Fix 1（RUNBOOK 追記）を実施し一次返済。承認フロー（`/admin` 画面、`GET/PUT /api/admin/requests...`）を実装で確認した上で `docs/manual/RUNBOOK.md` に手順を追記。Recommended Fix 2（代理承認者 / 自動エスカレーション）は未着手のため P2 として残存
- **2026-07-04 追加返済**: #5「Test Harness Fragility」の Recommended Fix 1-4 を返済（[EP-20260704-test-harness-fragility](./completed/EP-20260704-test-harness-fragility.md)）。
  `backend/tests/support/mod.rs` に `EnvVarGuard` / `integration_guard()` / `profile::{db_only, db_and_redis, db_and_smtp_skip, db_and_smtp_failure}` を追加し、
  47 ファイルに重複していた file-local `integration_guard()` を共有関数へ統一、`scripts/harness.sh` に `backend-security-smoke` stage と
  cross-invocation 用 `flock` lock（`BACKEND_INTEGRATION_LOCK`）を追加した。実行モデルの実測（test binary は逐次実行される）を
  `docs/manual/HARNESS.md` の "Suite Execution Model" に明文化。検証中、`admin_requests_api.rs` の 3 test（`test_admin_can_approve_leave_request` 等、
  本 diff 適用前の同ファイルでも再現）と `user_update_api.rs` の 3 test（`UserRole` enum decode 不具合。本返済では未変更のファイル）が
  本タスクと無関係に `main` 相当のコードで既に failing であることを確認した。これら既知の failing test は
  本 tracker のスコープ外（承認認可ロジック / テスト seed helper の不具合であり、item #5 の対象である harness/fixture 側の問題ではない）のため、
  item #15 として登録済み
- **2026-07-04 追加**: items 15–16 を登録。#15 は #5 返済検証中に確定した既存 failing integration test 6 件
  （単体再実行で原因診断済み — 6 件とも製品バグではなく PR #456 への test 未追従）。
  #16 は #11 返済中に発見された勤怠修正承認の管理 UI 未配線（approve/reject が API 直叩きのみ）
- **2026-07-04 追加返済**: #7「Queue / Worker Operational Debt」の Recommended Fix 1–3 を、
  `docs/exec-plans/attendance-domain-gap-tasks.md` T-16（汎用通知サービス基盤）と統合して返済
  （[EP-20260704-notification-service-generalization](./active/EP-20260704-notification-service-generalization.md)）。
  `docs/manual/RUNBOOK.md` に "Notification Worker Operations" 節（queue/retry/DLQ 観測コマンド、
  drain/replay 手順）を追加し、`scripts/harness.sh` に `worker-once` stage（live Postgres/Redis に
  対する `lockout_notification_worker --once` smoke）を追加した。加えて T-16 の指示に基づき、
  queue のメッセージ型を `notification_kind` + payload の internally-tagged enum
  （`backend/src/services/notification_queue.rs`）へ一般化し、既存 lockout 通知の Redis key・
  関数シグネチャ・JSON 直接デコード互換（`LockoutNotificationJob` への直接デシリアライズ）を
  維持したまま T-17（申請提出/承認・却下・打刻漏れ通知）が同じ queue 基盤に乗れる拡張点を用意した
