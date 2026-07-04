# EP-20260704-notification-service-generalization

## Goal

`docs/exec-plans/attendance-domain-gap-tasks.md` T-16（汎用通知サービス基盤、G12 前半）を、
`docs/exec-plans/tech-debt-tracker.md` item #7（Queue / Worker Operational Debt）と統合して実施する。

1. lockout 専用だった通知 queue のメッセージ型を `notification_kind` + payload の形へ一般化し、
   T-17（申請提出/承認・却下・打刻漏れ通知）が同じ queue 基盤に乗れる拡張点を作る。既存 lockout
   通知の挙動・テストは互換維持する
2. worker の retry / DLQ / drain の運用境界を `docs/manual/RUNBOOK.md` に追記する
   （tech-debt #7 Recommended Fix 1–2）
3. `scripts/harness.sh` に `worker-once` smoke stage を追加し、`--list` / `docs/manual/HARNESS.md` /
   ルート `AGENTS.md` の Validation Ladder を同期する（tech-debt #7 Recommended Fix 3）
4. tech-debt-tracker.md item #7 の Status を実測で更新する

## Scope

- In: `backend/src/services/lockout_notification_queue.rs`,
  `backend/src/services/notification_queue.rs`（新規）, `scripts/harness.sh`,
  `docs/manual/RUNBOOK.md`, `docs/manual/HARNESS.md`, `AGENTS.md`,
  `docs/exec-plans/tech-debt-tracker.md`
- Out: T-17 の実配線（申請提出/承認・却下・打刻漏れ通知を実際に enqueue するハンドラ変更）、
  `backend/src/services/lockout_notification_worker.rs` の処理ロジック変更（dequeue の内部実装のみ
  一般化 queue 経由にし、公開シグネチャ・lockout 固有の処理内容は変更しない）、
  rebuild target（`crates/`, `apps/`）側の worker 設計、既存 integration test の弱体化・削除

## Done Criteria (Observable)

- [x] `backend/src/services/notification_queue.rs` に `notification_kind` タグ付き
      `NotificationJob` enum（internally tagged: `#[serde(tag = "notification_kind")]`）と
      generic enqueue/dequeue/retry-schedule/requeue-due/dead-letter 関数が存在する
- [x] `lockout_notification_queue.rs` の既存 public API（`LockoutNotificationJob` の全フィールド、
      `LOCKOUT_NOTIFICATION_QUEUE_KEY` / `_RETRY_KEY` / `_DLQ_KEY` の値、
      `enqueue_lockout_notification_job` / `dequeue_lockout_notification_job` /
      `schedule_lockout_notification_retry` / `requeue_due_lockout_notification_jobs` /
      `push_lockout_notification_dead_letter` のシグネチャ）が一切変更されていない
