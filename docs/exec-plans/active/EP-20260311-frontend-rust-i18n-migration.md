# EP-20260311-frontend-rust-i18n-migration

Related Issue: #429

## Goal

- frontend の日本語直書きメッセージを `rust-i18n` ベースの翻訳キー参照へ移行する
- locale ファイル、locale 選択、翻訳ヘルパーを導入し、今後の UI 文言追加が直書き前提にならない状態にする
- ヘッダー右上の locale 切替導線から `en` / `ja` を完全切替できるようにする
- 主要な shared UI / user-facing page / admin page で日本語直書きを排除し、残件を測定可能にする

## Scope

- In: `frontend/Cargo.toml`, `frontend/src/**/*`, 新規 `frontend/locales/*`, frontend docs/rules 更新
- Out: backend API の文言仕様変更、DB migration、サーバーサイド翻訳、英訳品質の最終レビュー

## Current Baseline (Measured 2026-03-11)

- [ ] `rust-i18n` は frontend 未導入
- [ ] locale ディレクトリ未作成
- [ ] production code の日本語直書きが広範囲に存在
  - 代表箇所: `frontend/src/components/cards.rs`, `frontend/src/components/forms.rs`, `frontend/src/components/layout.rs`, `frontend/src/pages/settings/panel.rs`, `frontend/src/pages/admin/components/attendance.rs`, `frontend/src/pages/admin_users/*`
- [ ] `frontend/src` 配下で日本語文字列を含む `.rs` は 74 ファイル

## Done Criteria (Observable)

- [ ] `frontend/Cargo.toml` に `rust-i18n` が追加され、app root で `i18n!` 初期化が行われる
- [ ] `frontend/locales/ja.yml` と `frontend/locales/en.yml` が追加される
- [ ] locale 選択の source of truth が frontend state/context に定義され、ヘッダー右上から変更できる
- [ ] locale 選択は `localStorage` に保存され、初回訪問時はブラウザ言語判定で `ja` を選び、未一致時は `en` を既定にする
- [ ] shared components の表示文言が `t!` ベースへ移行される
- [ ] attendance / dashboard / requests / settings / login / mfa / reset-password の表示文言が `t!` ベースへ移行される
- [ ] admin / admin_users / admin_audit_logs / admin_export の表示文言が `t!` ベースへ移行される
- [ ] `en` / `ja` locale 間で frontend 内部生成 UI 文言が完全切替される
- [ ] 英語文言について自然な英語レビューが完了している
- [ ] frontend production code の日本語直書きが、翻訳定義・テスト assertion・コメント・ログ・API モックデータ・fixture 文字列を除いて解消される
- [x] frontend 文言追加時に直書き禁止であることが `docs/` の運用ルールと `frontend/AGENTS.md` に明記される

## Constraints / Non-goals

- 既存 UI 文言の意味は原則維持し、翻訳基盤導入と同時に copy rewrite を広げない
- `en` / `ja` の 2 locale を最小セットとし、どちらでも UI が成立する完全切替を前提にする
- locale 切替導線はヘッダー右上に配置する
- locale 選択値は `localStorage` を source of persistence とする
- 一部の API error payload は backend から自然言語で来るため、今回の「真の国際化」は frontend 内部生成メッセージを優先する
- backend 由来の自然言語エラーは今回の翻訳対象外とし、既存どおり表示を継続する
- ラベルのみ翻訳対象とし、日付・曜日・単位フォーマットは今回変更しない
- translation key は `pages.login.title` のような階層型 naming を採用する
- 表示文言 assertion は translation key 非依存へ寄せる

## Task Breakdown

### PR1: Foundation

1. [x] i18n 基盤を導入する
   - [x] `frontend/Cargo.toml` に `rust-i18n` を追加する
   - [x] `frontend/locales/ja.yml` を新規作成する
   - [x] `frontend/locales/en.yml` を新規作成する
   - [x] app root で `i18n!` 初期化を行う
2. [x] locale state と永続化を導入する
   - [x] current locale を保持する state/context を追加する
   - [x] ブラウザ言語判定で初期 locale を解決する
   - [x] locale 選択を `localStorage` に保存する
   - [x] host test で `en` / `ja` 初期解決と永続化を固定する
