# EP-20260704-attendance-calculation-policy-design

## Goal

- [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-01（G1 設計）を完了する
- 労働時間の法令区分（所定内 / 法定内残業 / 法定外残業 / 深夜 / 法定休日労働）と丸め方針の定義を
  `docs/design-docs/attendance-calculation-policy.md` として確定し、T-03 以降の source of truth を作る

## Scope

- In: `docs/design-docs/attendance-calculation-policy.md` の新規作成、
  `docs/design-docs/work-schedule-master.md` Follow-up Designs 1 への相互リンク追記
- Out: T-03（日次区分 read-model）の実装、就業規則マスタの migration、丸めポリシーの具体実装、
  36協定閾値マスタ（T-08）、割増率・賃金額の計算

## Done Criteria (Observable)

- [x] design doc が存在し、T-01 の全論点（区分定義 / 深夜×`work_date` 帰属 / 週 40h / 夜勤・boundary 前打刻 /
      flex の役割分担 / 休暇・欠勤・休日出勤 / 丸め / パラメータ外部化）に採用案と不採用理由が書かれている
- [x] 既存決定（settlement-balance EP Design Decisions、work-schedule-master.md Time Semantics）と
      矛盾しない（実績 = effective 整数分 / 実休憩ベース / `work_date` 帰属 / 丸めなし / raw punch 不変を前提として明記）
- [x] `work-schedule-master.md` Follow-up Designs 1 に新 doc への相互リンクがある
- [x] `bash scripts/harness.sh docs-check` が green

## Constraints / Non-goals

- コード変更なし（docs のみ）
- 法令値はハードコードせず就業規則マスタの既定値とする方針を doc 内で確定する（実装は T-03）
- published version / locked projection の不変性、`ResolveWorkday` の挙動には触れない

## Task Breakdown

1. [x] 入力精読（settlement-balance EP Design Decisions 1–13、work-schedule-master.md Time Semantics /
       Resolution Rules / Weekly Pattern、`Attendance::calculate_work_hours`、migration 043–049）
2. [x] design doc 作成（Decision 1–12、Consumers 対応表、Required Test Matrix）
3. [x] work-schedule-master.md への相互リンク追記
4. [x] docs-check 実行

## Validation Plan

- [x] `bash scripts/harness.sh docs-check`
- コード変更なしのため fmt / clippy / test は対象外

## Git Snapshot Log

- [x] `git status --short`
- [x] `git commit`（`40e10b8 docs: add attendance calculation policy design (T-01)`）

## Progress Notes

- 2026-07-31: git 履歴と現行成果物を再確認し、統合 commit `40e10b8` を記録して完了扱いとした。

- 2026-07-04: T-01 実施。主要決定: (1) 基本区分 4 種の partition + 深夜 overlay の 2 軸構成、
  (2) 深夜は「測定 = 暦時刻交差、帰属 = `work_date` 全量」で夜勤の非分割と両立、
  (3) 週 40h は起算曜日設定値（既定日曜）・時系列累積・月跨ぎ週は隣接月データを読んで判定し
  応答は対象月のみ（どの月から見ても同一日の区分が一致する不変条件）、
  (4) 法定休日 = 就業規則マスタ指定曜日 × 非勤務 day_kind（動的判定は再現性を壊すため不採用）、
  (5) flex は日次・週次判定から除外し清算期間の法定総枠で判定（settlement balance と役割分担、
  計算不可条件を流用）、(6) 所定内/法定内の境界は分量比較（時間帯比較は給与実務と乖離するため不採用）、
  (7) 承認済み休暇は労働 0 分・別軸カウンタ、休暇日打刻は実績優先、
  (8) 丸めなし・導入時は read-model 再計算で互換、grace フィールドは T-09 判定専用、
  (9) 法定パラメータは effective-dated 就業規則マスタ（未設定期間は fail-closed、seed で既定値投入）
