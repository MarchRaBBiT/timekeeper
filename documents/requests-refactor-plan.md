# Requests Page Refactor Plan

`documents/pages-refactor-plan.md` の Requests セクションをベースに、`frontend/src/pages/requests.rs`（単一ファイルで休暇・残業申請フォームと申請一覧をハンドリングしている状態）をモジュール構成へ再編する具体的なロードマップをまとめる。

## 1. 目的

- 休暇申請フォーム・残業申請フォーム・申請一覧を個別のコンポーネントへ分割し、`panel.rs` は状態接続・権限チェックに専念できるようにする。
- API 呼び出しを `requests/repository.rs` に集約し、`create_resource` / `create_action` を通じて pending/error UI を統一する。
- 申請フィルタやフォーム状態を `utils.rs` で管理し、`wasm_bindgen_test` で TDD を実践可能にする。
- Playwright シナリオを整備し、フォーム送信と一覧反映の回帰確認を自動化する。

## 2. 現状と課題 (`frontend/src/pages/requests.rs`)

1. 休暇申請・残業申請・一覧すべてが単一の `RequestsPage` にベタ書きされ、フォーム状態・API 呼び出し・一覧描画が混在している。
2. `ApiClient::new()` をフォーム送信毎に生成し `spawn_local` で呼び出しており、pending/error のフィードバックが存在しない。
3. 申請一覧は `serde_json::Value` のまま扱っており、構造化された型や詳細表示がない。
4. フォームバリデーション（入力必須、日付範囲チェックなど）がない／再利用できない。
5. 依存モジュール（mod/layout/components/repository/utils）が存在せず、他ページと共通のリファクタリングパターンに沿っていない。

## 3. 目標構成

```
frontend/src/pages/requests/
    mod.rs               # pub use panel::RequestsPage
    layout.rs            # ページタイトル・説明などのラッパー
    panel.rs             # create_resource + create_action の配線と状態ハブ
    repository.rs        # API 呼び出し (leave/ot/my_requests/detail)
    utils.rs             # LeaveFormState / OvertimeFormState / RequestFilterState
    types.rs             # RequestSummary / RequestDetail / enum などの型
    components/
        leave_form.rs
        overtime_form.rs
        list.rs
        detail_modal.rs
        filter.rs
```

## 4. リファクタリングステップ

1. **モジュール骨格の作成**  
   - `frontend/src/pages/requests/mod.rs` と `layout.rs` を追加し、既存 `requests.rs` からページ公開部分を移植。
   - `panel.rs` を新設して `RequestsPage` を委譲。既存コードを段階的に panel / components / repository へ分割する。

2. **Repository + Types 抽出 (TDD)**  
   - `repository.rs` で以下 API をラップ：`submit_leave(request)` / `submit_overtime(request)` / `list_my_requests()` / `fetch_detail(id)`。
   - `types.rs` に `RequestSummary`, `RequestStatus`, `RequestDetail`, `LeavePayload`, `OvertimePayload` を定義し、JSON とのシリアライズをテスト。

3. **状態・ユーティリティ整備**  
   - `utils.rs` に `LeaveFormState`, `OvertimeFormState`, `RequestFilterState` を実装し、検証ロジック・初期値・リセット処理を `#[cfg(test)]` 付きで提供。
   - pending/error メッセージを `MessageState` などで一元管理。

4. **UI コンポーネント分割**  
   - `components/leave_form.rs` / `overtime_form.rs` を作成し、フォーム描画 + submit action dispatch を担当させる。
   - `components/list.rs` で申請一覧テーブルを `RequestSummary` ベースで表示。`components/detail_modal.rs` が詳細表示 + approve/reject（将来的に）を担えるよう拡張余地を残す。
   - `components/filter.rs` が簡易フィルタやリロード操作を提供。

5. **Panel での接続 & 権限**  
   - `panel.rs` で `create_action` (`submit_leave_action`, `submit_overtime_action`) と `create_resource` (`my_requests_resource`, `detail_resource`) を定義。
   - Action 成功時にメッセージをセットし、`Resource::refetch()` or reload signal で一覧を更新。
   - 将来的な承認機能を見据えて `detail_modal` への props を設計（現状は self service 表示のみ）。

6. **テスト & ドキュメント**  
   - `utils.rs` / `types.rs` のユニットテストと wasm_bindgen_test を追加。
   - `documents/pages-refactor-plan.md` と本計画書に作業完了ログを記録。
   - Playwright (`e2e/requests.mjs` など) で休暇申請→一覧反映フローを自動化。

## 5. TODO チェックリスト

- [x] `frontend/src/pages/requests/` に `mod.rs` / `layout.rs` / `panel.rs` を追加し、既存 `requests.rs` を段階的に置き換える。
- [x] `repository.rs` / `types.rs` を実装して API 呼び出しを集約（`submit_leave`, `submit_overtime`, `list_my_requests`, `fetch_detail`）。型のシリアライズ/デシリアライズテストを追加。
- [x] `utils.rs` に Leave/Overtime フォーム状態・フィルタ状態を実装し、`wasm_bindgen_test` でバリデーションをカバー。
- [x] `components/leave_form.rs`, `components/overtime_form.rs`, `components/list.rs`, `components/detail_modal.rs`, `components/filter.rs` を作成し、UI を分割。
- [x] `panel.rs` で `create_resource` / `create_action` を接続し pending/error 表示を統一。
- [ ] `documents/pages-refactor-plan.md` および本計画書を更新し、`cargo test -p timekeeper-frontend --lib`, `wasm-pack test --headless --firefox frontend`, Playwright シナリオの結果を記録。

## テストログ

- 2025-11-19: `cargo test -p timekeeper-frontend --lib`
- TBD: `wasm-pack test --headless --firefox frontend`（Requests モジュール移行後に実行予定）
- TBD: `node e2e/requests.mjs`（新シナリオ追加済み。API 接続環境で実行）
