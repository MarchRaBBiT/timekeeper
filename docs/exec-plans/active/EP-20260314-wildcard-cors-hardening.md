# EP-20260314-wildcard-cors-hardening

## Goal
- credentials 付き CORS と origin 検証を fail-closed にし、wildcard 設定の危険な抜け道をなくす

## Scope
- In: `backend/src/main.rs`, `backend/src/config.rs`, `backend/src/utils/security.rs`, `backend/src/middleware/csrf.rs` の変更 seam（挙動は変えないが影響を受ける）, 関連 config / startup / security / csrf tests, 必要に応じて docs
- Out: CORS を使わない構成への全面移行、infra レベルの ALB/CDN 設定変更

## Current State（Job1 調査結果、2026-07-03）

### CORS 初期化の現行構造

`backend/src/main.rs`:
- `log_config()`（L600-623）: `CORS Allowed Origins: {:?}` を常にログ出力する（L615）。wildcard `"*"` を検出した場合、`config.production_mode == true` のときのみ `panic!` で起動拒否する（L617-621）。`production_mode == false` の場合は `tracing::warn!` のみで起動を続行する（L622）
- `cors_layer()`（L626-650付近）: `allow_credentials(true)` を常に設定（L633付近）。`cors_allow_origins` に `"*"` が含まれる場合は `AllowOrigin::predicate(|_, _| true)`（全 origin 許可、L639-640）、それ以外は明示 allowlist（`Vec<HeaderValue>`）を使う。**このレイヤーには `production_mode` チェックが無い** — `log_config` の panic をすり抜けた non-production wildcard 構成は `allow_credentials(true)` + 全 origin 許可のまま動く

→ 危険な組み合わせ（`allow_credentials(true)` + wildcard origin）は `PRODUCTION_MODE` の値に完全に依存して許容されており、これが EP の Done Criteria 3（PRODUCTION_MODE非依存）が未達成である直接の原因。

### config.rs の現行構造

`backend/src/config.rs`:
- `Config::load()`（L79〜）は `anyhow::Result<Self>` を返せる構造だが、`cors_allow_origins`（L150-155）は `CORS_ALLOW_ORIGINS` 環境変数を単純 split するだけで、値の検証（wildcard 拒否等）を一切行わない。default は `http://localhost:8000`

**validation 配置の設計判断（Job2/3/4 が従うべき方針）**: `Config::load()` 内でのフィールド構築時点で `credentials 前提 + wildcard` を検出し `anyhow::Result` の `Err` として返す方式を推奨する。理由: (1) 起動処理全体が `Config::load()?` の失敗で即座に停止する既存パターンと一致する、(2) `log_config`/`cors_layer` の2箇所に分散した現状のロジックを一本化できる、(3) `production_mode` に依存させないという Done Criteria をコンストラクタ内で機械的に保証できる。`log_config` の panic 分岐と `cors_layer` の wildcard predicate 分岐は、`Config::load()` が失敗を返すようになった時点で「バリデーション済みの値のみ到達する」防御的コードとして残すか削除するかを Job3 で判断する。

### `verify_request_origin` の wildcard 許容分岐

`backend/src/utils/security.rs`（L18-38）:
```rust
pub fn verify_request_origin(headers: &HeaderMap, config: &Config) -> Result<(), AppError> {
    let origin_str = match extract_request_origin(headers) { ... };
    if config.cors_allow_origins.iter().any(|o| o == "*" || o == origin_str.trim_end_matches('/')) {
        Ok(())
    } else {
        Err(AppError::Forbidden("Invalid Origin or Referer".into()))
    }
}
```
L32 の `o == "*"` が wildcard 許容分岐そのもの。ここを除去し allowlist の完全一致のみにするのが Task 3 の実装対象。

### 【重要】CSRF middleware との関係

