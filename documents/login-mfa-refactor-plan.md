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

1. **モジュール作成**  
   - `mod.rs` を追加し、既存の `login.rs` と `mfa.rs` を `panel.rs` へ移動。
   - ルーティングからは `pub use panel::LoginPage` のように再エクスポート。

2. **フォームとメッセージの分離**  
   - フォーム UI (`input`, `button`) を `components::form` に切り出し、`panel` は `FormState` の読み書きのみ担当。
   - 成功/失敗メッセージ、ローディング表示を `components::messages` へ集約。

3. **状態管理・API レイヤ**  
   - `create_action` を利用して `login`, `logout`, `refresh`, `mfa_register`, `mfa_activate` の各アクションを管理。
   - `repository.rs` に `fn login(request: LoginRequest)` などの async 関数を実装し、`ApiClient` の呼び出しを一箇所にまとめる。

4. **ユーティリティとテスト**  
   - メール/パスワード/MFAコードのバリデーションを `utils.rs` に移動し、`#[cfg(test)]` でケースを追加。
   - デバイスラベル生成やトークン処理は `utils` に切り出し、`auth.rs` からも再利用する。

5. **Guard/State 連携**  
   - `state::auth` のログイン成功時更新ロジックを再確認し、`panel` で `set_auth` を適切に呼び出す。
   - `RequireAuth` からのリダイレクトは既存のストレージヘルパー `utils::storage` を利用。

6. **テスト & ドキュメント**  
   - `cargo fmt`, `cargo test -p timekeeper-frontend --lib` で回帰確認。
   - `documents/pages-refactor-plan.md` に記載した進捗を更新。

## 4. 注意点

- アトリビュートによる Warning 抑制 (`allow(dead_code)` など) は禁止。未使用コードは削除または利用箇所を実装する。
- ログイン/MFA ページも UTF-8 / LF ルールを遵守する。
