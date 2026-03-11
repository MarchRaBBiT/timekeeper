# EP-20260311-p0-build-health-zero

## Goal

- `cargo clippy --all-targets -- -D warnings` を workspace 全体で green にする
- `cargo fmt --all` を baseline として repo 全体へ適用し、以後の機能 PR で unrelated formatting diff を出さない状態にする
- `clippy` 0 件と `cargo fmt` を repo の受け入れ条件と harness/reporting に組み込む

## Scope

- In: `backend`, `frontend`, `scripts/harness.sh`, `AGENTS.md`, `docs/manual/HARNESS.md`, `docs/exec-plans/tech-debt-tracker.md`, ExecPlan テンプレート系ドキュメント
- Out: P1 以降の大規模責務分割、UI/機能改善、lint を黙らせるためだけの恒久 `allow` 追加

## Current Baseline (Measured 2026-03-11)

- [x] `bash scripts/harness.sh doctor`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
  - 9 errors
  - 主因: `clippy::useless_conversion`, `clippy::too_many_arguments`
  - 集中箇所: `backend/src/handlers/admin/attendance_correction_requests.rs`, `backend/src/handlers/attendance_correction_requests.rs`, `backend/src/handlers/requests.rs`, `backend/src/repositories/attendance_correction_request.rs`
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
  - 31 clippy errors + 1 compile error
  - 主因: `unused_imports`, `dead_code`, `clone_on_copy`, `useless_conversion`, `type_complexity`, `too_many_arguments`
  - 追加修正が必要な compile break: `frontend/src/pages/admin_users/view_model.rs` の `UserResponse` 初期化漏れ
- [x] `cargo clippy --all-targets -- -D warnings`
  - frontend 違反に加えて `utoipa-swagger-ui` の build artifact path 解決エラーで失敗
  - workspace gate 化の前に build path 再現条件を固定する必要がある

## Done Criteria (Observable)

- [x] `cargo fmt --all --check` が workspace root で常に成功する
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` が成功する
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` が成功する
- [x] `cargo clippy --all-targets -- -D warnings` が workspace root で成功する
- [x] `scripts/harness.sh` に `fmt-check`, `clippy-backend`, `clippy-frontend`, `lint` の stage/profile が追加される
- [x] [AGENTS.md](/home/mrabbit/Documents/timekeeper/AGENTS.md) の完了条件から「既存違反なら切り分けて明記」の暫定運用を外し、repo-wide `fmt` / `clippy` green を要求する
- [x] [docs/manual/HARNESS.md](/home/mrabbit/Documents/timekeeper/docs/manual/HARNESS.md) と ExecPlan テンプレートが新しい lint gate を前提に更新される
- [x] 技術的負債トラッカーに P0 返済結果と残課題が反映される

## Constraints / Non-goals

- `#[allow(...)]` は原則追加しない。やむを得ない場合も file-level ではなく item-level かつ理由付きに限定する
- `cargo fmt --all` の baseline commit はロジック変更と分離する
- workspace `clippy` が環境差異で揺れる場合、lint 修正と build-path 修正を同一 PR に混ぜず、順序を固定する
- SQLx migration や機能仕様の変更は本タスクの主目的ではない

## Task Breakdown

1. [x] workspace `clippy` 実行前提を固定する
   - `utoipa-swagger-ui` の build artifact path failure を再現・是正する
   - root で `cargo clippy --all-targets` が frontend/backend 以外の理由で落ちない状態にする
2. [x] backend の既存 clippy 違反を 0 にする
   - correction request 系の `useless_conversion` を削除する
   - 引数過多は request/command struct 導入などで解消し、責務の寄せ先を repository/service で整理する
   - focused unit/integration を変更 seam ごとに実行する
3. [x] frontend の既存 clippy 違反と compile break を 0 にする
   - まず `UserResponse` 初期化漏れを解消する
   - `clone_on_copy`, `useless_conversion`, `unused_imports`, `redundant_locals` を機械的に解消する
   - `too_many_arguments`, `type_complexity`, `dead_code` は API filter struct / type alias / 呼び出し経路整理で潰す
   - 変更 seam ごとに wasm/lib test を実行する