`verify_request_origin` は CORS ヘルパーであると同時に、`backend/src/middleware/csrf.rs`（`csrf_check`、L18-44）の CSRF 防御の中核である。同ファイルの doc comment（L16-17）に「All other state-changing requests must have a valid `Origin` or `Referer` header matching the configured `CORS_ALLOW_ORIGINS`」と明記されている。`csrf_check` は GET/HEAD/OPTIONS/TRACE と `Authorization: Bearer` 付きリクエストをスキップし、それ以外の state-changing request（cookie 認証の POST/PUT/PATCH/DELETE）でのみ `verify_request_origin` を呼ぶ（L40）。

**→ Task 3/4 での `verify_request_origin` の変更は CORS 設定だけでなく CSRF 防御そのものに影響する。** 現状 `CORS_ALLOW_ORIGINS="*"` が設定されていると、CSRF 検証も無条件で通過してしまう（wildcard は CORS だけでなく CSRF の allowlist も無効化している）ことを意味する。Task 3 は `backend/tests/csrf_protection_api.rs` に wildcard 設定時の cookie-mutation 拒否ケースを追加する必要がある（すでに Task 3 の description に反映済み）。

`docs/design-docs/backend-api-catalog.md` の L33-37 にも CSRF 契約の記載を確認したが、`CORS_ALLOW_ORIGINS` とのこの結合関係までは明記されていない。Job で docs 更新が必要になった場合はこの結合を明記する。

### 既存テストの反転・調整対象

1. **`backend/src/utils/security.rs::verify_origin_success_wildcard`**（L157-163）: 現在 `cors_allow_origins = ["*"]` で `verify_request_origin` が `Ok` になることを固定している。wildcard 拒否実装後は逆（`Err`）になるべきテスト。**RED化・反転対象**
2. **`backend/src/main.rs::test_production_mode_wildcard_cors_panics`**（L792-800）: `production_mode = true` + wildcard で `panic!` を期待。validation を `Config::load()` に移す場合、`log_config` の panic 自体が不要になる可能性があり、テストの対象関数が変わる
3. **`backend/src/main.rs::test_production_mode_specific_cors_allows`**（L802-807）: 明示 origin では production でも通ることを固定。回帰として維持する
4. **`backend/src/main.rs::test_log_config_with_read_database_and_wildcard_in_non_production`**（L871-877）: `production_mode = false` + wildcard で `log_config` が(panicせず)成功することを期待している。**PRODUCTION_MODE非依存化の実装後は反転対象**（non-productionでもwildcardは拒否されるべき）
5. **`backend/src/main.rs::test_app_router_builds`**（L809-828）・**`test_user_admin_and_system_routes_require_auth`**（L830-869）: `test_config(vec!["*".to_string()])` を単なる「動くconfigのfixture」として流用しているだけで、CORS/wildcard自体の検証が目的ではない（`cors_layer(&config)` を呼ばず、`Router::new().merge(...)` を直接組み立てている。`cors_layer` は L114 の実アプリ構築経路でのみ呼ばれる）。**validation を `Config::load()` に置く場合、これらのテストは `Config` 構造体リテラルを直接構築しており `Config::load()` を経由しないため、実装次第では影響を受けない可能性が高い**。ただし fixture の wildcard 使用が紛らわしいため、Task 3/4 で明示的な origin（例: `http://localhost:8000`）へ差し替えるかどうかを判断する

### 破壊的変更リスク（本番設定値の調査結果）

`SETUP_GUIDE.md` L306-317（本番環境変数の例）を確認した結果、`CORS_ALLOW_ORIGINS=https://your-frontend.example.com` という**具体的な origin のプレースホルダー**が記載されており、wildcard `*` の使用は運用ドキュメント上どこにも見当たらない（`rg`で repo 全体・SETUP_GUIDE.md・docs/を検索）。

**結論: wildcard 依存の本番デプロイ構成が存在する証跡はない。** 既存の運用ガイドはすでに明示 origin を前提としているため、本 EP の変更が正当な本番運用を壊すリスクは低いと判断する。ただし実際の本番環境変数値そのもの（デプロイ先の実際の `.env` や secret store の値）はこの調査の範囲外であり confirm できていないため、Risks に残す。

