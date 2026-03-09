# Harness Engineering

## Intent

このリポジトリのハーネスは、OpenAI の Harness Engineering 記事
<https://openai.com/ja-JP/index/harness-engineering/>
の次のポイントを、ローカル開発向けに落とし込んだものとする。

- エージェントが迷わない短い入口を用意する
- repo 内に実行方法と前提条件を残す
- 高速で決定的な検証から、重い live 検証へ段階化する
- 失敗理由を環境不足と回帰に分離して返す

## Applied To This Repo

### Single entry point
- `scripts/harness.sh` を追加し、検証の入口を 1 つに寄せる

### Fast failure
- `doctor` stage がコマンド不足、Playwright 解決失敗、URL 未到達を先に返す
- live stage 実行前に環境不備で止められる

### Layered feedback loops
- `backend-unit`
  - コード変更直後の最短フィードバック
- `api-smoke`
  - backend が live で期待どおりかを確認
- `frontend-login`
  - UI 側の最小ログイン経路を確認
- `e2e`
  - 主要ユーザーフローをまとめて確認

### Reuse instead of rewrite
- 既存の `scripts/test_backend.sh`
- 既存の `scripts/test_backend_integrated.sh`
- 既存の `e2e/run.mjs`

を再利用し、ハーネス層は orchestration と preflight だけを担当する。

## Non-goals
- CI/CD の全面再設計
- テストケースの網羅性改善そのもの
- container 起動や browser install までの完全自動化

## Follow-up Candidates
- PowerShell 版ハーネスの追加
- JSON 形式の結果出力
- CI からの profile 呼び分け
- 起動スクリプトと連動した `up + smoke` プロファイル