- [x] `backend/tests/auth_lockout_redis_integration.rs`（10 test）が無改変で green
- [x] `backend/tests/auth_flow_api.rs`（24 test）が無改変で green
- [x] `cargo test -p timekeeper-backend --lib` が green（新規 unit test 5 件を含む）
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` が green
- [x] `docs/manual/RUNBOOK.md` に "Notification Worker Operations" 節が存在し、queue/retry/DLQ の
      Redis key、depth 観測コマンド、drain/replay 手順が書かれている
- [x] `bash scripts/harness.sh --list` に `worker-once` が出力される
- [x] `bash scripts/harness.sh worker-once` が live Postgres/Redis に対して実測で green
      （必須環境変数未設定時は `die` で即座に fail することも確認済み）
- [x] `docs/manual/HARNESS.md` / ルート `AGENTS.md` の Validation Ladder に `worker-once` が
      反映されている
- [x] `bash scripts/harness.sh docs-check` が green
- [x] `docs/exec-plans/tech-debt-tracker.md` item #7 の Status / Summary 表 / Priority Queue /
      Suggested Execution Order が実測に合わせて更新されている

## Constraints / Non-goals

- 既存 lockout 通知の integration test の assertion を弱めて通すことは禁止（互換が壊れる場合は
  設計を見直す方針で臨んだ。実際には internally tagged enum による `notification_kind` フィールド
  追加のみで、既存 `LockoutNotificationJob` への直接デシリアライズ（serde は未知フィールドを既定で
  無視する）に影響がないことを unit test で確認できたため、テスト変更は不要だった）
- T-17（実際のハンドラ配線）は行わない。`NotificationJob` に新しい variant を追加する際、
  1 variant のみを前提にした既存の match（`dequeue_lockout_notification_job` 内）はコンパイルが
  通らなくなるが、これは意図的な安全弁（新 kind を lockout 専用 dequeue が黙って誤処理しないための
  強制）であり、今回の変更のスコープではこれ以上手を入れない
- SQLx migration は不要（Redis のみの変更）
- `.agent/PLANS.md` と `docs/exec-plans/attendance-domain-gap-tasks.md` は編集しない（並行作業との
  衝突回避のため、親セッション側で反映する）

## Task Breakdown

1. [x] 既存実装（`lockout_notification_queue.rs` / `lockout_notification_worker.rs` /
       `bin/lockout_notification_worker.rs`）と関連 integration test
       （`auth_lockout_redis_integration.rs`）を読み、互換制約（Redis key の厳密な値、
       `LockoutNotificationJob` の構造体リテラル直接構築、`process_lockout_notification_job` /
       `work_once` のシグネチャ）を洗い出す
2. [x] `notification_queue.rs` を新規作成し、`NotificationKind` / `NotificationJob`
       （internally tagged enum）と generic enqueue/dequeue/retry/requeue/dead-letter 関数を
       実装。wire compat（`notification_kind` タグ追加後も `LockoutNotificationJob` への直接
       デシリアライズが可能なこと）を unit test で先に固定する
3. [x] `lockout_notification_queue.rs` を、生の Redis 操作から `notification_queue.rs` の generic
       関数へ委譲する薄いアダプタへリファクタ。public API は一切変更しない
4. [x] `cargo build -p timekeeper-backend --lib` / `cargo test -p timekeeper-backend --lib` /
       `cargo fmt --all` / `cargo clippy -p timekeeper-backend --all-targets -- -D warnings` を
       通す（`dead_code` lint は既存スタイルに合わせて `#[allow(dead_code)]` で対応）
