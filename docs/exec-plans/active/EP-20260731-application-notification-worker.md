# EP-20260731-application-notification-worker

**Status:** Active — implementation not started

**Parent:** [EP-20260704-attendance-domain-gap-backlog](../completed/EP-20260704-attendance-domain-gap-backlog.md) G12 / T-17

**Foundation:** [EP-20260704-notification-service-generalization](../completed/EP-20260704-notification-service-generalization.md)

## Goal

- `app:notifications` に enqueue 済みの申請提出・承認・却下 job を、再試行・重複排除・DLQ 付き worker で実際に配送する
- `missing_clock_out_reminder` を業務 timezone と勤務予定に基づいて検出し、同一 user/work date へ一度だけ enqueue する
- lockout 通知の既存 worker と application 通知の責務・queue・運用を分離する

## Scope And Decisions

- In: application queue consumer、request/reminder renderer、EmailService 配送、retry/DLQ/idempotency、missing-clock-out producer、常駐 binary と `--once`、RUNBOOK / harness / metrics
- Out: push/SMS/in-app inbox、通知テンプレート管理 UI、lockout worker の挙動変更、`missing_clock_in` / `absent` 通知
- queue payload は ID と event metadata のみに保ち、email、氏名、理由本文を格納しない。配送直前に DB から recipient と最新 event/scope/status を再検証する
- renderer は event kind、申請種別、状態、日時、固定導線だけを入力とし、申請理由・備考・管理者コメントを受け取らない
- consumer は lockout queue と混在させず、application 専用 ready/retry/DLQ と durable DB delivery ledger/outbox を持つ
- delivery key は kind + event + recipient とし、DB claim/attempt/provider message id/sent outcome を記録する。provider が idempotency key を提供する場合は同じ key を渡す。通常 SMTP では外部送信と DB commit を atomic にできないため at-least-once とし、送信成功後 crash の重複可能性を RUNBOOK と metric で明示する
- reminder producer は `APP_TIMEZONE`、resolved workday、承認済み休暇、clock-in/clock-out、猶予設定を参照し、複数 instance でも一度だけ enqueue する
- worker が green になった後に queue の暫定 silent `LTRIM` を廃止し、明示的な capacity/backpressure/alert 契約へ置き換える
- claim/ack/requeue は Redis transaction/Lua と DB delivery claim で crash-safe にし、lease renewal、bounded exponential backoff + jitter、poison job DLQ を実装する。DLQ の閲覧/replay/purge は System Admin に限定して監査し、IDを含むDLQ/log/metricsにも retention と access control を適用する

## TDD And Implementation

1. [ ] application queue の wire round-trip、dequeue、retry、DLQ、DB delivery claim/attempt/outcome の integration test を RED にする
2. [ ] request submitted/approved/rejected の recipient・event 再検証と locale-aware plain-text renderer を実装する
3. [ ] SMTP success/temporary failure/permanent invalid recipient、重複 job、消失 event、退職・匿名化 user の outcome を実装する
4. [ ] missing-clock-out scan use case を実装し、非勤務日・休暇日・flex・日跨ぎ・grace・DST・再 scan・並行 worker を固定する
5. [ ] `application_notification_worker` binary に常駐 loop、graceful shutdown、health/metrics、`--once` を追加する
6. [ ] RUNBOOK、HARNESS、backend API catalog と `application-worker-once` stage を同期し、暫定 queue trimming を置き換える
7. [ ] 先に queue depth/drop alert、次に consumer、最後に silent trim 撤去の順で rollout し、各段階の rollback 条件を実測する

## Observable Acceptance

- [ ] 申請提出は認可範囲の manager、承認・却下は申請者へ一度だけ配送される
- [ ] event の状態変化、scope 外、退職・匿名化 recipient、PII 復号不能を漏洩なく skip/retry/DLQ の明示 outcome にする
- [ ] 一時 SMTP failure は bounded retry 後に成功でき、上限超過は DLQ へ移る。claim 中の並行 consumer は二重送信せず、送信成功後 crash の曖昧な outcome は delivery ledger と metric から検出・照合できる
- [ ] reminder は clock-in 済みかつ clock-out 未完了の勤務日のみ対象とし、休暇・非勤務日を除外して user/work date ごとに一度だけ送る
- [ ] 常駐 worker と `--once` が同じ処理契約を使い、停止時に in-flight job を失わない
- [ ] queue 本文、application log、metric label、email subject に自由入力・email・氏名を露出しない
- [ ] renderer の型と回帰 test が申請理由・備考・decision comment を入力・本文・DLQへ流せないことを固定する
- [ ] lease 中断、claim後crash、retry移動中crash、graceful shutdown、DLQ replay の loss/duplicate 境界が integration test で固定される

## Validation

- [ ] notification queue/renderer/scan unit tests
- [ ] application notification Redis + SMTP integration tests
- [ ] existing request API and `work_schedule_phase2_api` tests remain green
- [ ] live Redis/Postgres で `bash scripts/harness.sh application-worker-once`
- [ ] `bash scripts/harness.sh backend-security-smoke`
- [ ] `bash scripts/harness.sh docs-check`
- [ ] `bash scripts/harness.sh harness-contract`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

## Progress Notes

- 2026-07-31: `app:notifications` producer と4種の job variant は存在するが consumer がなく、10,000件超を古い順に破棄する暫定策だけであることを確認した。application consumer と reminder producer を分離した本 EP を登録した。