3. [x] ヘッダー右上の locale 切替導線を追加する
   - [x] `components/layout.rs` に切替 UI を追加する
   - [x] locale 変更時に全体へ反映されることを確認する
4. [x] shared components の土台を移行する
   - [x] `components/layout.rs` の固定文言を `t!` 化する
   - [x] `components/confirm_dialog.rs` の固定文言を `t!` 化する
   - [x] `components/error.rs` の固定文言を `t!` 化する
   - [x] key naming を階層型で統一する

### PR2: Core Pages

5. [ ] shared components の主要文言を移行する
   - [ ] `components/forms.rs` のラベル・状態文言を `t!` 化する
   - [ ] `components/cards.rs` のカード見出し・ラベルを `t!` 化する
   - [ ] 補間を含む shared component 文言を translation helper 経由へ寄せる
6. [ ] user-facing core pages を移行する
   - [ ] attendance を移行する
   - [ ] dashboard を移行する
   - [ ] requests を移行する
   - [ ] login / forgot_password / reset_password を移行する
   - [ ] mfa / settings を移行する
7. [ ] frontend 内部生成の status / confirmation 文言を key 化する
   - [ ] `match => 文字列` を `match => key` or translation helper に寄せる
   - [ ] ラベルのみ翻訳し、日付・曜日・単位フォーマットは維持する
8. [ ] core pages 向けテストを translation key 非依存へ更新する
   - [ ] 文字列の完全一致に依存する assertion を見直す
   - [ ] locale 固定値ではなく構造・状態・key 解決結果で検証する

### PR3: Admin Pages

9. [ ] admin pages を移行する
   - [ ] admin を移行する
   - [ ] admin_users を移行する
   - [ ] admin_audit_logs を移行する
   - [ ] admin_export を移行する
10. [ ] admin 周辺の helper / message を移行する
   - [ ] admin 向け modal / feedback / banner 文言を `t!` 化する
   - [ ] related shared helpers の文字列生成を translation helper 経由へ寄せる
   - [ ] backend 由来エラーをそのまま表示する境界を明示する
11. [ ] 英語文言の自然さレビューを反映する
   - [ ] admin 含む `en.yml` を通読し、不自然な直訳を修正する
   - [ ] 略語、ボタン文言、エラーメッセージのトーンを揃える

### PR4: Residual Cleanup

12. [ ] 残直書きを scan して例外を整理する
   - [ ] `rg '[ぁ-んァ-ヶ一-龠々ー]' frontend/src -g '*.rs'` の結果を確認する
   - [ ] 許容例外（テスト assertion / コメント / ログ / API モック / fixture）を切り分ける
   - [ ] production code の残件を解消または明記する
13. [ ] 運用ルールを追加する
   - [ ] `docs/` に i18n 運用ルールを追加する
   - [ ] `frontend/AGENTS.md` に「UI 文言は translation key 経由」を追記する
14. [ ] 最終検証と仕上げを行う
   - [ ] 表示文言 assertion を translation key 非依存に更新したテストが green
   - [ ] `en` / `ja` 切替と `localStorage` 永続化を確認する focused host test を通す
   - [ ] full residual scan の結果を plan / PR に記録する

## Sequencing

1. 基盤導入
2. shared components
3. core pages
4. admin pages
5. error/status helpers
6. residual scan + docs update

shared components を先に移行するのは、以後の各画面が同じ translation helper と key naming を再利用できるようにするためです。

## Validation Plan

### PR1: Foundation

- [x] `cargo fmt --all --check`
- [x] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
- [x] `cargo test -p timekeeper-frontend --lib layout`
- [x] locale 切替の focused host test を追加して green にする

### PR2: Core Pages

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
- [ ] `cargo test -p timekeeper-frontend --lib dashboard`
- [ ] `cargo test -p timekeeper-frontend --lib requests`
- [ ] `cargo test -p timekeeper-frontend --lib login`
- [ ] 表示文言 assertion を translation key 非依存へ更新した core page テストが green

