# EP-20260704-test-harness-fragility

## Goal

tech-debt-tracker.md item #5「P1: Test Harness Fragility」の Recommended Fix 1-4 を返済する。

1. backend integration fixture を `db-only` / `db+redis` / `db+smtp-*` の named profile として `backend/tests/support/` に集約する
2. env mutation (`env::set_var` / `env::remove_var`) を共通 RAII ヘルパーへ寄せ、直接呼び出しを減らす
3. `integration_guard()` の file-local 実装を `support::` の共有関数へ集約し、cross-file / cross-invocation の DB 競合に対する実効性のある方針を定義・明文化する
4. `scripts/harness.sh` に `backend-security-smoke` focused stage を追加する

## Scope

- In: `backend/tests/**`, `scripts/harness.sh`, `docs/manual/HARNESS.md`, `docs/exec-plans/tech-debt-tracker.md`
- Out: `backend/src/**` の本体実装変更（今回は不要だった）、test の削除・弱体化、rebuild target（`crates/`, `apps/`）側のテスト基盤

## Current Baseline (Measured 2026-07-04)

- [x] `bash scripts/harness.sh doctor` — pass（`podman` あり、`docker` コマンドなし、Redis/SMTP のローカル常駐サービスなし。podman.socket は active）
- 事前調査:
  - `env::set_var` / `env::remove_var` 直接呼び出し: `backend/tests/support/mod.rs` 12 箇所、`backend/tests/auth_lockout_redis_integration.rs` 7 箇所、`backend/tests/rate_limit_redis_integration.rs` 3 箇所、`backend/tests/password_reset_api.rs` 1 箇所
  - file-local `async fn integration_guard()` 定義: `backend/tests/*.rs` に **47 ファイル**（ほぼ同一ボイラープレートが 2 variant + 1 個別実装で重複）
  - `backend/tests/admin_holiday_list.rs` は非同期ではない別系統の sync `integration_guard()`（`#[cfg(feature = "test-utils")]`）を持ち、今回の集約対象から除外
  - 実行モデルの実測: 2 つの probe test binary（3 秒 sleep）を `cargo test -p timekeeper-backend --test A --test B` で実行し、タイムスタンプ出力から **test binary は逐次実行される**（同時に走らない）ことを確認済み。したがって単一の `cargo test --tests` 呼び出し内では cross-file の TRUNCATE 競合は原理的に起きない
  - 実際の競合ウィンドウは「同一 invocation 内の並列」ではなく「別々の `cargo test` 呼び出しが同じ共有 Postgres を指す」場合。`scripts/test_backend_integrated.sh` は `docker-compose.test-db.yml`（127.0.0.1:55432 の固定 Postgres）を `TEST_DATABASE_URL` として全 test binary に共有させる高速経路であり、これを 2 つの端末/agent セッションで同時に起動すると file-local mutex では防げない cross-process 競合が起こり得る

## Done Criteria (Observable)

