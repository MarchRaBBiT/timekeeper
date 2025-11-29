# フロントエンド コードレビュー指摘（2025-11-29）

## 概要
フロントエンドコードのレビューで発見した不具合を共有します。再現条件と推奨対処を明記し、修正計画策定のインプットとします。

## 指摘事項

### 1. API ベース URL が誤ってキャッシュされる
- **場所**: `frontend/src/lib.rs` (`config::init()` 起動タイミング), `frontend/src/state/auth.rs` → `config::await_api_base_url()`, `frontend/src/config.rs::await_api_base_url`
- **内容**: `config::init()` を非同期で起動する前に `AuthProvider` が `ApiClient` を呼び、`await_api_base_url()` がデフォルト (`http://localhost:3000/api`) を `OnceLock` に確定させる。`env.js` や `config.json` の読み込みより先に初回アクセスが走ると、本番ホストが無視され、以後の API 呼び出しが常に誤った URL を使用する。
- **影響**: 非ローカル環境で API 接続がすべて失敗し、ログインを含む機能が動作しない。
- **対応案**:
  1. `config::init()` 完了まで認証チェックや API 呼び出しを開始しない（起動シーケンスを直列化）。
  2. もしくは `await_api_base_url()` に上書きパスを追加し、`config::init()` 完了時に `OnceLock` を更新できる設計に変更する。

### 2. 認証ガードが未認証でも子ビューを描画する
- **場所**: `frontend/src/components/guard.rs::RequireAuth`
- **内容**: リダイレクト用の `create_effect` のみでビュー遮断を行っておらず、`loading` false かつ `is_authenticated` false でも子コンポーネントが描画される。結果として未認証状態でも管理画面 UI が一瞬表示され、子コンポーネントの API フェッチが未認証で走る。
- **影響**: UI フラッシュと不要な API 呼び出しが発生し、ログやユーザー体験が悪化する。
- **対応案**:
  1. `state.loading` 中はスケルトン/スピナーのみを返す。
  2. `is_authenticated` が false なら子を描画せずフォールバック（例: ログインへのリンクや空）を返す。
  3. 必要ならガード内でフェッチを抑止するため、子側の初期フェッチを `on:mounted` など遅延トリガーにする。

## 次ステップ案
- ベース URL 初期化のシーケンス変更案を検討し、影響範囲（AuthProvider/ApiClient/config 初期化順序）を整理。
- `RequireAuth` の描画ロジック改善方針（フォールバック UI とフェッチ抑止方法）を決定し、影響ページを列挙。