### PR3: Admin Pages

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`
- [ ] `cargo test -p timekeeper-frontend --lib admin`
- [ ] `cargo test -p timekeeper-frontend --lib admin_users`
- [ ] `cargo test -p timekeeper-frontend --lib admin_audit_logs`
- [ ] `cargo test -p timekeeper-frontend --lib admin_export`

### PR4: Residual Cleanup

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test -p timekeeper-frontend --lib`
- [ ] `bash scripts/harness.sh lint`
- [ ] `rg '[ぁ-んァ-ヶ一-龠々ー]' frontend/src -g '*.rs'`
- [ ] `en` / `ja` 切替と `localStorage` 永続化を確認する focused host test が green

## Risks / Watchpoints

- `format!` と日本語助詞を含む文言は単純置換できず、interpolation key へ寄せる必要がある
- 既存 test が表示文言へ強く依存しているため、locale 導入で assertion の更新範囲が広がる
- backend 由来の自然言語エラーをそのまま表示している箇所は、今回だけでは完全な i18n にならない
- `rust-i18n` の macro 利用箇所は host test/SSR build でもコンパイル可能な置き方に揃える必要がある
- 英語 locale は自然な英語レビューを求めるため、単純直訳では完了にできない

## Suggested Delivery Split

### PR1: Foundation

- [x] `rust-i18n` 導入
- [x] locale files 新設
- [x] app root locale state / helper 追加
- [x] ブラウザ言語判定、`localStorage` 永続化、ヘッダー右上切替導線
- [x] `layout` / `confirm_dialog` / `error` の基盤移行
- [x] focused host test 追加

想定差分ファイル:
- [ ] `frontend/Cargo.toml`
- [ ] `frontend/locales/ja.yml`（新規）
- [ ] `frontend/locales/en.yml`（新規）
- [ ] `frontend/src/lib.rs`
- [ ] `frontend/src/main.rs`
- [ ] `frontend/src/router.rs`
- [ ] `frontend/src/components/layout.rs`
- [ ] `frontend/src/components/confirm_dialog.rs`
- [ ] `frontend/src/components/error.rs`
- [ ] `frontend/src/state/mod.rs`
- [ ] `frontend/src/utils/storage.rs`
- [ ] `frontend/src/test_support/mod.rs`
- [ ] `frontend/src/test_support/ssr.rs`
- [ ] `frontend/src/components/mod.rs`
- [ ] `frontend/src/pages/mod.rs`

波及候補:
- [ ] `frontend/src/config.rs`
- [ ] `frontend/src/state/config.rs`
- [ ] `frontend/src/state/theme.rs`
- [ ] `frontend/src/utils/mod.rs`
- [ ] `frontend/index.html`
- [ ] `frontend/index.html.template`

原則変更しないファイル:
- [ ] `frontend/src/pages/attendance/**/*`
- [ ] `frontend/src/pages/dashboard/**/*`
- [ ] `frontend/src/pages/requests/**/*`
- [ ] `frontend/src/pages/login/**/*`
- [ ] `frontend/src/pages/forgot_password/**/*`
- [ ] `frontend/src/pages/reset_password/**/*`
- [ ] `frontend/src/pages/mfa/**/*`
- [ ] `frontend/src/pages/settings/**/*`
- [ ] `frontend/src/pages/admin/**/*`
- [ ] `frontend/src/pages/admin_users/**/*`
- [ ] `frontend/src/pages/admin_audit_logs/**/*`
- [ ] `frontend/src/pages/admin_export/**/*`
- [ ] `frontend/src/api/attendance.rs`
- [ ] `frontend/src/api/requests.rs`
- [ ] `frontend/src/api/auth.rs`
- [ ] `frontend/src/api/audit_log.rs`
- [ ] `frontend/src/api/subject_requests.rs`
- [ ] `frontend/AGENTS.md`
- [ ] `docs/manual/**/*`

### PR2: Core Pages

- [ ] `forms` / `cards` の主要 shared component を移行
- [ ] attendance / dashboard / requests / auth / settings 移行
- [ ] status / confirmation 文言の key 化
- [ ] 表示文言 assertion の translation key 非依存化

