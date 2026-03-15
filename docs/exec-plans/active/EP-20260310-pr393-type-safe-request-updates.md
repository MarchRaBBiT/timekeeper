# EP-20260310-pr393-type-safe-request-updates

## Goal
- PR #393 の request update payload 型安全化を current `main` に整合する形で取り込み、無関係差分なしで merge する

## Scope
- In: `frontend/src/api/*`, `frontend/src/pages/requests/*`, 関連 frontend host tests
- Out: backend 変更、request create/cancel の仕様変更、path encoding の巻き戻し

## Done Criteria (Observable)
- [x] leave/overtime 更新 API が `Value` ではなく専用型で呼ばれる
- [x] `frontend` の request repository/view model/form が型付き update payload を使う
- [x] current `main` の path segment encoding を維持したまま関連テストが成功する
- [ ] PR #393 branch を push し、問題なければ merge する

## Constraints / Non-goals
- PR branch に混入している無関係変更は取り込まない
- `frontend/src/api/requests.rs` の current `main` 挙動を壊さない

## Task Breakdown
1. [x] typed update payload を API 層へ追加し、encoded path を維持した update メソッドへ差し替える
2. [x] repository / view model / form を typed payload ベースへ更新する
3. [x] API / repository / view model テストを更新して leave と overtime の update 経路を確認する
4. [ ] fmt と focused frontend tests を実行し、PR branch へ push して merge する

## Validation Plan
- [x] `cargo test -p timekeeper-frontend --lib requests_repository_calls_api -- --nocapture`
- [x] `cargo test -p timekeeper-frontend --lib requests_view_model_actions_update_messages -- --nocapture`
- [x] `cargo test -p timekeeper-frontend --lib api_client_attendance_and_requests_endpoints_succeed -- --nocapture`
- [x] `cargo fmt --all`
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` は既存の [`frontend/src/components/layout.rs`](/home/mrabbit/Documents/timekeeper/frontend/src/components/layout.rs) の unused import で失敗することを確認

## JJ Snapshot Log
- [x] `jj status`
- [x] frontend focused tests pass
- [ ] `jj commit -m "refactor: make request update payloads type-safe"`

## Progress Notes
- 2026-03-10: 計画作成。PR #393 の古い branch 差分を current `main` に clean に移植する方針を確定。
- 2026-03-10: `UpdateLeaveRequest` / `UpdateOvertimeRequest` を API, repository, view model, forms に導入。focused frontend tests と `cargo fmt --all` は成功。`cargo clippy -p timekeeper-frontend --all-targets -- -D warnings` は既存の `frontend/src/components/layout.rs` unused import で失敗。
