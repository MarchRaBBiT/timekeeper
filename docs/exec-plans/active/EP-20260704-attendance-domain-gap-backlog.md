# EP-20260704-attendance-domain-gap-backlog

**Updated:** 2026-07-04
**Kind:** ドメイン機能ギャップの棚卸しバックログ（個別実装 EP の親）

## Goal

- 現行 Timekeeper が「勤怠管理システム」として不足している機能観点を実装調査ベースで棚卸しし、優先順位付きバックログとして固定する
- 各観点を後続の design doc / ExecPlan へ分解する際の親 EP として機能させる
- 既存計画（active EP、`work-schedule-master.md` Follow-up Designs、tech-debt-tracker）と重複しない形で「まだどこにも積まれていない不足」を明示する

## Scope

- In: 機能ドメイン（労働時間集計、休暇、申請、締め、通知、レポート、保存要件）の不足観点の列挙と優先順位付け、後続 EP への分解方針
- Out: 個別観点の詳細設計・実装（後続 EP で行う）、技術的負債（[tech-debt-tracker.md](../tech-debt-tracker.md) が管轄）、rebuild アーキテクチャ自体の計画（EP-20260610 系が管轄）

## Current Coverage（2026-07-04 実装調査の結果）

実装済み:

- 打刻（出退勤・休憩）、勤怠修正申請 + 承認（effective value 反映）
- 休暇申請（`annual` / `sick` / `personal` / `other`、日単位）・残業申請（date + planned_hours）+ 部署階層スコープの承認
- 勤務体系マスタ（fixed / flex + コアタイム）、resolved workday、日別 override、月次ロック（DB trigger）
- anomaly 検知（`schedule_not_configured` / `unscheduled_work` / 打刻漏れ）
- 祝日管理（公休・週次・ユーザー例外・Google import）、部署階層、監査ログ、PII 暗号化、consent / subject request
- 本人・管理者 CSV export、月次サマリ（`total_work_hours` / `total_work_days` / `average_daily_hours` のみ）

計画済み（本 EP では重複登録しない）:

- flex 清算期間残高: [EP-20260703-work-schedule-phase3-settlement-balance](./EP-20260703-work-schedule-phase3-settlement-balance.md)（未着手）
- [work-schedule-master.md](../../design-docs/work-schedule-master.md) Follow-up Designs: 1 勤怠計算ポリシー / 2 月次締め・承認・再締め / 3 有給休暇付与・残高台帳 / 4 給与エクスポート契約（design doc 予告のみで EP 未作成 → 本 EP の G1 / G2 / G8 / G10 が EP 化の受け皿）
- 代理承認者 / system_admin 自動エスカレーション: tech-debt #11 残（P2）
- 勤怠修正承認の管理 UI 配線: tech-debt #16

## Gap Inventory（優先順位付き）

### P1: 給与計算・法令遵守に直結する不足

**G1. 労働時間の法令区分集計と丸め規則（勤怠計算ポリシー本体）**

- 現状: 労働時間は `attendance.total_work_hours`（f64、実休憩控除後の総時間）1 本のみ。所定内 / 法定内残業 / 法定外残業 / 深夜（22:00–5:00）/ 法定休日労働の区分が存在せず、丸め規則も未定義
- 影響: 割増賃金計算の入力が作れず、給与連携（G10）・36協定監視（G3）の前提が欠ける
- 対応: Follow-up Designs 1 を design doc + EP 化。resolved workday（予定）と effective 実績の突合で日次区分分を導出する read-model として設計する

**G2. 年次有給休暇の付与・残高・取得義務管理**

- 現状: `LeaveType::Annual` の申請はできるが、付与（勤続年数・比例付与）、残高台帳、消化引当、時効（2年）、年5日取得義務の追跡が一切ない。残高ゼロでも申請・承認できる
- 影響: 労基法39条まわりの管理が台帳なしでは成立せず、承認者は残高を系外（紙・Excel）で確認する運用になる
- 対応: Follow-up Designs 3 を design doc + EP 化（付与ルールマスタ → 残高台帳 → 申請時の残高検証 → 年5日アラートの順で増分化）

