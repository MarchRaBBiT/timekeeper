# EP-20260730-holiday-substitution-compensatory-leave

**Status:** 実装完了・検証済み  
**Parent task:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-13

## Goal

休日出勤の事前振替と事後代休を、申請・承認・workday反映・台帳付与・期限まで監査可能なworkflowとして実装する。

## Dependencies And Decisions

- T-11の休暇種別master contract確定後に実装する。
- migration `062` に休日出勤申請、benefit choice（`substitution / compensatory`）、振替先、状態、監査情報を置く。
- migration `063` に代休期限設定とledgerの休日出勤source参照を置き、source単位の一意制約で二重grantを防ぐ。
- 事前振替は元休日を勤務日、振替先勤務日を休日へ変える2つのoverrideを専用repositoryの1 transactionで保存する。既存の単日upsertを順番に呼ばない。
- 片側がlocked/closed、同日、期間外、不正なday-kindの場合は全体をrollbackする。
- 事前振替成立後の元休日は法定休日労働へ分類しない。事後代休では元の休日労働区分を維持する。
- 代休は`compensatory` ledger lotへ直接grantし、年5日義務・通常のannual付与ルール対象外とする。期限はeffective-dated設定から決定する。

## TDD And Implementation

1. workflow/domain testで選択肢、状態遷移、自己承認禁止、冪等性をREDにする。
2. repository integrationでoverride pairのcommit/rollback、locked月拒否、二重grant防止をREDにする。
3. migration 062–063とdomain/app/infraを実装する。
4. submit/list/approve/reject/cancel APIを部署scope認可と監査ログへ接続する。
5. substitution pairをclassification materializerへ反映し、compensatoryを既存FIFO/expire/consume/releaseへ接続する。
6. frontend申請・承認UIは親タスクT-13の完了条件外とし、必要時に別タスク化する。
7. OpenAPI、API catalog、attendance/leave設計を同期する。

## Observable Acceptance

- 振替承認で2つのoverrideが同時に作られ、片側失敗時はどちらも残らない。
- 振替元の休日労働分が0になり、代休方式では休日労働分が維持される。
- 代休が1回だけ付与され、消化・取消・期限切れが台帳へ反映される。
- scope外承認、自己承認、locked月変更、不正な組み合わせが拒否される。
- integration、contract、lintがgreen。

## Validation

- [x] domain/app RED → GREEN（contract 2、app 3）
- [x] migration/repository実装（062–063、pair transaction、source一意grant）
- [x] API contract / OpenAPI / catalog
- [x] repository integration（pair commit/rollback、locked拒否、grant/expiry/取消）
- [x] HTTP API integration（submit/list、swap/compensatory approve、approved cancel、reject、scope外/自己承認403、locked 409）
- [x] frontend申請・承認UIはT-13のAPI/integrationゴール外（既存申請UIへの追加は別タスク）
- [x] `bash scripts/harness.sh docs-check`
- [x] focused clippy / fmt
