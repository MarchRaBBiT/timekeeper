# EP-20260704-leave-entitlement-design

**親タスク:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-02（G2 設計）
**親 EP:** [EP-20260704-attendance-domain-gap-backlog](./EP-20260704-attendance-domain-gap-backlog.md)

## Goal

- 有給休暇の付与・残高台帳・消化引当・時効・年5日取得義務を「append-only ledger」として成立させる設計を確定し、後続実装（T-04 / T-05 / T-11 / T-13）の source of truth を作る
- 成果物は design doc [`docs/design-docs/leave-entitlement.md`](../../design-docs/leave-entitlement.md)（[work-schedule-master.md](../../design-docs/work-schedule-master.md) Follow-up Designs 3 の具体化）

## Scope

- In: `docs/design-docs/leave-entitlement.md` の新規作成、本 EP の新規作成
- Out（本 EP では行わない）:
  - 実装全般（migration・crates・handler・API・test）。実装は T-04 / T-05 / T-11 / T-13
  - 比例付与・出勤率 8 割判定・一律基準日の日数計算実装（拡張点の設計のみ）
  - 半休・時間単位年休・種別マスタの実装（T-11）
  - 代休台帳流用の実装（T-13。拡張余地の言及のみ）
  - `.agent/PLANS.md` / `attendance-domain-gap-tasks.md` / 他既存ファイルの編集（並列作業中の衝突防止）

## Done Criteria (Observable)

- [x] `docs/design-docs/leave-entitlement.md` が存在し、T-02 の全論点が「決定」として書かれている（付与ルール / ledger 5 イベント / FIFO・時効 2 年 / 年5日義務判定 / adjust 初期移行 / 数量=分 + `day_equivalent_minutes` / `Annual` 限定境界 / DDL スケッチ / T-04・T-05・T-11 参照項目 / T-13 代休流用）
- [x] design doc が `work-schedule-master.md` の Follow-up Designs 3 の具体化であることを相互リンクで明記している
- [x] `bash scripts/harness.sh docs-check` green
- [x] 本 EP が `.agent/PLANS.md` へ登録され、T-02 に EP リンクが追記されている

## Constraints / Non-goals

- 設計のみ。コード・migration・test を追加しない
- 法定値（付与日数・時効・年5日）はハードコードせず付与ルールマスタ / config として外部化する方針を書く（共通作業規約 8）
- 新規 use case は rebuild target 構成（`crates/domain` / `crates/app` / `crates/contract` / `crates/infra-postgres`）へ置く方針とし、現行 `backend/src/handlers/` は薄い配線に留める方針を書く
- 残高は保存せず read-model として都度導出する（settlement balance と同型）
- 既存ファイル（`.agent/PLANS.md`・タスクリスト・他 design doc）を編集しない。git commit / add は本タスクでは実行しない

## Task Breakdown

1. [x] T-02 の指示・親 EP G2・既存実装（`models/leave_request.rs`・`handlers/requests.rs`）・`work-schedule-master.md`・settlement balance EP を読み、矛盾しない設計方針を確定する
2. [x] `docs/design-docs/leave-entitlement.md` を新規作成し、全論点を決定として記述する（付与 / ledger / FIFO・時効 / 年5日 / 移行 / 単位 / 境界 / DDL / 後続参照 / T-13 拡張）
3. [x] 本 EP を `.agent/PLANS.md` テンプレート形式で新規作成する
4. [x] `bash scripts/harness.sh docs-check` を実行し green を確認する

## Validation Plan

- [x] `bash scripts/harness.sh docs-check`
- [x] fmt / clippy / test は対象外であることを確認（コード変更なし）

## Git Snapshot Log

- [x] `git status --short`（統合担当が実施）
- [x] `docs-check` pass
- [x] `git commit`（`5ec584b docs: add leave entitlement ledger design (T-02)`）

## Progress Notes

- 2026-07-31: `.agent/PLANS.md` / T-02 登録と commit `5ec584b` を再確認し、完了扱いとした。

- 2026-07-04: T-02（G2 設計）着手。design doc `leave-entitlement.md` を作成し、次を決定として明文化した — (1) 残高は保存しない導出値、正は append-only ledger（grant / consume / release / expire / adjust）、(2) 勤続年数テーブル付与（入社 6 ヶ月 10 日 → 20 日、法定値はマスタ外部化）、比例付与・出勤率 8 割判定は第一増分 Out で拡張点のみ、(3) FIFO（時効の近いロットから）消化・時効 2 年（遅延評価 + 冪等 expire 補記）、(4) 年5日義務は基準日 + 1 年 window の取得日数判定・時間単位年休は非算入・read API 導出、(5) 数量は分（`amount_minutes`）で保持しロット単位 `day_equivalent_minutes` で日⇔分換算、(6) `Annual` のみ残高連動・他種別は非連動で現行維持、(7) 既存残高は `adjust` でロット別に初期投入（dry-run 照合）、(8) DDL スケッチ（`leave_grant_rules` / `leave_ledger_entries`）と T-04 / T-05 / T-11 の参照項目、(9) T-13 代休は同一台帳へ `leave_type='compensatory'` として純ロジック流用。
- 2026-07-04: `bash scripts/harness.sh docs-check` green を確認。実装（T-04 以降）と `.agent/PLANS.md` 登録・タスクリストへの EP リンク追記は並列作業衝突防止のため本タスクの対象外とした。