**G3. 承認済み申請と勤怠実績の連動・突合**

- 現状: 承認済み休暇は attendance / 月次サマリ / CSV / resolved workday のいずれにも反映されない（`handlers/attendance.rs` は leave を参照しない）。承認済み残業申請（planned_hours）と実残業の突合もない。休暇承認日への出勤打刻も矛盾として検知されない
- 影響: 出勤簿上「休暇日」が単なる欠測日と区別できず、申請ワークフローが集計と分断されている
- 対応: 休暇承認 → resolved workday / 日次ステータスへの反映（休暇区分付き）を設計する EP、残業申請 vs 実績の乖離検知を anomaly へ足す EP に分解

**G4. 36協定上限の実績監視・アラート**

- 現状: 残業「申請」はあるが、実績ベースの時間外労働の月次・年次累計、上限（月45h / 年360h、特別条項、単月100h未満・2〜6ヶ月平均80h）への接近検知がない
- 影響: 上限超過が発生後にしか分からない。長時間労働者の把握（安衛法の医師面接指導ライン）もできない
- 依存: G1（法定外労働時間の定義）が前提
- 対応: G1 の区分集計の上に閾値監視 read API + 管理者向け一覧（G11 と統合可）を載せる EP

### P2: 運用品質・データ完全性の不足

**G5. 遅刻・早退・欠勤の判定**

- 現状: resolved workday に予定時刻はあるが、fixed schedule での遅刻・早退判定、および「予定勤務日なのに打刻も休暇承認もない日」（欠勤）の判定がない。anomaly は `schedule_not_configured` / `unscheduled_work` / 打刻漏れの 3 種のみ
- 対応: anomaly kind の拡張（`late` / `early_leave` / `absent`）+ 月次サマリへの件数追加。休暇連動（G3）と同時に設計しないと休暇日が欠勤誤検知になる点に注意

**G6. 休憩の法定下限チェック**

- 現状: 休憩は打刻ベースの実績のみで、労働時間 6h 超 45 分 / 8h 超 60 分の下限検証・警告がない
- 対応: 締め時・日次の warning（hard reject にはしない）として G1 の計算ポリシーに含めるか、独立 anomaly として追加

**G7. 半休・時間単位休暇と休暇種別マスタ**

- 現状: 休暇申請は日単位（start_date / end_date）のみ。半休・時間単位年休が表現できず、休暇種別もコード内 enum 4 種固定で会社固有の特別休暇（慶弔・夏季等）を追加できない
- 依存: G2（残高管理）と単位の整合が必要
- 対応: 休暇種別マスタ（有給/無給、残高連動有無）+ 取得単位（日/半日/時間）の design doc → EP

**G8. 月次締めの承認ワークフロー**

- 現状: `work-schedule-closures/monthly` による resolved workday の lock はあるが、「本人確認 → 上長承認 → 締め確定 → 再締め」の状態遷移・承認記録がない。締めは system_admin の一方的な lock 操作
- 対応: Follow-up Designs 2 を design doc + EP 化。締め状態（open / self-confirmed / approved / closed）と再締め監査証跡を定義する

**G9. 振替休日・代休の管理**

- 現状: 休日出勤は `unscheduled_work` として打刻可能だが、振替休日（事前振替で割増なし）と代休（事後付与）の付与・消化・期限管理がない
- 依存: G1（休日労働の区分）、G2（残高台帳の器）
- 対応: 休日出勤申請 + 振休/代休付与のワークフロー EP として G2 の後に積む

**G10. 給与エクスポート契約**

