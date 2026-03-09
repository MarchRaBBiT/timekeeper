# Harness Manual

このリポジトリでは、OpenAI の Harness Engineering 記事
<https://openai.com/ja-JP/index/harness-engineering/>
で示されている考え方に合わせて、エージェントが短い入口から段階的に検証を回せるようにしている。

## Goals
- 入口を 1 つに寄せる
- 重い検証の前に doctor で失敗要因を早く返す
- backend unit, live API smoke, frontend login, E2E を段階化する
- repo 内ドキュメントを system of record として残す

## Entry Point
- `bash scripts/harness.sh`

## Profiles
- `agent-fast`
  - `doctor + backend-unit`
  - コード変更後の最初の回帰確認向け
- `smoke`
  - `doctor + api-smoke + frontend-login`
  - 起動済みの backend/frontend に対する疎通確認向け
- `full`
  - `doctor + backend-unit + backend-integration + api-smoke + frontend-login + e2e`
  - ローカルでまとめて確認したいとき向け

## Stages
- `doctor`
  - 必要コマンド、Playwright 依存、live URL の到達性を確認する
- `backend-unit`
  - `cd backend && cargo test --lib`
- `backend-integration`
  - `./scripts/test_backend_integrated.sh`
- `api-smoke`
  - `./scripts/test_backend.sh`
- `frontend-login`
  - `node scripts/test_frontend_login.mjs`
- `e2e`
  - `cd e2e && node run.mjs`

## Examples
```bash
# 最短の回帰確認
bash scripts/harness.sh

# 起動済み環境への smoke
bash scripts/harness.sh smoke \
  --backend-url http://localhost:3000 \
  --frontend-url http://localhost:8000

# 必要な段だけ個別実行
bash scripts/harness.sh --stage doctor --stage api-smoke \
  --backend-url http://localhost:3000

# 失敗しても最後まで回して要約を見る
bash scripts/harness.sh full --continue-on-fail
```

## What Doctor Checks
- `cargo`, `curl`, `python`, `node`, `podman` など stage ごとの必要コマンド
- `e2e/` 配下から `playwright` が解決できること
- live stage を回す場合の backend `/api/docs` 到達性
- live stage を回す場合の frontend `/login` 到達性

## Current Boundary
- ハーネスは既存スクリプトを束ねるだけで、起動責務までは吸収しない
- backend/frontend を自動起動したい場合は既存の `scripts/backend.ps1` と `scripts/frontend.ps1` を使う
- CI 連携や永続的な結果保存はこの段階では対象外
