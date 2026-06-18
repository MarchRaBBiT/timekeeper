# EP-20260612-rebuild-target-workspace

## Goal
- `docs/design-docs/rebuild-architecture.md` の移行戦略 1 に沿って、target workspace の crate/app 境界を実体化する

## Scope
- In: root `Cargo.toml`, `apps/api`, `apps/web`, `apps/worker`, `crates/domain`, `crates/app`, `crates/contract`, `crates/infra-postgres`, `crates/infra-security`, `crates/infra-integrations`, `crates/observability`, focused tests, this ExecPlan
- Out: 既存 endpoint の移植、DB migration 作成、Leptos/Axum/SQLx の version migration、既存 `backend/` と `frontend/` の置換

## Done Criteria (Observable)
- [x] target workspace の app/crate shell が Cargo workspace member として登録されている
- [x] `crates/domain` が Axum/SQLx/Leptos に依存しない business value object を持つ
- [x] `crates/contract` が shared API DTO を持ち、wire format test がある
- [x] `crates/app` が repository trait を受け取る use case entrypoint を持ち、focused test がある
- [x] focused Rust tests と `bash scripts/harness.sh docs-check` が成功する

## Constraints / Non-goals
- 現行 production code の route / wire format / DB schema は変更しない
- `timekeeper-backend/` と `timekeeper-frontend/` の未追跡ディレクトリには触れない
- この slice では empty shell 以上の adapter 実装を作らない

## Task Breakdown
1. [x] app/crate ディレクトリと manifest を追加する
2. [x] `domain` / `contract` / `app` の focused tests を先に追加する
3. [x] focused tests を通す最小実装を追加する
4. [x] fmt と focused validation を実行する
5. [x] plan と作業結果を更新する

## Validation Plan
- [x] `cargo test -p timekeeper-domain -p timekeeper-contract -p timekeeper-app`
- [x] `cargo check -p timekeeper-api -p timekeeper-worker -p timekeeper-web -p timekeeper-infra-postgres -p timekeeper-infra-security -p timekeeper-infra-integrations -p timekeeper-observability`
- [x] `cargo clippy -p timekeeper-domain -p timekeeper-contract -p timekeeper-app -p timekeeper-api -p timekeeper-worker -p timekeeper-web -p timekeeper-infra-postgres -p timekeeper-infra-security -p timekeeper-infra-integrations -p timekeeper-observability --all-targets -- -D warnings`
- [x] `cargo fmt --all --check`
- [x] `bash scripts/harness.sh docs-check`
- [x] `git status --short`

## Git Snapshot Log
- [x] `git status --short`
- [x] focused tests pass
- [ ] commit pending user direction

## Progress Notes
- 2026-06-12: `crates/` と `apps/` が未作成であることを確認。移行戦略 1 に絞り、empty app shell と最小 domain/contract/app boundary test から開始。
- 2026-06-12: `apps/api`, `apps/web`, `apps/worker`, `crates/domain`, `crates/app`, `crates/contract`, `crates/infra-postgres`, `crates/infra-security`, `crates/infra-integrations`, `crates/observability` を workspace member 化。`WorkDate`, `ClockInRequest`, `ClockIn` use case の focused tests と新規 package clippy が成功。