- 現状: export は打刻明細の raw CSV のみ。給与システムが必要とする「従業員 × 月 × 賃金項目（所定内/残業/深夜/休日/欠勤控除…）」の確定値出力がない
- 依存: G1、G8（締め確定値の固定）
- 対応: Follow-up Designs 4 を design doc + EP 化

**G11. 管理者向け月次レポート / ダッシュボード**

- 現状: 月次サマリ API は本人用のみ。管理者が全従業員・部署別の月次集計、長時間労働者一覧、未処理 anomaly 件数を一覧する手段が raw CSV 以外にない
- 対応: 部署スコープ付きの月次集計 read API + `/admin` セクション追加 EP（G4 の閾値監視と統合してよい）

**G12. 申請・打刻イベントの通知**

- 現状: 申請提出・承認・却下・打刻漏れのいずれも通知されない（メール基盤と queue/worker 実装は lockout 通知専用に存在）
- 対応: lockout notification queue/worker を汎用 notification service へ一般化する EP。rebuild の `apps/worker` 設計（tech-debt #7）と統合して計画する

### P3: 中長期・要方針判断

**G13. 勤務間インターバルチェック**

- 現状: 前日の退勤〜当日の出勤の間隔（努力義務 11h 目安）の検証がない
- 対応: G5 と同型の anomaly として追加可能。優先度は運用要望が出た時点で引き上げる

**G14. 勤怠記録の法定保存期間と削除要求の整合**

- 現状: 出勤簿・賃金台帳相当データの保存期間（労基法 109 条: 5年、当分の間 3年）に対する retention policy が未定義。subject request（削除要求）承認と法定保存義務の優先関係も未整理
- 対応: retention policy design doc（アーカイブ / 物理削除の基準、subject request 処理時の例外規定）を作成する EP

**G15. 打刻手段の拡張（IC カード・生体・位置情報）**

- 現状: Web UI 打刻のみ
- 判断: 当面 explicit out of scope とする。要件が出た時点で本バックログへ再登録する

## Done Criteria (Observable)

- [x] 不足観点が実装調査（API catalog / migrations / handlers / 既存 EP・design doc）に基づき優先度付きで列挙されている
- [x] 既存計画（settlement-balance EP、Follow-up Designs 1–4、tech-debt #7/#11/#16）との対応関係が明示され、二重登録がない
- [ ] P1 の各観点（G1–G4）に対応する design doc または個別 EP が作成されている
- [ ] P2 の各観点が、着手時に本 EP を起点として個別 EP 化されている（着手順は下記 Suggested Order）

## Constraints / Non-goals

- 本 EP 自体はコード変更を行わない（docs のみ）
- 法令要件の記述は設計判断の背景であり、法的助言ではない。就業規則・36協定の実値はマスタ設定として外部化する前提で設計する
- rebuild mode 進行中のため、新規 use case は `crates/app` / `crates/contract` / `crates/infra-postgres` の rebuild target 構成で設計する（現行 `backend/src/handlers` へ大きなロジックを足さない）

## Task Breakdown

実装タスクへの分解は [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md)（T-01〜T-19、担当エージェント向け指示付き）を source of truth とする。

1. [x] 現状カバレッジ調査（backend-api-catalog / migrations 001–049 / handlers / services / active EP / Follow-up Designs / tech-debt-tracker）
2. [x] gap inventory の文書化・優先順位付け（本ファイル）
3. [x] 実装タスクリストの作成（前提・目的・ゴール・次タスク付き。G1→T-01/T-03、G2→T-02/T-04/T-05、G3→T-06/T-07、G4→T-08、G5→T-09、G6→T-10、G7→T-11、G8→T-12、G9→T-13、G10→T-14、G11→T-15、G12→T-16/T-17、G13→T-18、G14→T-19）
4. [x] Phase 0（T-01, T-02, T-16, T-19）の着手・個別 EP 化（2026-07-04 完了。commits 40e10b8 / 5ec584b / 8d5101a / 92ca21a）
5. [ ] Phase 1（T-03, T-04, T-06）の着手・個別 EP 化
6. [x] Phase 2（T-05, T-07, T-08, T-09, T-10, T-12, T-17）の着手・個別 EP 化
7. [ ] Phase 3（T-11, T-13, T-14, T-15, T-18）の着手・個別 EP 化、G15 の再評価

