# Login / MFA Page Refactor Plan

`documents/pages-refactor-plan.md` の方針を踏まえ、`frontend/src/pages/login.rs` と `frontend/src/pages/mfa/` を以下の構造に再編する。

## 1. 目標

- ログインと MFA 専用のモジュールを導入し、フォーム/UI ロジックと API 呼び出しを分離する。
- 認証状態やメッセージ表示を再利用可能なコンポーネントへ切り出す。
- バリデーションやトークン処理をテスト可能な純粋関数に移行する。

## 2. ディレクトリ構成

```
frontend/src/pages/login/
    mod.rs          # エントリ＋認可ガード
    panel.rs        # LoginPanel (フォーム + 状態制御)
    components/
        form.rs     # ユーザー名/パスワードフォーム
        messages.rs # 成功・失敗メッセージ
    repository.rs   # login/refresh/logout API ラッパー
    utils.rs        # 入力検証・デバイスラベル生成など

frontend/src/pages/mfa/
    mod.rs
    panel.rs
    components/
        setup.rs
        verify.rs
    repository.rs   # MFA status/register/activate API
    utils.rs
```

## 3. 実装ステップ

1. **モジュール作成** ✅ (Login) / ✅ (MFA)  
   - `mod.rs` を追加し、既存の `login.rs` と `mfa.rs` を `panel.rs` へ移動。
   - ルーティングからは `pub use panel::LoginPage` のように再エクスポート。

2. **フォームとメッセージの分離** ✅ (Login) / ✅ (MFA)  
   - フォーム UI (`input`, `button`) を `components::form` に切り出し、`panel` は `FormState` の読み書きのみ担当。
   - 成功/失敗メッセージ、ローディング表示を `components::messages` へ集約。

3. **状態管理・API レイヤ** 🔄  
   - `create_action` を利用して `login`, `logout`, `refresh`, `mfa_register`, `mfa_activate` の各アクションを管理。
   - Login 側は `login/repository.rs` と `utils.rs` の導入完了。MFA 側も `repository.rs` / `components` への分離完了し、`state::auth` から利用中。

4. **ユーティリティとテスト** 🔄  
   - メール/パスワード/MFAコードのバリデーションを `utils.rs` に移動済み（Login/MFA）。今後 `#[cfg(test)]` の追加が必要。

5. **Guard/State 連携** ✅  
   - `state::auth` のログイン状態をコンテキストで共有し、`panel` は `set_auth` を呼び出すのみで更新されるよう整理済み。
   - `frontend/src/components/guard.rs` を `use_auth` 監視ベースへ置き換え、`loading` 完了後に未認証なら `/login` へ遷移する。

6. **テスト & ドキュメント** 🔄  
   - `cargo fmt`, `cargo test -p timekeeper-frontend --lib` で回帰確認。
   - `documents/pages-refactor-plan.md` に記載した進捗を更新。

## 4. 注意点

- アトリビュートによる Warning 抑制 (`allow(dead_code)` など) は禁止。未使用コードは削除または利用箇所を実装する。
- ログイン/MFA ページも UTF-8 / LF ルールを遵守する。

## 5. 進捗ログ

- 2025-11-19: LoginForm / MfaRegisterPanel を create_action ベースに置き換え、非同期通信の pending 状態とメッセージ反映を Action 完了イベントへ寄せた。
- 2025-11-19: `frontend/src/pages/login/utils.rs` と `frontend/src/pages/mfa/utils.rs` に wasm_bindgen_test のユニットテストを追加し、バリデーションエラーメッセージを UTF-8 の日本語に整理。
- 2025-11-19: `frontend/src/state/auth.rs` に `AuthProvider` を追加して Router 全体をコンテキストでラップし、`use_auth` から得られる状態がガード/ヘッダー/ログイン/MFA へ共通化されるよう統合。
- 2025-11-19: `frontend/src/components/guard.rs` を `use_auth` ベースに刷新し、認証状態が未確定の間は待機・未認証が確定したら `/login` へ自動遷移させるようにした。
- 2025-11-19: `state::auth` に `use_login_action` / `use_logout_action` / `use_refresh_action` を追加し、Header や LoginForm から `create_action` ベースで状態更新を統一できるようにした。
- 2025-11-19: MFA API 呼び出しを `state::auth` に統合し、`frontend/src/pages/mfa/repository.rs` を廃止して認証フロー経由に一本化。
- 2025-11-19: LoginPanel がフォーム状態とアクションを集中管理し、`LoginForm` は純粋な UI コンポーネントとしてシグナル/コールバックを受け取る構成へ整理。
- 2025-11-19: `frontend/src/api/client.rs` に wasm_bindgen_test を追加し、`ensure_device_label` / `decode_jti` / `current_access_jti` の挙動を検証できるようにした。
- 2025-11-19: `documents/pages-refactor-plan.md` の Login/MFA セクションを最新構成へ更新し、進捗と残課題を同期した。

## 6. TODO Checklist

- [x] `state::auth` から `logout` / `refresh` などのアクションを `create_action` 化し、Login/MFA 双方の API 呼び出し経路を一本化する。
- [x] `frontend/src/pages/mfa/repository.rs` を直接参照するコンポーネントを見直し、`state::auth` か専用フロー経由に段階的に寄せる。
- [x] `LoginPanel` / `MfaRegisterPanel` の追加状態（成功メッセージやログアウト連動など）をどう扱うか方針を固め、必要なら panel 層で責務を整理する。
- [x] `repository.rs` / トークン関連ユーティリティの `#[cfg(test)]` を整備し、API ラッパー層のロジックを回帰テストできるようにする。
- [x] `documents/pages-refactor-plan.md` の Login/MFA セクションへ最新の進捗と残課題を反映する。
