# Harness Engineering

## Context

この repo は backend / frontend / live smoke / worker / Redis の seam が多く、agent が毎回 ad-hoc に検証コマンドを選ぶと、次の問題が起きやすいです。

- 成功条件が人によって変わる
- PR に「何をどこまで確認したか」が残らない
- live smoke と local test が混ざる
- `AGENTS.md` が肥大化して読まれなくなる

そのため、ハーネスを「巨大な手順書」ではなく、次の 3 層に分解します。

1. `AGENTS.md`
   - 入口
   - source of truth
   - validation ladder
2. `docs/manual/HARNESS.md`
   - 実行手順
   - stage / profile の意味
3. `scripts/harness.sh`
   - 実際に動く共通入口

## Design Goals

- agent が最初に読む情報量を減らす
- 完了条件を stage 名で共有する
- 小さな変更は小さな harness で閉じる
- issue / PR に残る検証語彙を統一する

## Why AGENTS-Centered

`AGENTS.md` は repo の「意思決定の入口」であり、ここに必要なのは全手順ではなく、以下だけです。

- 何を source of truth にするか
- どの順で検証するか
- 何が done か

長い具体手順まで `AGENTS.md` に抱え込むと、agent は必要な情報へ到達する前に context を消費します。  
そのため、`AGENTS.md` は短く保ち、詳細は manual と executable harness に逃がします。

## Validation Ladder

### `doctor`

環境差異を最初に切り分ける。

### `backend-unit`

ローカルコードのもっとも近い seam を確認する。

### `backend-integration`

DB / migration / repository を含む境界を確認する。

### `api-smoke`

live backend の wiring を確認する。
事前疎通確認には既存の public endpoint `GET /api/config/timezone` を使い、専用 `/health` endpoint は前提にしない。

### `frontend-login`

browser / routing / auth cookie / redirect の結線を確認する。

### `full`

PR 前の集約確認。

## Operational Rules

- stage は「何を検証したか」を表す安定語彙として扱う
- 新しい seam を追加したら、まず harness stage へマッピングする
- PR には raw command だけでなく stage 名も書く
- stage を回せない場合は、欠けている前提を明記する

## Non-Goals

- すべてのテストを 1 つの script に押し込むこと
- `AGENTS.md` だけで repo の全知識を表現すること
- flaky な performance benchmark を常時 gate にすること

## Expected Outcome

この構成により、agent は次のように振る舞えるようになります。

1. `AGENTS.md` を読む
2. 最小 stage を選ぶ
3. 実装する
4. `scripts/harness.sh` で確認する
5. PR / issue に同じ語彙で報告する

これがこの repo における harness engineering の最小単位です。