- [x] `backend/tests/support/mod.rs` に `pub async fn integration_guard()`、`pub struct EnvVarGuard`、`pub mod profile { db_only, db_and_redis, db_and_smtp_skip, db_and_smtp_failure }` を追加
- [x] 47 ファイルの file-local `integration_guard()` を `support::integration_guard` の re-use（`use support::integration_guard;` または明示的な薄い委譲）へ置換
- [x] `auth_lockout_redis_integration.rs` の自前 `EnvGuard` / `ensure_docker_cli` / `allocate_ephemeral_port` を `support::` 版へ置換し、単純な db+redis テスト 4 件を `support::profile::db_and_redis()` へ移行
- [x] `password_reset_api.rs` の `configure_email_skip()`（無制限 `env::set_var`、restore なし）を `support::profile::db_and_smtp_skip()` へ置換
- [x] `backend/tests/support/mod.rs` 内部の `#[cfg(test)] mod tests` の手動 `restore_env` を `EnvVarGuard` へ置換
- [x] `scripts/harness.sh` に `backend-security-smoke` stage を追加（auth/lockout/rate-limit/password/mfa/session 系 8 ファイルの focused `cargo test`）
- [x] `scripts/harness.sh` に cross-invocation 用 `flock` ベースの suite-level serial execution policy（`BACKEND_INTEGRATION_LOCK`）を追加し、`backend-integration` と `backend-security-smoke` の両方に適用
- [x] `docs/manual/HARNESS.md` に "Suite Execution Model" を追記し、cargo test の逐次実行の実測結果と cross-invocation リスクの説明・運用方針を明文化
- [x] `cargo fmt --all --check` green
- [x] `cargo test -p timekeeper-backend --lib` green（379 passed; 0 failed）
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` green（0 warnings）
- [x] `bash scripts/harness.sh docs-check` green
- [x] `bash scripts/harness.sh backend-security-smoke`（live Postgres/Redis, testcontainers）: green（8 binary・88 test すべて pass）
- [x] `bash scripts/harness.sh backend-integration`（live Postgres/Redis, 全 test file, `--no-fail-fast`）: 1314 passed / 6 failed。failed 6 件はすべて既存問題（`admin_requests_api.rs` 3 件は本 diff 適用前の HEAD 版ファイルでも再現、`user_update_api.rs` 3 件は本返済で未変更のファイルの seed helper 不具合）。本返済で変更した 48 ファイルを含む残りの binary はすべて green

## Constraints / Non-goals

- 既存 test の挙動・カバレッジを弱めない。test 削除で green にしない
- rebuild target（`crates/`, `apps/`）側のテスト基盤には触れない
- 全 47 ファイルの fixture 呼び出しを `profile::` ヘルパーへ強制的に書き換えることはしない。代表ファイル（`auth_lockout_redis_integration.rs`, `password_reset_api.rs`）のみ移行し、残りは `support::test_pool()` を暗黙の `db-only` profile として扱う
- `rate_limit_redis_integration.rs` は意図的に `mod support;` を追加しない（`support` は `#[ctor]` で Postgres testcontainers を起動するため、Postgres 不要なこのテストに Postgres 起動コストを強制することになる）。ここでは `ensure_docker_cli` / `allocate_ephemeral_port` の重複を許容し、follow-up として記録する

## Task Breakdown

1. [x] 実行モデルの実測（cargo test の逐次/並列実行）
2. [x] `support/mod.rs` に `integration_guard` / `EnvVarGuard` / `profile` モジュールを追加
3. [x] 47 ファイルの file-local `integration_guard()` を機械的に除去し `support::integration_guard` を re-use
4. [x] `auth_lockout_redis_integration.rs` / `password_reset_api.rs` を代表として `profile::` ヘルパーへ移行
5. [x] `cargo check --tests` / `cargo fix --tests --allow-dirty` で unused import を解消
6. [x] `scripts/harness.sh` に `backend-security-smoke` + `flock` ベースの suite lock を追加
7. [x] `docs/manual/HARNESS.md` に Suite Execution Model を追記
8. [x] `fmt-check` / `backend-unit` / `clippy-backend` / `docs-check` を実行
9. [x] `backend-integration` / `backend-security-smoke`（live DB, testcontainers 経由）を実行し結果を記録
10. [x] tech-debt-tracker.md item #5 の Status を更新し、triage 表 / Priority Queue / Suggested Execution Order を整合

## Sequencing

1. 実行モデルの実測（設計判断の前提を固定する）
2. `support/mod.rs` への集約実装
3. 47 ファイルの機械的移行 + 代表ファイルの profile 移行
4. `cargo check` / `cargo fix` / `cargo fmt` による静的検証
5. harness stage 追加 + docs 反映
6. 可能な範囲で live DB 検証
7. tracker 更新 + commit

## Status (2026-07-04) — COMPLETED

全 Task Breakdown 完了。実測結果は本 EP の Done Criteria チェックボックスと
`docs/exec-plans/tech-debt-tracker.md` item #5 の Status セクションを参照。

- 変更規模: backend/tests 48 ファイル（285 insertions / 566 deletions）、`scripts/harness.sh`、docs 4 ファイル
- 検証はすべて実測（testcontainers による実 Postgres/Redis 起動を含む）
- 検証中に発見した本タスク無関係の既存 failing test（`admin_requests_api.rs` 3 件、`user_update_api.rs` 3 件）は
  tracker の Notes に記録済み。次回トリアージで item 化する
- follow-up（`rate_limit_redis_integration.rs` の docker-cli 重複、`admin_holiday_list.rs` の sync guard、
  ~40 ファイルの `profile::db_only()` 統一、SMTP 動的トグルテストのプロファイル化）は tracker の
  「P2 Test harness fragility 残 follow-up（#5 残）」として記録済み
