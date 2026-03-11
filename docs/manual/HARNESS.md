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
cargo test -p timekeeper-backend --tests
```

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

内部では [scripts/test_backend.sh](../../scripts/test_backend.sh) を使います。

### `frontend-login`

```bash
FRONTEND_BASE_URL=http://localhost:8080 bash scripts/harness.sh frontend-login
```

内部では [scripts/test_frontend_login.mjs](../../scripts/test_frontend_login.mjs) を使います。

## Profiles

### `lint`

repo-wide の formatting / lint gate です。

1. `fmt-check`
2. `clippy-backend`
3. `clippy-frontend`
4. `cargo clippy --all-targets -- -D warnings`

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

## Reporting Format

PR / issue / final response では、次の 3 点だけを残します。

- 実行した stage
- pass / fail
- fail の場合は「今回差分」か「既存問題」か

例:

```text
- `doctor`: pass
- `fmt-check`: pass
- `backend-unit`: pass
- `backend-integration`: not run
- `clippy-backend`: pass
- `clippy-frontend`: pass
- `lint`: pass
```
