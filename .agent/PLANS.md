# Exec Plans

複雑タスクのみ、このファイルに実行計画を作成する。  
小規模修正では計画作成は任意。

## 運用ルール
- 複数レイヤー横断（`backend` + `frontend` + `e2e` など）の変更は原則 `ExecPlan` を作成する
- 完了条件は「確認可能な挙動」で定義する（例: APIレスポンス、画面表示、テスト成功）
- 実装中はチェックボックスを更新し、未完了の作業を残す
- テスト成功の節目ごとに `jj` スナップショットを残す

## テンプレート

```md
# EP-YYYYMMDD-<short-slug>

## Goal
- <このタスクで達成すること>

## Scope
- In: <対象>
- Out: <対象外>

## Done Criteria (Observable)
- [ ] <確認可能な完了条件1>
- [ ] <確認可能な完了条件2>

## Constraints / Non-goals
- <制約や今回やらないこと>

## Task Breakdown
1. [ ] <実装タスク1>
2. [ ] <実装タスク2>
3. [ ] <実装タスク3>

## Validation Plan
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `./scripts/test_backend_integrated.sh` または `cargo test --test <target>`
- [ ] `pwsh -File .\scripts\test_backend.ps1`（必要時）
- [ ] `cd frontend; wasm-pack test --headless --firefox`（必要時）
- [ ] `cd e2e; node run.mjs`（必要時）

## JJ Snapshot Log
- [ ] `jj status`
- [ ] <対象テスト> pass
- [ ] `jj commit -m "chore(test): snapshot after <test_target> pass"`

## Progress Notes
- YYYY-MM-DD: <実施内容>
```

---

# EP-20260206-frontend-coverage-90

## Goal
- frontend (`timekeeper-frontend`) の line coverage を `cargo frontend-cov` で 90%以上にする

## Scope
- In: frontend のテスト追加、テスト補助モジュール追加、既存テストの修正
- Out: バックエンド変更、機能仕様変更、E2E シナリオ追加

## Done Criteria (Observable)
- [x] `cargo test -p timekeeper-frontend --lib` が成功する
- [x] `cargo frontend-cov` の line coverage が 90%以上
- [x] 追加したテストが主要分岐（正常/異常/境界）をカバーしている

## Constraints / Non-goals
- 実装ロジックの仕様は変更しない（テスト追加中心）
- 既存 UI/UX の見た目や操作フローを変更しない
- 進捗はこの計画のチェックボックスで管理する

## Task Breakdown
1. [x] frontend テスト基盤（`test_support`）を整備し、現状 failing test を解消する
2. [x] `state/config` と `config` の共有状態競合を直列化し、分岐テストを追加する
3. [x] `cargo frontend-cov` alias を導入し、対象スコープで 90% 以上を達成する
4. [x] 最終検証（fmt, 対象テスト, llvm-cov）を実施する

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-frontend --lib`
- [x] `cargo frontend-cov`

## JJ Snapshot Log
- [x] `jj status`
- [x] `cargo test -p timekeeper-frontend --lib` pass
- [x] `jj commit -m "chore(test): snapshot after frontend lib test pass"`
- [x] `cargo frontend-cov` 90% pass
- [x] `jj commit -m "chore(test): snapshot after frontend coverage 90 pass"`

## Progress Notes
- 2026-02-06: 初期計画作成。現状は `crate::test_support` 未定義で frontend lib test が失敗。
- 2026-02-06: `frontend/src/test_support` を追加し、`detail.rs` host test が参照する基盤を復旧。
- 2026-02-06: `state/config` テストを拡張し、`config` 側に test serial lock を追加して共有状態テストを安定化。
- 2026-02-06: `.cargo/config.toml` に `cargo frontend-cov` alias を追加し、`cargo frontend-cov` で line coverage 100% を確認。
- 2026-02-06: `jj commit -m "chore(test): snapshot after frontend lib test pass"` を作成し、続けて coverage 達成スナップショットを作成。
