# EP-20260314-clickjacking-header-hardening

## Goal
- clickjacking 防御を meta CSP 依存から脱却し、HTTP レスポンスヘッダベースで確実に有効化する

## Scope
- In: `frontend/nginx.conf`, `frontend/index.html`, `frontend/index.html.template`, 必要に応じて container/runtime 配信設定
- Out: デザイン変更、CSP の全面再設計、外部 CDN の撤廃

## Done Criteria (Observable)
- [x] `frame-ancestors 'none'` もしくは同等防御が HTTP response header として配信される
- [x] `X-Frame-Options: DENY` などの defense-in-depth が追加されている、または不要と判断した理由が文書化されている
- [x] 既存フロント起動に必要なスクリプト読込を壊さずにブラウザ保護が成立する

## Constraints / Non-goals
- 現行の Leptos/WASM 起動方式は維持する
- local/dev でも動く現実的な CSP に留める
- 既存の外部フォント/CDN 依存は今回の主目的ではない

## Task Breakdown
1. [x] 現行 meta CSP のうち header 化すべきディレクティブを整理する
2. [x] nginx で CSP / anti-framing / 必要なら Referrer-Policy をヘッダ配信する
3. [x] `index.html` / template の meta CSP を最小化または削除し、二重管理を避ける
4. [x] dev/prod で壊れないことを focused 起動確認または smoke で確認する

## Validation Plan
- [x] frontend 配信設定の静的 review
- [x] `bash scripts/harness.sh fmt-check`
- [ ] `pwsh -File .\\scripts\\frontend.ps1 start` は今回未実行。container 配信で header 実測を優先し、dev script は非変更
- [x] `curl -I` 相当の header 確認
- [x] `bash scripts/harness.sh lint`

## Git Checkpoint Log
- [x] `git status --short`
- [x] header hardening validation pass
- [x] `git commit -m "fix(frontend): enforce header-based browser security policy"`

## Progress Notes
- 2026-03-14: clickjacking finding を単独で進められるよう、frontend 配信設定中心の ExecPlan として分離。
- 2026-03-14: PR #449 のマージ後、残件のうち唯一の High severity かつ frontend 配信設定に閉じた対応として次着手に選定し、`ongoing` へ移動。
- 2026-03-14: issue #450 を起票し、`fix/frontend-clickjacking-headers` ブランチで着手。
- 2026-03-14: `frontend/tests/browser_security_headers.rs` を追加し、`cargo test -p timekeeper-frontend --test browser_security_headers -- --nocapture` を現状実行して 3 件失敗を確認。
- 2026-03-14: `frontend/nginx.conf` に header-based CSP / `X-Frame-Options: DENY` / `Referrer-Policy` / `X-Content-Type-Options` を追加し、`/pkg/` location でも継承欠落を避けるため同じ security header を明示。
- 2026-03-14: `frontend/index.html` と `frontend/index.html.template` の meta CSP から `frame-ancestors` を除去し、anti-framing の source of truth を response header に寄せた。
- 2026-03-14: `frontend/Dockerfile` を更新し、`nginx.conf` も `index.html.template` と同じ `__CSP_CONNECT_SRC__` placeholder を build 時に置換するよう変更。
- 2026-03-14: 回帰テスト再実行で green を確認。加えて `bash scripts/harness.sh fmt-check` と `bash scripts/harness.sh lint` を通過。
- 2026-03-14: `podman build -f frontend/Dockerfile -t timekeeper-frontend-clickjacking-test .` の後、`podman run --add-host backend:127.0.0.1 -p 8443:443 ...` と `curl -k -I https://127.0.0.1:8443/` で `Content-Security-Policy` に `frame-ancestors 'none'`、`X-Frame-Options: DENY`、`Referrer-Policy: strict-origin-when-cross-origin`、`X-Content-Type-Options: nosniff` が返ることを確認。
- 2026-03-14: commit `0742c0f fix(frontend): enforce header-based browser security policy` を作成し、PR #451 `fix: enforce header-based clickjacking protection` を登録。
