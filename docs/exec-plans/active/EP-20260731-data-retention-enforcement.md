# EP-20260731-data-retention-enforcement

**Status:** Active — implementation not started

**Parent:** [EP-20260704-attendance-domain-gap-backlog](../completed/EP-20260704-attendance-domain-gap-backlog.md) G14 / T-19

**Design source of truth:** [data-retention.md](../../design-docs/data-retention.md)

## Goal

- 分類別の保存期間を設定・監査でき、満了データを安全かつ再実行可能な purge で処理できるようにする
- `delete` 型 subject request の承認を、単なる状態更新ではなく、法定保存データを保持したアクセス停止・PII 匿名化として DB transaction 内で atomic に確定する
- 保存期限前の物理削除、請求証跡の消失、残高・締め・給与 snapshot の参照整合性破壊を fail-closed で防ぐ

## Scope And Decisions

- In: 保存期間設定マスタ、legal hold、削除要求実行記録、匿名化 use case、session/token 失効、purge dry-run/once worker、既存 hard-delete への eligibility check、運用 runbook / harness / API contract
- Out: 法令判断そのもの、per-subject DEK/crypto-shredding、保存対象全テーブルの一括物理削除、backup の自動廃棄
- 法令由来の年数をコードへ散在させず、effective-dated の既定値付き設定として管理する。法定対象の期間短縮と legal hold 解除は単独管理者で確定させず、法務確認済み policy version と二者承認を要求して監査する
- Redis の best-effort queue は削除要求に使わない。承認、認証 epoch/revoked-at、匿名化、durable DB execution record を単一 transaction にし、全 auth path は DB 上の失効状態を fail-closed で確認する。Redis session cleanup は transactional outbox + retry とし、認可の正にはしない
- `subject_requests.user_id ON DELETE CASCADE` と新規勤怠テーブルの `ON DELETE RESTRICT` を先に解消し、請求・監査証跡を保持したまま匿名化できる schema にする
- unique 制約用 tombstone は削除専用 secret を用いた HMAC(user id, secret) 等の非推測値とする。元 email hash は残さず、暗号化 PII、MFA/recovery secret、password、active session/token を列別に NULL/ランダム失効する。ログ・監査 metadata に PII を含めない
- purge は分類ごとの起算点、未消滅 leave ledger、月次締め、給与 snapshot、legal hold、FK を確認し、判断不能なら削除しない

## TDD And Implementation

1. [ ] 分類・起算点・cutoff・`forever`・legal hold の pure test を RED にする
2. [ ] retention policy / legal hold / subject-request execution record の migration、repository、二者承認付き System Admin API を追加する
3. [ ] pending request を lock し、対象検証、認証 epoch/revoked-at、PII tombstone、実行記録、approve 更新を DB transaction 内で atomic に行う use case と、session cleanup outbox を実装する
4. [ ] 二重承認、自己削除、最後の System Admin、途中失敗、既に匿名化済みの再実行を安全に扱う
5. [ ] 分類別 dry-run/count と bounded batch purge を行う独立 worker binary（`--once`）を実装する
6. [ ] archived-user hard delete に共通 eligibility check を適用し、保存期間内・legal hold・参照不整合を拒否する
7. [ ] OpenAPI / backend API catalog / RUNBOOK / HARNESS と `retention-worker-once` stage を同期する

## Observable Acceptance

- [ ] `delete` request 承認 commit 後は Redis cleanup の成否にかかわらず全 auth path が本人のログイン・既存 session 継続を拒否し、直接識別子は復元不能な tombstone になる
- [ ] attendance、break、correction、request、leave ledger、closing、payroll snapshot、audit/consent/subject-request 証跡は保存条件を満たす間不変である
- [ ] approve の途中失敗では匿名化も status 更新も残らず、同一 request の並行・再実行でも効果は一度だけである
- [ ] purge は期限直前/ちょうど/超過、`forever`、legal hold、未消滅 ledger、FK 制約を正しく扱い、dry-run と実削除件数を監査できる
- [ ] 保存期限前の archived-user hard delete は拒否され、期限後も依存データの分類判断なしに cascade しない
- [ ] System Admin 以外は設定変更・手動実行できず、監査ログと worker log に PII が出ない
- [ ] 期間短縮・遡及適用・legal hold解除は policy version と二者の異なる承認者を要求し、単独操作と自己承認を拒否する

## Validation

- [ ] focused domain/app/repository tests
- [ ] `cargo test -p timekeeper-backend --test admin_subject_requests_api`
- [ ] retention / archived-user integration tests
- [ ] `bash scripts/harness.sh backend-security-smoke`
- [ ] live Postgres で `bash scripts/harness.sh retention-worker-once`
- [ ] `bash scripts/harness.sh docs-check`
- [ ] `bash scripts/harness.sh harness-contract`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

## Risks And Open Decisions

- 保存年限・起算点・legal hold の運用値は法務確認が必要であり、本 EP は法的助言を提供しない
- HMAC secret の保管・rotation と tombstone 再生成不能時の運用を secret-management runbook に定義する
- 物理削除対象は参照グラフを migration 単位で列挙し、第一増分は安全に分類できる archived/完了済みデータへ限定する
- backup、replica、外部 export に残る複製の削除 SLA は RUNBOOK で明示し、未定義のまま「完全削除済み」と応答しない

## Progress Notes

- 2026-07-31: 現行 approve が `subject_requests` の status 更新だけであること、既存 hard delete が新しい勤怠 FK と請求証跡保持を満たさないことを確認し、本 EP を登録した。