想定差分ファイル:
- [ ] `frontend/src/components/forms.rs`
- [ ] `frontend/src/components/cards.rs`
- [ ] `frontend/src/state/attendance.rs`
- [ ] `frontend/src/api/client.rs`
- [ ] `frontend/src/api/tests.rs`
- [ ] `frontend/src/pages/attendance/panel.rs`
- [ ] `frontend/src/pages/attendance/view_model.rs`
- [ ] `frontend/src/pages/attendance/layout.rs`
- [ ] `frontend/src/pages/attendance/utils.rs`
- [ ] `frontend/src/pages/attendance/components/alerts.rs`
- [ ] `frontend/src/pages/attendance/components/form.rs`
- [ ] `frontend/src/pages/attendance/components/history.rs`
- [ ] `frontend/src/pages/attendance/components/summary.rs`
- [ ] `frontend/src/pages/dashboard/panel.rs`
- [ ] `frontend/src/pages/dashboard/view_model.rs`
- [ ] `frontend/src/pages/dashboard/layout.rs`
- [ ] `frontend/src/pages/dashboard/utils.rs`
- [ ] `frontend/src/pages/dashboard/components/activities.rs`
- [ ] `frontend/src/pages/dashboard/components/alerts.rs`
- [ ] `frontend/src/pages/dashboard/components/clock.rs`
- [ ] `frontend/src/pages/dashboard/components/global_filters.rs`
- [ ] `frontend/src/pages/dashboard/components/summary.rs`
- [ ] `frontend/src/pages/requests/panel.rs`
- [ ] `frontend/src/pages/requests/view_model.rs`
- [ ] `frontend/src/pages/requests/layout.rs`
- [ ] `frontend/src/pages/requests/utils.rs`
- [ ] `frontend/src/pages/requests/types.rs`
- [ ] `frontend/src/pages/requests/components/correction_form.rs`
- [ ] `frontend/src/pages/requests/components/detail_modal.rs`
- [ ] `frontend/src/pages/requests/components/filter.rs`
- [ ] `frontend/src/pages/requests/components/leave_form.rs`
- [ ] `frontend/src/pages/requests/components/list.rs`
- [ ] `frontend/src/pages/requests/components/overtime_form.rs`
- [ ] `frontend/src/pages/requests/components/status_label.rs`
- [ ] `frontend/src/pages/login/panel.rs`
- [ ] `frontend/src/pages/login/view_model.rs`
- [ ] `frontend/src/pages/login/utils.rs`
- [ ] `frontend/src/pages/login/components/form.rs`
- [ ] `frontend/src/pages/login/components/messages.rs`
- [ ] `frontend/src/pages/forgot_password/panel.rs`
- [ ] `frontend/src/pages/forgot_password/view_model.rs`
- [ ] `frontend/src/pages/reset_password/panel.rs`
- [ ] `frontend/src/pages/reset_password/view_model.rs`
- [ ] `frontend/src/pages/mfa/panel.rs`
- [ ] `frontend/src/pages/mfa/view_model.rs`
- [ ] `frontend/src/pages/mfa/utils.rs`
- [ ] `frontend/src/pages/mfa/components/setup.rs`
- [ ] `frontend/src/pages/mfa/components/verify.rs`
- [ ] `frontend/src/pages/settings/panel.rs`
- [ ] `frontend/src/pages/settings/view_model.rs`
- [ ] `frontend/src/pages/home.rs`

波及候補:
- [ ] `frontend/src/pages/attendance/repository.rs`
- [ ] `frontend/src/pages/dashboard/repository.rs`
- [ ] `frontend/src/pages/requests/repository.rs`
- [ ] `frontend/src/pages/login/repository.rs`
- [ ] `frontend/src/pages/forgot_password/repository.rs`
- [ ] `frontend/src/pages/reset_password/repository.rs`
- [ ] `frontend/src/pages/mfa/repository.rs`
- [ ] `frontend/src/pages/settings/repository.rs`
- [ ] `frontend/src/api/auth.rs`
- [ ] `frontend/src/api/attendance.rs`
- [ ] `frontend/src/api/requests.rs`
- [ ] `frontend/src/api/types.rs`

