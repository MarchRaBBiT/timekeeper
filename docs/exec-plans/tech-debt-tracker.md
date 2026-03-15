# Technical Debt Tracker

## Purpose

このファイルは、現行コードと設計に存在する技術的負債を「後で読むメモ」ではなく、優先順位付きの実行バックログとして残すための tracker です。  
対象は 2026-03-10 時点の `main` 相当コードベースです。

## Summary

最優先は次の 3 点でした。2026-03-11 時点で P0 build health debt は返済済みです。

1. backend / frontend の中核モジュールが肥大化し、PR の clean reapply と review コストを押し上げている
2. repository / handler / test support / docs の source-of-truth が分散し、変更 1 件あたりの追従コストが高い
3. harness / fixture の fragile point が残っている

## Priority Queue

| Priority | Debt | Scope | Why now |
|---|---|---|---|
| P1 | Backend god modules | `handlers/auth.rs`, `main.rs`, `middleware/audit_log.rs`, `handlers/attendance.rs` | review / rebase / regression isolation が重い |
| P1 | Frontend god modules | `api/client.rs`, `pages/admin/components/holidays.rs` | UI 変更で unrelated diff が混ざりやすい |
| P1 | Repository / handler duplication | attendance, correction requests, request repos | 挙動変更の適用点が複数に分散 |
| P1 | Test harness fragility | integration tests, env mutation, live smoke | flaky / slow / local-only failure を生みやすい |
| P2 | Docs source-of-truth drift | `AGENTS.md`, `docs/*`, plan placement | 実装ルールと実際の repo 状態がずれる |
| P2 | Queue / worker operational debt | lockout notification worker | いまは動くが運用境界がまだ薄い |
| P2 | Frontend i18n follow-up debt | locale foundation, shared/core page localization | PR #430 / #431 で merge-blocking ではない residual review item が残っている |

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

### 2. P1: Backend God Modules

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

### 3. P1: Frontend God Modules

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

### 4. P1: Repository / Handler Duplication

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

### 6. P2: Docs Source-Of-Truth Drift

**Symptoms**

- `AGENTS.md` / subdirectory `AGENTS.md` のサイズ・記述が current code とずれている
- file size / line count の記述が stale
- plan 配置の運用が `docs/generated/exec-plans` と今回要求された `docs/exec-plans` に分かれている

**Evidence**

- [frontend/AGENTS.md](../../frontend/AGENTS.md)
  - `api/client.rs` を `692 lines` と書いているが現状は `942 lines`
- [backend/AGENTS.md](../../backend/AGENTS.md)
  - `handlers/auth.rs` を `642 lines` と書いているが現状は `1993 lines`
- [AGENTS.md](../../AGENTS.md)
- [docs/generated/exec-plans/active](../generated/exec-plans/active)
- [docs/exec-plans/tech-debt-tracker.md](./tech-debt-tracker.md)

**Impact**

- agent / reviewer が stale metadata を信用して誤った見積もりをしやすい
- docs の配置規約が揺れると、次の文書追加時に迷う

**Recommended Fix**

1. `AGENTS.md` の line-count のような変化しやすい metadata を削除する
2. exec plan の保存先を 1 箇所に統一する
3. tracker / runbook / design-doc の役割を README で明記する

---

### 7. P2: Queue / Worker Operational Debt

**Symptoms**

- lockout notification worker は実装されたが、運用境界が doc と harness にまだ十分現れていない
- queue semantics は implicit で、FIFO/LIFO や drain strategy が operator 向けに明文化されていない
- worker binary の health / lag / DLQ monitor が manual 化されていない

**Evidence**

- [backend/src/bin/lockout_notification_worker.rs](../../backend/src/bin/lockout_notification_worker.rs)
- [backend/src/services/lockout_notification_queue.rs](../../backend/src/services/lockout_notification_queue.rs)
- [backend/src/services/lockout_notification_worker.rs](../../backend/src/services/lockout_notification_worker.rs)
- [docs/manual/RUNBOOK.md](../manual/RUNBOOK.md)

**Impact**

- 本番障害時に「worker が落ちているのか、queue が溜まっているのか、SMTP が失敗しているのか」を素早く分けにくい
- security feature は実装済みでも運用 readiness がまだ薄い

**Recommended Fix**

1. worker runbook を追加する
2. queue depth / retry depth / DLQ depth の観測項目を定義する
3. harness に `worker-once` smoke stage を追加する

---

### 8. P2: Frontend I18n Follow-up Debt

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

## Suggested Execution Order

1. P0 Build health debt
2. P1 Backend god modules
3. P1 Frontend god modules
4. P1 Repository / handler duplication
5. P1 Test harness fragility
6. P2 Docs source-of-truth drift
7. P2 Queue / worker operational debt
8. P2 Frontend i18n follow-up debt

## Notes

- この tracker は「今すぐ全部直す」ためではなく、issue 化・分割計画の起点として使う
- まずは P0 を返済しないと harness gate の信頼性が上がらない