## Risks / Watchpoints

- 本番環境の実際の `CORS_ALLOW_ORIGINS` 値（deploy先のsecret/.env）はこの調査で直接確認できていない。SETUP_GUIDE.md 上のガイダンスは明示 origin 前提だが、実際の運用値が wildcard になっていないかは実装・デプロイ担当者が別途確認すること
- `verify_request_origin` の変更は CSRF 防御の実効性に直結する。CORS の観点だけでテストを固定すると CSRF 側の回帰を見逃す
- `test_app_router_builds` 等の無関係テストが `test_config(vec!["*"])` を fixture として使っているため、validation の実装場所によっては予期せず巻き込まれる可能性がある

## Constraints / Non-goals
- local 開発で必要な cross-origin は明示 allowlist で成立させる
- 既存の正当な origin allowlist は壊さない
- CORS 緩和を必要とする将来要件があれば、明示設計として別計画に切り出す

## Task Breakdown
1. [x] 現行 CORS 初期化と `verify_request_origin` の wildcard 分岐を整理する（Job1、本ファイルの Current State 節に記録済み）
2. [ ] credentials + wildcard を常時拒否する validation を config/startup に追加する（推奨: `Config::load()` 内、詳細は Current State の設計判断を参照）
3. [ ] `verify_request_origin` から wildcard 許容を除去し、allowlist のみ許可する
4. [ ] config / startup / security helper / CSRF middleware の focused test を追加する（既存テストの反転・調整は Current State の「既存テストの反転・調整対象」1・4を参照）

## Validation Plan
- [ ] `bash scripts/harness.sh fmt-check`
- [ ] `cargo test -p timekeeper-backend --test config_api -- --nocapture`
- [ ] `cargo test -p timekeeper-backend --lib security`
- [ ] `cargo test -p timekeeper-backend --lib` （main.rs 内の startup/cors テスト群を含む）
- [ ] `cargo test -p timekeeper-backend --test csrf_protection_api`
- [ ] `bash scripts/harness.sh backend-unit`
- [ ] `bash scripts/harness.sh lint`

## Git Checkpoint Log
- [ ] `git status --short`
- [ ] CORS/config focused tests pass
- [ ] `git commit -m "fix(security): fail closed on wildcard cors with credentials"`

## Progress Notes
- 2026-03-14: wildcard CORS finding を config / startup hardening の独立 ExecPlan として作成。
- 2026-07-03: Job1（現状調査）を実施し、`docs/exec-plans/backlog/20260314-wildcard-cors-hardening.md` から `docs/exec-plans/active/EP-20260314-wildcard-cors-hardening.md` へ移動した（active の命名規則 `EP-YYYYMMDD-slug.md` に合わせて改名）。
  - CORS初期化が `log_config`（production_mode時のみpanic）と `cors_layer`（production_mode非依存でwildcard許可）の2箇所に分散し、これが Done Criteria「PRODUCTION_MODE非依存」の未達成原因であることを確認した
  - validation 配置は `Config::load()` への集約を推奨案として記録した
  - `verify_request_origin` が CSRF middleware（`csrf_check`）の中核であり、CORS wildcard設定がCSRF allowlistも無効化してしまう結合関係を確認した。Task 4 に CSRF middleware 経由のテストを追加する必要がある
  - 反転・調整が必要な既存テスト5件（`verify_origin_success_wildcard`、`test_production_mode_wildcard_cors_panics`、`test_production_mode_specific_cors_allows`、`test_log_config_with_read_database_and_wildcard_in_non_production`、および `test_config(vec!["*"])` を無関係目的で流用している2テスト）を列挙した
  - `SETUP_GUIDE.md` の本番環境変数ガイダンスが明示originを前提としており、wildcard依存の本番デプロイ証跡が無いことを確認した（ただし実際のデプロイ環境変数値は未確認としてRisksに残した）