4. [x] repo-wide formatting baseline を確定する
   - ロジック変更が一段落した時点で `cargo fmt --all` を repo 全体へ適用する
   - baseline 用 snapshot を切り、その後の PR は `fmt --check` 前提にする
5. [x] harness と受け入れ条件を更新する
   - `scripts/harness.sh` に lint stage を追加する
   - `smoke` / `full` にどこまで含めるかを決め、最低でも `lint` profile を共通入口として提供する
   - `AGENTS.md`, `docs/manual/HARNESS.md`, ExecPlan テンプレートの validation 欄を新 gate に合わせて更新する
6. [x] 運用へ定着させる
   - active ExecPlan / issue / PR に実測結果を記録する
   - green の節目ごとに `jj commit` で snapshot を残す

## Sequencing

1. build-path prerequisite fix
2. backend clippy zero
3. frontend clippy zero
4. full-workspace `cargo fmt --all`
5. harness/docs/acceptance update
6. final repo-wide validation and `jj` snapshot

`cargo fmt --all` を最後に寄せるのは、lint 修正中に unrelated diff を何度も混ぜないためです。baseline commit 作成後は、以降の機能変更で `fmt` 差分が対象ファイルに閉じるようになります。

## Validation Plan

- [x] `bash scripts/harness.sh doctor`
- [x] `cargo fmt --all --check`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --tests`
- [x] `cargo test -p timekeeper-frontend --lib`
- [x] `bash scripts/harness.sh lint`（stage 追加後）

## Suggested Delivery Split

### PR1: Workspace lint prerequisite

- `utoipa-swagger-ui` path failure の解消
- lint stage の雛形追加
- root `clippy` 実行条件を安定化

### PR2: Backend clippy zero

- backend 既存違反解消
- backend focused validation

### PR3: Frontend clippy zero

- frontend compile break + clippy 違反解消
- frontend focused validation

### PR4: Formatting baseline + acceptance gate

- repo-wide `cargo fmt --all`
- harness/docs/Completion Rule/ExecPlan テンプレート更新
- final workspace validation

## Risks / Watchpoints

- frontend の `type_complexity` / `too_many_arguments` は単純置換でなく API shape の小整理が必要
- `utoipa-swagger-ui` の失敗は lint debt ではなく環境・パス依存のため、未分離だと「clippy 0 件」の定義が曖昧になる
- repo-wide fmt を機能修正と混ぜると review 不可能になるため、baseline commit 分離は必須

## JJ Snapshot Log

- [x] `jj status`
- [x] prerequisite fix pass
- [x] backend clippy zero pass
- [x] frontend clippy zero pass
- [x] workspace fmt/clippy pass
- [ ] `jj commit -m "chore(lint): make workspace clippy and fmt green"`

## Progress Notes

- 2026-03-11: plan 作成。`cargo fmt --all --check` は成功、backend clippy は 9 errors、frontend clippy は 31 errors + 1 compile error、workspace clippy は加えて `utoipa-swagger-ui` path failure を確認。
- 2026-03-11: `backend/Cargo.toml` で `utoipa-swagger-ui` に `vendored` を追加し、`scripts/harness.sh` の clippy stage で `cargo clean -p utoipa-swagger-ui` を先行実行するよう修正。workspace `clippy` 前提を安定化。
- 2026-03-11: backend は correction request 系の引数過多を param struct 化し、`useless_conversion` と dead code を解消。`cargo clippy -p timekeeper-backend --all-targets -- -D warnings` green を確認。
- 2026-03-11: frontend は query struct / type alias 導入、`clone_on_copy`・`useless_conversion`・`type_complexity`・compile break を解消。`cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` と `cargo test -p timekeeper-frontend --lib` green を確認。
- 2026-03-11: backend integration で `admin_export_api` / `attendance_breaks_api` の `TRUNCATE` に `CASCADE` を追加し、`audit_log_middleware.rs` に `integration_guard()` を導入して flaky DB reset を抑止。`cargo test -p timekeeper-backend --tests` green を確認。
- 2026-03-11: `AGENTS.md`、`docs/manual/HARNESS.md`、`.agent/PLANS.md`、`docs/exec-plans/tech-debt-tracker.md` を lint gate 前提へ更新し、`bash scripts/harness.sh lint`、`cargo fmt --all --check`、workspace `cargo clippy --all-targets -- -D warnings` の green を確認。