原則変更しないファイル:
- [ ] `frontend/src/pages/admin/**/*`
- [ ] `frontend/src/pages/admin_users/**/*`
- [ ] `frontend/src/pages/admin_audit_logs/**/*`
- [ ] `frontend/src/pages/admin_export/**/*`
- [ ] `frontend/src/api/audit_log.rs`
- [ ] `frontend/src/api/subject_requests.rs`
- [ ] `frontend/AGENTS.md`
- [ ] `docs/manual/**/*`
- [ ] `frontend/index.html`
- [ ] `frontend/index.html.template`

### PR3: Admin Pages

- [ ] admin / admin_users / admin_audit_logs / admin_export 移行
- [ ] admin 向け message / modal / feedback 文言の key 化
- [ ] backend 自然言語エラーの境界確認
- [ ] 英語文言の自然さレビュー

想定差分ファイル:
- [ ] `frontend/src/pages/admin/panel.rs`
- [ ] `frontend/src/pages/admin/view_model.rs`
- [ ] `frontend/src/pages/admin/layout.rs`
- [ ] `frontend/src/pages/admin/utils.rs`
- [ ] `frontend/src/pages/admin/components/attendance.rs`
- [ ] `frontend/src/pages/admin/components/holidays.rs`
- [ ] `frontend/src/pages/admin/components/requests.rs`
- [ ] `frontend/src/pages/admin/components/subject_requests.rs`
- [ ] `frontend/src/pages/admin/components/system_tools.rs`
- [ ] `frontend/src/pages/admin/components/user_select.rs`
- [ ] `frontend/src/pages/admin/components/weekly_holidays.rs`
- [ ] `frontend/src/pages/admin_users/panel.rs`
- [ ] `frontend/src/pages/admin_users/view_model.rs`
- [ ] `frontend/src/pages/admin_users/layout.rs`
- [ ] `frontend/src/pages/admin_users/utils.rs`
- [ ] `frontend/src/pages/admin_users/components/archived_detail.rs`
- [ ] `frontend/src/pages/admin_users/components/archived_list.rs`
- [ ] `frontend/src/pages/admin_users/components/detail.rs`
- [ ] `frontend/src/pages/admin_users/components/invite_form.rs`
- [ ] `frontend/src/pages/admin_users/components/list.rs`
- [ ] `frontend/src/pages/admin_audit_logs/panel.rs`
- [ ] `frontend/src/pages/admin_audit_logs/view_model.rs`
- [ ] `frontend/src/pages/admin_export/panel.rs`
- [ ] `frontend/src/pages/admin_export/view_model.rs`
- [ ] `frontend/src/api/audit_log.rs`
- [ ] `frontend/src/api/subject_requests.rs`
- [ ] `frontend/locales/en.yml`
- [ ] `frontend/locales/ja.yml`

波及候補:
- [ ] `frontend/src/pages/admin/repository.rs`
- [ ] `frontend/src/pages/admin_users/repository.rs`
- [ ] `frontend/src/pages/admin_export/repository.rs`
- [ ] `frontend/src/api/client.rs`
- [ ] `frontend/src/api/types.rs`
- [ ] `frontend/src/api/tests.rs`
- [ ] `frontend/src/components/empty_state.rs`
- [ ] `frontend/src/components/common.rs`

原則変更しないファイル:
- [ ] `frontend/src/pages/attendance/**/*`
- [ ] `frontend/src/pages/dashboard/**/*`
- [ ] `frontend/src/pages/requests/**/*`
- [ ] `frontend/src/pages/login/**/*`
- [ ] `frontend/src/pages/forgot_password/**/*`
- [ ] `frontend/src/pages/reset_password/**/*`
- [ ] `frontend/src/pages/mfa/**/*`
- [ ] `frontend/src/pages/settings/**/*`
- [ ] `frontend/src/components/forms.rs`
- [ ] `frontend/src/components/cards.rs`
- [ ] `frontend/AGENTS.md`
- [ ] `docs/manual/**/*`
- [ ] `frontend/index.html`
- [ ] `frontend/index.html.template`

