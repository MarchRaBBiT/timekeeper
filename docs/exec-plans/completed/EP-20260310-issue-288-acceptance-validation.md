## ExecPlan: Issue #288 Acceptance Validation

## Goal

Issue #288 の未完了受け入れ基準 2 点を、再現可能な integration test と実測で確認し、issue / PR に反映する。

## Done Criteria

- [x] 存在ユーザーと非存在ユーザーのログイン失敗レイテンシ差を測る acceptance test が追加される
- [x] lockout 通知失敗が認証 API レイテンシに影響しないことを測る acceptance test が追加される
- [x] focused acceptance test が成功する
- [x] issue #288 の受け入れ基準が更新される
- [x] 検証成功後に `jj` snapshot を作成する

## Validation

- [x] `cargo test -p timekeeper-backend --test auth_flow_api login_failure_timing_distribution_overlaps_for_missing_and_existing_users -- --exact --nocapture`
- [x] `cargo test -p timekeeper-backend --test auth_lockout_redis_integration lockout_enqueue_latency_is_stable_even_when_worker_smtp_fails -- --exact --nocapture`
- [x] `cargo fmt --all`

## Progress Log

- [x] Issue / PR / current tests を確認
- [x] acceptance 測定用 test を実装
- [x] focused validation 実行
- [x] issue checklist 更新
- [x] snapshot 作成

## Notes

- `login_failure_timing_distribution_overlaps_for_missing_and_existing_users`:
  - existing=`[773, 764, 807, 791, 797, 781, 791, 783, 805, 787, 796, 773]`
  - missing=`[770, 781, 776, 797, 793, 796, 781, 770, 770, 784, 792, 781]`
  - median delta=`6ms`, p90 delta=`9ms`
- `lockout_enqueue_latency_is_stable_even_when_worker_smtp_fails`:
  - success path=`[819, 802, 802, 816, 793, 796]`
  - failure path=`[829, 797, 814, 785, 791, 795]`
  - median delta=`7ms`
