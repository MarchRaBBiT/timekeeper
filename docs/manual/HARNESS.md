# Harness Manual

## Goal

この repo のハーネスは、agent が「何を変えたか」ではなく「どの stage を通したか」で完了を判断するための実行面です。
入口は `scripts/harness.sh` に統一します。

## Principles

- 1 つの共通入口を使う
- 最小 stage から順に上げる
- live 環境前提の検証と local test を分ける
- issue / PR には stage 名と結果を書く

## Stages

### `doctor`

ローカル前提の確認です。

- `bash`
- `cargo`
- `node`
- `python3` or `python`
- `curl`

また、live smoke を回す場合に使う URL も表示します。

### `docs-check`

ハーネスと再構築 architecture の source of truth が揃っているかを確認します。

確認内容:

- root `AGENTS.md`
- `docs/manual/CODING_AGENT.md`
- `docs/manual/HARNESS.md`
- `docs/design-docs/harness-engineering.md`
- `docs/design-docs/rebuild-architecture.md`
- `docs/exec-plans/active/EP-20260610-rebuild-architecture-harness.md`
- `.agent/PLANS.md`
- 安定ハーネス文書に旧 VCS 前提が残っていないこと

```bash
bash scripts/harness.sh docs-check
```

### `fmt-check`

```bash
cargo fmt --all --check
```

### `backend-unit`

```bash
cargo test -p timekeeper-backend --lib
```

### `backend-integration`

```bash
cargo test -p timekeeper-backend --tests --no-fail-fast
```

内部では `scripts/harness.sh` が `BACKEND_INTEGRATION_LOCK`（既定: `target/harness-locks/backend-integration.lock`）を `flock` で取得してから実行します。理由は "Suite Execution Model" を参照してください。`--no-fail-fast` を付けているのは、この stage が ~50 の独立した test binary を束ねているため、1 binary の failure で以降の binary が丸ごとスキップされると harness の signal が失われるためです（1 binary の failure でも stage 全体としては exit code が非 0 になります）。

### `backend-security-smoke`

```bash
bash scripts/harness.sh backend-security-smoke
```

auth / lockout / rate-limit / password / mfa / session まわりの focused integration test だけを実行します（`scripts/harness.sh` の `BACKEND_SECURITY_SMOKE_TESTS` 参照: `auth_flow_api`, `auth_lockout_redis_integration`, `rate_limit_redis_integration`, `password_api`, `password_reset_api`, `mfa_api`, `session_api`, `active_session_repo`）。`backend-integration` full suite より短時間で認証/セキュリティ系の regression を検知したい場合に使います。`backend-integration` と同じ `BACKEND_INTEGRATION_LOCK` を使うため、両者を同時に走らせても cross-invocation の DB 競合は起きません。

### `clippy-backend`

```bash
cargo clippy -p timekeeper-backend --all-targets -- -D warnings
```

実行前に `cargo clean -p utoipa-swagger-ui` を挟み、repo 移動後の stale build artifact で workspace lint が壊れないようにします。

### `clippy-frontend`

```bash
cargo clippy -p timekeeper-frontend --all-targets -- -D warnings
```

こちらも同じく `utoipa-swagger-ui` の stale artifact を先に掃除します。

### `api-smoke`

```bash
BACKEND_BASE_URL=http://localhost:3000 bash scripts/harness.sh api-smoke
```

内部では先に `GET /api/config/timezone` で live backend の疎通を確認し、その後 [scripts/test_backend.sh](../../scripts/test_backend.sh) を実行します。`/health` や `/api/health` は前提にしません。

### `frontend-login`

```bash
FRONTEND_BASE_URL=https://localhost:8080 bash scripts/harness.sh frontend-login
```

内部では [scripts/test_frontend_login.mjs](../../scripts/test_frontend_login.mjs) を使います。

## Suite Execution Model

`backend/tests/*.rs` の integration test は 1 ファイル = 1 バイナリで compile されます。この節は
「どこまでが安全に並列で、どこからが手動で気をつけるべきか」を実測ベースで明文化します
（tech-debt-tracker.md item #5「Test Harness Fragility」参照）。