### PR4: Residual Cleanup

- [ ] 残直書き scan の解消
- [ ] docs / AGENTS 更新
- [ ] 全体テストと lint
- [ ] final validation

想定差分ファイル:
- [ ] `docs/generated/exec-plans/active/EP-20260311-frontend-rust-i18n-migration.md`
- [ ] `frontend/AGENTS.md`
- [ ] `docs/manual/frontend-i18n-rule.md`（新規、命名は調整可）
- [ ] `frontend/locales/en.yml`
- [ ] `frontend/locales/ja.yml`
- [ ] residual scan で見つかった `frontend/src/**/*`

波及候補:
- [ ] `docs/index.md`
- [ ] `docs/design-docs/frontend-structure.md`
- [ ] `frontend/src/api/tests.rs`
- [ ] `frontend/src/test_support/**/*`

原則変更しないファイル:
- [ ] `frontend/Cargo.toml`（新規依存追加が残っていない限り）
- [ ] `frontend/src/main.rs`
- [ ] `frontend/src/lib.rs`
- [ ] `frontend/src/router.rs`
- [ ] `frontend/index.html`
- [ ] `frontend/index.html.template`
- [ ] `frontend/src/api/attendance.rs`
- [ ] `frontend/src/api/requests.rs`
- [ ] `frontend/src/api/auth.rs`
- [ ] `frontend/src/api/audit_log.rs`
- [ ] `frontend/src/api/subject_requests.rs`

## Merge Conflict Watchlist

複数 PR で再度触る可能性が高く、マージ競合を起こしやすいファイル:

- [ ] `frontend/locales/ja.yml`
- [ ] `frontend/locales/en.yml`
- [ ] `frontend/src/components/layout.rs`
- [ ] `frontend/src/api/client.rs`
- [ ] `frontend/src/api/tests.rs`
- [ ] `frontend/src/test_support/mod.rs`
- [ ] `frontend/src/test_support/ssr.rs`
- [ ] `frontend/AGENTS.md`
- [ ] `docs/generated/exec-plans/active/EP-20260311-frontend-rust-i18n-migration.md`

競合理由メモ:

- [ ] `frontend/locales/ja.yml` / `frontend/locales/en.yml`
  - 全 PR で key 追加が発生しやすい。key の並び順と階層構造を先に固定する
- [ ] `frontend/src/components/layout.rs`
  - PR1 で locale switcher、PR2/PR3 で nav や共通ラベルの翻訳が追加されやすい
- [ ] `frontend/src/api/client.rs`
  - core/admin の内部生成エラー文言を両 PR で key 化する可能性がある
- [ ] `frontend/src/api/tests.rs`
  - 文言 assertion の translation key 非依存化で複数 PR から更新が入りやすい
- [ ] `frontend/src/test_support/mod.rs` / `frontend/src/test_support/ssr.rs`
  - locale 初期化 helper や test harness 追加で共通基盤の追記が集中しやすい
- [ ] `frontend/AGENTS.md`
  - docs 変更の一部を先行 PR で触ると PR4 と競合しやすい。原則 PR4 まで温存する
- [ ] `docs/generated/exec-plans/active/EP-20260311-frontend-rust-i18n-migration.md`
  - 実施ログ更新が各 PR で重なりやすい。マージ時に最新状態へ手で整列する

競合を減らす運用:

- [ ] locale key は PR1 でトップレベル構造と並び順を先に定義する
- [ ] `layout.rs` の共通 nav 文言は PR1 でできるだけ先取りして移行する
- [ ] `api/client.rs` の内部生成エラー key 化は PR2 でまとめて行い、PR3 では新規追加を避ける
- [ ] `api/tests.rs` は PR2 で共通 assertion helper を整え、PR3 は追記中心にする
- [ ] docs / AGENTS の運用ルール更新は PR4 に寄せる

## Locale Structure

`frontend/locales/ja.yml` と `frontend/locales/en.yml` のトップレベル構造は次で固定する。