## Validation Plan

- [x] `bash scripts/harness.sh docs-check`（2026-07-04 pass）
- コード変更なしのため fmt / clippy / test は対象外

## Git Snapshot Log

- [x] `git status --short`
- [x] `docs-check` pass
- [x] `git commit -m "docs: register attendance domain gap backlog exec plan"`

## Progress Notes

- 2026-07-04: 実装調査（有給残高・区分集計・遅刻早退判定・申請実績連動・通知・締めワークフローの不在を grep / API catalog / migration 一覧で確認）に基づき G1–G15 を登録。P1 = G1–G4、P2 = G5–G12、P3 = G13–G15 とした
- 2026-07-04: G1–G14 を実装タスク T-01〜T-19 へ分解し、担当エージェント向け指示（前提タスク・目的・ゴール・次タスク・共通作業規約）を [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) として登録。依存関係を Phase 0–3 に整理（G15 はタスク化せず要件発生時に再登録）
- 2026-07-04: Phase 0 を並列実行。T-01（勤怠計算ポリシー design doc、40e10b8）、T-02（有給台帳 design doc、5ec584b）、T-19（保存期間 policy design doc、8d5101a）完了。相互整合レビュー済み（休暇日 = 労働 0 分・別軸カウンタで T-01/T-02 一致、settlement-balance 既存決定と無矛盾）。T-16（通知基盤汎用化）は実装進行中
- 2026-07-04: T-16 完了（92ca21a）。レビューで検出した「デプロイ跨ぎの legacy 形式 in-flight job 消失」エッジを fallback decode + 互換テスト 3 件で修正済み。Phase 0 全 4 タスク完了。次は Phase 1（T-03 / T-04 / T-06）
- 2026-07-09: Phase 1 実装（b39ed35 / 05f45e9 / a2b1294 / 25104c0）の多観点レビュー（ドメイン正当性 / DB / セキュリティ / 契約同期）を実施。HIGH 3 件を検出・修正: 有給消化の暦日→稼働日ベース化（144ec32、leave-entitlement.md に Consumption Target Days を追記）、RunLeaveGrants/AdjustLeaveLedger の TOCTOU 修正 + 付与バッチのユーザー単位 tx 分離・二重起動排他（457281d、migration 055）、管理系ミューテーション 3 本の監査ログ登録（10b60d0）。MEDIUM 群も修正: adjust 入力の i32 範囲 validate + checked_add、更新時残高再検証、migration 056 の adjust CHECK、エラー変換一本化、admin export の既定期間 + 366 日上限 + user_id マージキー化（c7116c5）。leave_ledger.rs は use case 単位に分割（42c2074、mod/balance/consume/grants/adjust）。区分計算（T-03）は design doc との不一致なし。残課題: 承認済み休暇 read 表示と稼働日消化の非対称は仕様として明文化済み、RunLeaveGrantsReport.failed の API 露出は契約変更を伴うため未対応
- 2026-07-09: Phase 2 残タスクを実装。T-07（残業申請突合 anomaly）、T-08（36協定監視 API/settings）、T-09/T-10（遅刻・早退・欠勤・休憩不足 anomaly）、T-12（月次締め workflow + `monthly-closing.md`）、T-17（申請提出/承認/却下 notification queue enqueue）を個別 EP 化して登録。`cargo test -p timekeeper-backend --test work_schedule_phase2_api -- --nocapture` は 18 passed。T-17 の missing clock-out reminder は notification enum variant として予約済みで、定期 scan/worker delivery は次の通知 worker 拡張に委ねる