- **同一 `cargo test` invocation 内は逐次実行**: `cargo test -p timekeeper-backend --tests`
  （または `--test A --test B` のように複数 test target を指定した単一呼び出し）は、各 test
  binary を **1 つずつ順番に** 実行します。2 本の probe test binary（それぞれ 3 秒 sleep）で
  実測したところ、2 本目は 1 本目が完全に終了してから開始しており、重なりはありませんでした。
  したがって、単一の `bash scripts/harness.sh backend-integration` 実行の中では、file-local /
  `support::integration_guard()` による同一バイナリ内シリアライズで十分であり、ファイルをまた
  いだ `TRUNCATE` 競合は原理的に発生しません。
- **バイナリ内の並列は `#[tokio::test]` 単位**: 同じファイル内の複数 `#[tokio::test]` は既定で
  並列実行されるため、`support::integration_guard()` によるファイル内シリアライズは引き続き必
  要です。
- **実際のリスクはクロス invocation**: `support/mod.rs` の `#[ctor]` は `TEST_DATABASE_URL` が
  未設定なら test binary ごとに使い捨ての testcontainers Postgres を起動するため、既定では
  binary 間で DB を共有しません。一方、`scripts/test_backend_integrated.sh` は
  `docker-compose.test-db.yml`（`127.0.0.1:55432` の固定 Postgres）を `TEST_DATABASE_URL` として
  全 test binary に共有させる高速経路です。この経路を使っている状態で、**別々の端末やエージェ
  ントセッションが同時に** `cargo test` / `scripts/harness.sh backend-integration` /
  `backend-security-smoke` を実行すると、file-local な in-process mutex ではプロセスをまたいだ
  競合を防げません。
- **対策**: `scripts/harness.sh` の `backend-integration` と `backend-security-smoke` は、実行前
  に `BACKEND_INTEGRATION_LOCK`（既定 `target/harness-locks/backend-integration.lock`）を
  `flock` で取得します。これにより、同じ共有 DB を指す複数の harness 呼び出しが同時に走っても
  自動的に直列化されます。`flock` が存在しない環境ではロックなしで実行され、その旨をログに出し
  ます。
- **運用ルール**: 共有 DB 経路（`scripts/test_backend_integrated.sh` や `TEST_DATABASE_URL` を
  手動 export する運用）を使うときは、`scripts/harness.sh` 経由で実行することを推奨します。直接
  `cargo test` を叩く場合は、同じ共有 DB に対して複数の invocation を同時に走らせないでくださ
  い。使い捨て testcontainers 経路（`TEST_DATABASE_URL` 未設定）であれば、この制約はありません。

## Profiles

### `lint`

repo-wide の formatting / lint gate です。

1. `docs-check`
2. `fmt-check`
3. `clippy-backend`
4. `clippy-frontend`
5. `cargo clippy --all-targets -- -D warnings`

### `smoke`

最小限の end-to-end 確認です。

1. `doctor`
2. `backend-unit`
3. `api-smoke`
4. `frontend-login`

### `full`

repo の主要入口を一通り確認します。

1. `doctor`
2. `lint`
3. `backend-unit`
4. `backend-integration`
5. `api-smoke`
6. `frontend-login`

## Typical Flows

### Backend bug fix

```bash
bash scripts/harness.sh doctor
bash scripts/harness.sh fmt-check
bash scripts/harness.sh backend-unit
```

必要なら:

```bash
bash scripts/harness.sh backend-integration
```

### Auth / routing / cookie / refresh 変更

```bash
bash scripts/harness.sh doctor
bash scripts/harness.sh lint
bash scripts/harness.sh backend-unit
bash scripts/harness.sh api-smoke
bash scripts/harness.sh frontend-login
```

### PR 前の確認

```bash
bash scripts/harness.sh full
```

### Docs / harness / architecture 変更

```bash
bash scripts/harness.sh docs-check
git diff --check
```

## Reporting Format

PR / issue / final response では、次の 3 点だけを残します。

- 実行した stage
- pass / fail
- fail の場合は「今回差分」か「既存問題」か

例:

```text
- `doctor`: pass
- `docs-check`: pass
- `fmt-check`: pass
- `backend-unit`: pass
- `backend-integration`: not run
- `clippy-backend`: pass
- `clippy-frontend`: pass
- `lint`: pass
```