```yaml
app:
  name:
  locale:
    label:
    options:
      en:
      ja:
    actions:
    status:

common:
  actions:
  labels:
  states:
  status:
  feedback:
  validation:
  time:
  units:
  pagination:
  navigation:

components:
  layout:
  confirm_dialog:
  error:
  empty_state:
  cards:
  forms:
  guard:

pages:
  attendance:
  dashboard:
  requests:
  login:
  forgot_password:
  reset_password:
  mfa:
  settings:
  admin:
  admin_users:
  admin_audit_logs:
  admin_export:
  home:

admin_components:
  attendance:
  holidays:
  requests:
  subject_requests:
  system_tools:
  user_select:
  weekly_holidays:

api:
  errors:
  auth:
  attendance:
  requests:
  audit_log:
  subject_requests:

state:
  attendance:
  auth:
  config:

docs:
  i18n:
```

設計ルール:

- [ ] page 固有の表示文言は `pages.<page>.*` に置く
- [ ] shared component 固有の文言は `components.<component>.*` に置く
- [ ] admin 配下の部分コンポーネントは `admin_components.<name>.*` に置く
- [ ] 複数画面で再利用するボタン、ラベル、成功/失敗文言は `common.*` に寄せる
- [ ] frontend 内部生成の API エラー文言は `api.errors.*` に寄せる
- [ ] locale switcher 自体の文言は `app.locale.*` に置く
- [ ] 文言 key は `snake_case` ではなく階層 + `lower_snake_case` の leaf を使う
- [ ] date / weekday / unit format は今回 localize しないため、必要ならラベルだけ `common.time.*` / `common.units.*` に置く

key 配置の優先順:

1. [ ] `common.*` に置けるか確認する
2. [ ] shared component 専用なら `components.*`
3. [ ] page 専用なら `pages.*`
4. [ ] admin 部品専用なら `admin_components.*`
5. [ ] API / state 内部生成文言なら `api.*` / `state.*`

## JJ Snapshot Log

### PR1: Foundation

- [x] `jj status`
- [x] foundation 実装差分を確認
- [x] foundation validation pass
- [ ] `jj commit -m "feat(i18n): add frontend locale foundation"`

### PR2: Core Pages

- [ ] `jj status`
- [ ] core pages 実装差分を確認
- [ ] core pages validation pass
- [ ] `jj commit -m "feat(i18n): migrate frontend core pages copy"`

### PR3: Admin Pages

- [ ] `jj status`
- [ ] admin pages 実装差分を確認
- [ ] admin pages validation pass
- [ ] `jj commit -m "feat(i18n): migrate frontend admin copy"`

### PR4: Residual Cleanup

- [ ] `jj status`
- [ ] residual scan 結果を確認
- [ ] final validation pass
- [ ] `jj commit -m "docs(i18n): enforce translation-key workflow"`

## Progress Notes

- 2026-03-11: plan 作成。frontend に `rust-i18n` は未導入、locale ディレクトリ未作成、`frontend/src` 配下で日本語文字列を含む `.rs` は 74 ファイルであることを確認。
- 2026-03-11: GitHub issue #429 `feat(i18n): migrate frontend hardcoded copy to rust-i18n` を作成し、本 plan を登録。
- 2026-03-11: 実行前インタビューを反映。locale 切替はヘッダー右上、`localStorage` 永続化、初回はブラウザ言語判定で未一致時 `en` 既定、`en` / `ja` 完全切替、backend 自然言語エラーは対象外、translation key は階層型、docs は `docs/` + `frontend/AGENTS.md` 更新、PR は 4 分割で進める方針を確定。
- 2026-03-11: PR1 foundation を実装。`rust-i18n` 導入、`frontend/locales/{ja,en}.yml` 新設、`state::locale` によるブラウザ言語判定と `localStorage` 永続化、ヘッダー右上 locale switcher、`layout` / `confirm_dialog` / `error` の基盤翻訳化を追加。`cargo fmt --all --check`、`cargo clippy -p timekeeper-frontend --all-targets -- -D warnings`、`cargo test -p timekeeper-frontend --lib` を確認。