5. [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration` /
       `--test auth_flow_api` を live Postgres/Redis（testcontainers）で実行し、無改変で
       green であることを確認する
6. [x] `docs/manual/RUNBOOK.md` に "Notification Worker Operations" 節を追加する
7. [x] `scripts/harness.sh` に `require_env` ヘルパーと `run_worker_once` を追加し、
       `usage()` / `--list` / `case` に配線する
8. [x] `docs/manual/HARNESS.md` に `worker-once` の節を追加する
9. [x] ルート `AGENTS.md` の Validation Ladder（item 7 として挿入、以降を繰り上げ）と
       共通入口のコマンド例を更新する
10. [x] podman で使い捨て Postgres/Redis を用意し、`DATABASE_URL` / `REDIS_URL` / `JWT_SECRET`
        を設定して `bash scripts/harness.sh worker-once` を実測で実行する（green 確認 +
        必須環境変数未設定時の fail 確認）
11. [x] `bash scripts/harness.sh docs-check` を実行する
12. [x] `docs/exec-plans/tech-debt-tracker.md` item #7 の Status / Summary 表 / Priority Queue /
        Suggested Execution Order / Notes を更新する
13. [x] 本 ExecPlan を作成し、Progress Notes に実測結果を記録する

## Validation Plan

- [x] `cargo fmt --all --check`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration`
- [x] `cargo test -p timekeeper-backend --test auth_flow_api`
- [x] `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`
- [x] `bash scripts/harness.sh worker-once`（live Postgres/Redis, podman 経由）
- [x] `bash scripts/harness.sh docs-check`

## Git Snapshot Log

- [ ] `git status --short`
- [x] 対象テスト pass（上記 Validation Plan 参照）
- [ ] commit（親セッションが実施。本タスクの指示で git commit / git add は本 EP 実施中は行っていない）

## Progress Notes

- 2026-07-04: 計画作成・実装開始。既存 `auth_lockout_redis_integration.rs` /
  `auth_flow_api.rs` を読み、`LockoutNotificationJob` の構造体リテラル直接構築（DLQ テスト）と
  queue エントリの直接デシリアライズ（`queued_lockout_notifications` ヘルパー）が最も厳しい互換
  制約であることを特定。internally tagged enum（`#[serde(tag = "notification_kind")]`）による
  wire format なら、追加フィールドが `serde` のデフォルト（未知フィールド無視）により
  `LockoutNotificationJob` への直接デコードを壊さないと判断し、この設計を採用。
- 2026-07-04: `backend/src/services/notification_queue.rs` を新規作成（`NotificationKind` /
  `NotificationJob` / generic enqueue・dequeue・retry・requeue・dead-letter 関数、wire compat を
  固定する unit test 3 件）。`lockout_notification_queue.rs` を薄いアダプタへリファクタ
  （public API・Redis key 値は無変更、内部実装のみ generic 関数へ委譲）。unit test を 2 件追加
  （`notification_queue_wire_compat_deserializes_into_lockout_notification_job` /
  `retrying_increments_attempt_without_mutating_original`）。`lockout_notification_worker.rs` /
  `bin/lockout_notification_worker.rs` は無変更（公開 API が変わっていないため）。
- 2026-07-04: 検証実施。
  - `cargo build -p timekeeper-backend --lib`: pass
  - `cargo test -p timekeeper-backend --lib`: pass（384 passed; 0 failed。既存 379 + 新規 5）
  - `cargo fmt --all --check`: pass（1 回 `cargo fmt --all` を実行後 green）
  - `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`: pass（0 warnings。
    `NotificationKind` / `NotificationJob::kind()` に `#[allow(dead_code)]` を追加して対応。
    このクレートは bin ターゲットとして dead_code lint が pub 関数にも適用されるため、既存
    lockout queue 関数群と同じスタイルに合わせた）
  - `cargo test -p timekeeper-backend --test auth_lockout_redis_integration`（testcontainers
    Postgres + 個別起動 Redis コンテナ）: pass（10 passed; 0 failed。うち
    `lockout_notification_is_enqueued_in_redis` / `worker_sends_enqueued_lockout_notification_job` /
    `worker_moves_exhausted_notification_to_dlq` / `lockout_enqueue_latency_is_stable_...` が
    今回の generalization を直接検証する既存テスト）
  - `cargo test -p timekeeper-backend --test auth_flow_api`: pass（24 passed; 0 failed）
- 2026-07-04: `docs/manual/RUNBOOK.md` に "Notification Worker Operations" 節を追加
  （アーキテクチャ概要、Redis key 表、retry backoff 実測根拠、`redis-cli` による depth 観測
  コマンド、worker 起動方法、DLQ 手動 replay/破棄手順）。
- 2026-07-04: `scripts/harness.sh` に `require_env` ヘルパーと `run_worker_once`
  （`DATABASE_URL` / `REDIS_URL` / `JWT_SECRET` 必須チェック → `cargo run --bin
  lockout_notification_worker -- --once`）を追加し、`usage()` / `--list` / `case` に配線。
  `docs/manual/HARNESS.md` に節を追加、ルート `AGENTS.md` の Validation Ladder に item 7 として
  挿入（以降 clippy-backend 以降を繰り下げ）、共通入口コマンド例にも追加。
- 2026-07-04: podman で使い捨て Postgres（`postgres:16`）+ Redis（`redis:7-alpine`）を起動し、
  `DATABASE_URL` / `REDIS_URL` / `JWT_SECRET` を設定して `bash scripts/harness.sh worker-once` を
  実行 → exit 0（queue が空のため `Ok(None)` で `--once` ループが即終了する経路を実測確認）。
  環境変数を unset した状態で再実行し、`DATABASE_URL` 欠如を `die` で即座に検知することも確認。
  検証後、コンテナは削除済み（`podman rm -f`）。
  - `bash scripts/harness.sh docs-check`: pass
- 2026-07-04: `docs/exec-plans/tech-debt-tracker.md` item #7 の Status を「返済済み」に更新し、
  Recommended Fix 1–3 の返済内容・実測検証結果・Constraints を記載。Summary 表 (2026-07-04
  status 列)、Priority Queue（該当行を打ち消し線 + 返済済み表記へ）、Suggested Execution Order
  （item 11 を打ち消し線へ）、Notes（2026-07-04 追加返済の bullet）を整合させた。
- 2026-07-04: 本 ExecPlan 作成、Progress Notes に実測結果を記録。git commit は親セッションが
  実施するため本タスクでは `git add` / `git commit` を行っていない。
- 2026-07-04: レビューで backward compatibility gap を 1 件指摘され、修正した。
  - 指摘: `dequeue_lockout_notification_job` が `notification_queue::dequeue_notification_job`
    （internally tagged `NotificationJob` への strict デシリアライズ）に委譲されたことで、
    deploy 時点で queue / retry ZSET に残っていた legacy（タグなし `LockoutNotificationJob` を
    直接 serialize した）形式の job が deserialize エラーになる。`BRPOP` は既に pop 済みのため、
    この job は DLQ にも入らず消失する（旧方向の互換は unit test で担保されていたが、
    「legacy → 新 dequeue」方向は未担保だった）。
  - 修正: `notification_queue.rs` に `dequeue_notification_payload(pool, queue_key,
    timeout_seconds) -> anyhow::Result<Option<String>>` を追加し、raw JSON を返す BRPOP に
    切り出した。`dequeue_notification_job` はこれを使って strict タグ付き parse を行うよう整理
    （generic モジュールは lockout 固有の legacy 知識を持たない）。
    `lockout_notification_queue.rs` に private `decode_lockout_notification_payload(raw: &str)
    -> anyhow::Result<LockoutNotificationJob>` を追加し、`dequeue_lockout_notification_job` は
    raw payload を受け取ってこれで decode する。decode はまず `NotificationJob`（タグ付き）として
    parse を試み、失敗したら legacy の素の `LockoutNotificationJob` として parse する
    fallback を行う。両方失敗した場合のみ Err とし、エラーメッセージに
    `tagged_parse_error=...; legacy_parse_error=...` の形で両方の失敗理由を含める。
  - 追加 unit test（`backend/src/services/lockout_notification_queue.rs`）:
    `decode_lockout_notification_payload_falls_back_to_legacy_untagged_shape`
    （legacy 形式 → 正しく decode されることを固定）、
    `decode_lockout_notification_payload_decodes_current_tagged_shape`
    （現行タグ付き形式が primary path で decode されることを固定）、
    `decode_lockout_notification_payload_reports_both_errors_when_genuinely_corrupt`
    （両方失敗時のエラーメッセージに両方の理由が含まれることを固定）。
  - `notification_queue.rs` の module docs に "Legacy (untagged) payload fallback" 節を追記し、
    `lockout_notification_queue.rs` の module docs にも fallback の存在を追記。
    `docs/manual/RUNBOOK.md` の "Notification Worker Operations" > Notes に、この互換 fallback
    （deploy を跨ぐ in-flight job が消えないこと）を 1 項目追記。
  - 変更ファイル: `backend/src/services/notification_queue.rs`,
    `backend/src/services/lockout_notification_queue.rs`, `docs/manual/RUNBOOK.md`
    （いずれも `.agent/PLANS.md` / `docs/exec-plans/attendance-domain-gap-tasks.md` /
    他エージェント成果物には触れていない）。
  - 検証（実測、いずれも green）:
    - `cargo fmt --all --check`: pass（1 回 `cargo fmt --all` 実行後 green。match 式の
      フォーマットが自動整形された）
    - `cargo test -p timekeeper-backend --lib`: pass（387 passed; 0 failed。既存 384 + 新規
      unit test 3 件）
    - `cargo test -p timekeeper-backend --test auth_lockout_redis_integration`
      （podman 上の testcontainers Postgres + Redis）: pass（10 passed; 0 failed。無改変で green
      — このテストは既存互換方向のみを検証しており、今回追加した legacy fallback 方向は
      新規 unit test 側でのみ直接検証している）
    - `cargo clippy -p timekeeper-backend --all-targets -- -D warnings`: pass（0 warnings）
  - git commit / git add は本修正でも実施していない（親セッションが実施）。
