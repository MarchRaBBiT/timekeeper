# Attendance Domain Gap — 実装タスクリスト

**Updated:** 2026-07-04
**親 EP:** [EP-20260704-attendance-domain-gap-backlog](./active/EP-20260704-attendance-domain-gap-backlog.md)
**Purpose:** 親 EP で棚卸しした不足観点 G1–G14 を、担当エージェントが単独で着手できる実装タスクへ分解した実行バックログ。各タスクは着手時に個別 ExecPlan（`docs/exec-plans/active/EP-YYYYMMDD-<slug>.md`）へ転記して進める。

## 全タスク共通の作業規約（担当エージェントは必ず守ること）

1. 着手前にルート [AGENTS.md](../../AGENTS.md) → [docs/manual/CODING_AGENT.md](../manual/CODING_AGENT.md) → 該当サブディレクトリの `AGENTS.md` を読むこと
2. 着手時に `.agent/PLANS.md` のテンプレートから個別 ExecPlan を作成し、`.agent/PLANS.md` へ登録すること。本ファイルの該当タスクに EP へのリンクを追記すること
3. 新規挙動は test 先行で固定すること（TDD）。domain 純ロジックは `crates/domain` / `crates/app` の unit test、API 契約は `backend/tests/*_api.rs` の integration test で固定する
4. 新規 use case は rebuild target 構成（`crates/app` = use case、`crates/domain` = 純ロジック、`crates/contract` = DTO + round-trip test、`crates/infra-postgres` = SQL）に置くこと。現行 `backend/src/handlers/` は薄い配線に留める
5. SQLx migration は新規ファイル追加のみ（既存 migration の編集禁止）。次番号は `050` から
6. `/api/**` の契約を追加・変更したら、同じ commit で [docs/design-docs/backend-api-catalog.md](../design-docs/backend-api-catalog.md) と `backend/src/docs.rs`（OpenAPI）を更新すること
7. 検証は Validation Ladder の順（`fmt-check` → `backend-unit` → `backend-integration` → `clippy-backend` → `lint`）。docs 変更を含む場合は `docs-check` も実行すること
8. 法定値（法定労働時間、深夜帯、36協定上限、付与日数、保存年限など）はハードコードせず、設定マスタまたは config として外部化すること。本タスクリストの法令記述は設計の背景であり、実値は運用側が設定する前提で作ること
9. 完了条件は親リポジトリの Completion Rule（test green / harness stage green / catalog 同期 / commit 記録 / EP 実測更新）に従うこと

## 依存関係（着手順の目安）

```
Phase 0（設計・並列可）    : T-01, T-02, T-16, T-19
Phase 1（P1 実装）         : T-03(←T-01), T-04(←T-02), T-06(←T-01)
Phase 2（P1 完結 + 運用）  : T-05(←T-04), T-07(←T-03), T-08(←T-03), T-09(←T-01,T-06),
                             T-10(←T-01), T-12(設計独立/実装←T-03), T-17(←T-16)
Phase 3（拡張）            : T-11(←T-04,T-05), T-13(←T-03,T-04), T-14(←T-03,T-12),
                             T-15(←T-03,T-08), T-18(←T-09)
```

G15（打刻手段拡張）はタスク化しない。要件が発生した時点で親 EP へ再登録する。

---

## Phase 0: 設計

### T-01: 勤怠計算ポリシー design doc（G1 設計）

- **前提タスク:** なし。入力として [EP-20260703-work-schedule-phase3-settlement-balance](./active/EP-20260703-work-schedule-phase3-settlement-balance.md) の Design Decisions 1–3（実績 = effective timestamp 差分の整数分 / 実休憩打刻ベース / 第一増分は丸めなし）と [work-schedule-master.md](../design-docs/work-schedule-master.md) の Time Semantics を必ず読み、矛盾する決定をしないこと
- **目的:** 割増賃金計算・36協定監視・給与連携の全てが依存する「労働時間の法令区分」の定義を 1 箇所で確定し、後続実装タスクの source of truth を作る
- **指示:**
  1. `docs/design-docs/attendance-calculation-policy.md` を新規作成する
  2. 次を「決定」として記述する（選択肢の列挙で終わらせない）:
     - 日次区分の定義: 所定内 / 法定内残業 / 法定外残業 / 深夜（22:00–5:00 の暦時刻帯。`work_date` 帰属と両立する分割方法を明記）/ 法定休日労働（weekly holiday との対応付け方法を明記）
     - 週 40h 判定の週境界と、`work_date` 基準の月帰属との整合方法
     - 夜勤（`end_day_offset = 1`）・`workday_boundary` 前打刻の区分計算での扱い
     - flex スケジュール日の扱い（清算期間残高との役割分担。二重定義しない）
     - 承認済み休暇日・休日出勤・欠勤の区分上の扱い（T-06 / T-09 / T-13 への入力になる）
     - 丸め: 第一増分は丸めなし（生の分値）を既定とし、丸め導入時の互換方針を書く
     - パラメータ外部化: 法定労働時間・深夜帯・法定休日の指定は就業規則マスタ（新規テーブルまたは config）とする方針
  3. Follow-up Designs 1 の具体化であることを `work-schedule-master.md` に相互リンクで明記する
- **ゴール（Observable）:** design doc が存在し、上記論点すべてに決定が書かれている。`bash scripts/harness.sh docs-check` green。個別 EP 作成・commit 済み
- **次の関連タスク:** T-03（この doc を実装する）、T-06 / T-09 / T-10 / T-12（休暇日・判定猶予・警告扱いの決定を参照する）
- **Status:** 完了（2026-07-04、[EP-20260704-attendance-calculation-policy-design](./active/EP-20260704-attendance-calculation-policy-design.md) / commit 40e10b8。成果物: [attendance-calculation-policy.md](../design-docs/attendance-calculation-policy.md)）

### T-02: 有給休暇付与・残高台帳 design doc（G2 設計）

- **前提タスク:** なし（T-01 と並列可）
- **目的:** 現状「残高ゼロでも申請・承認できる」状態を解消するため、付与・消化・時効・取得義務を台帳として成立させる設計を確定する
- **指示:**
  1. `docs/design-docs/leave-entitlement.md` を新規作成する
  2. 次を決定として記述する:
     - 付与ルール: 入社 6 ヶ月 10 日 → 勤続年数テーブルによる付与。比例付与（週所定労働日数 4 日以下）は第一増分の対象外とするか含めるかを明示（推奨: 対象外とし拡張点だけ設計）
     - 台帳モデル: append-only の ledger イベント（`grant` / `consume` / `release`（取消・却下による引当解放）/ `expire` / `adjust`）。残高は導出値とし、残高スナップショットを正にしない
     - 消化順序: 時効の近い付与分から消化（FIFO）。時効は付与から 2 年
     - 年 5 日取得義務: 付与基準日から 1 年での取得日数判定の算出方法（アラートは T-08 / T-15 系の監視と同型の read API とする）
     - 既存データ移行: 稼働中テナントの現在残高を `adjust` イベントで初期投入する手順
     - 単位: 第一増分は日単位。半休・時間単位（T-11）へ拡張できるよう ledger の数量は「分」で持つことを推奨として検討・決定する
     - `LeaveType::Annual` 以外（sick / personal / other）は残高非連動として扱う（種別マスタ化は T-11）
- **ゴール:** design doc が存在し上記が決定済み。docs-check green。EP 作成・commit 済み
- **次の関連タスク:** T-04（実装）、T-11（単位拡張の前提）
- **Status:** 完了（2026-07-04、[EP-20260704-leave-entitlement-design](./active/EP-20260704-leave-entitlement-design.md) / commit 5ec584b。成果物: [leave-entitlement.md](../design-docs/leave-entitlement.md)）

### T-16: 汎用通知サービス基盤（G12 前半）

- **前提タスク:** なし。ただし [tech-debt-tracker.md](./tech-debt-tracker.md) #7（Queue / worker operational debt）と統合して 1 つの作業として扱うこと。rebuild target の `apps/worker` 設計と整合させる
- **目的:** lockout 専用になっている通知 queue / worker（`backend/src/services/lockout_notification_queue.rs` / `lockout_notification_worker.rs`）を汎用 notification 基盤へ一般化し、T-17 以降の全通知要件の土台を作る
- **指示:**
  1. queue のメッセージ型を event type 拡張可能な形（例: `notification_kind` + serialized payload）へ一般化する。既存 lockout 通知の挙動・テストは互換維持する
  2. worker の retry / DLQ / drain の運用境界を [docs/manual/RUNBOOK.md](../manual/RUNBOOK.md) に追記する（tech-debt #7 の Recommended Fix 1–2）
  3. `scripts/harness.sh` に `worker-once` smoke stage を追加し、`--list` / [docs/manual/HARNESS.md](../manual/HARNESS.md) / ルート `AGENTS.md` の Validation Ladder を同期する（tech-debt #7 Recommended Fix 3）
  4. 完了時に tech-debt-tracker #7 の Status を実測で更新する
- **ゴール:** 汎用化された queue で lockout 通知の既存 integration test が green。`worker-once` stage が green。RUNBOOK に worker 運用節が存在する
- **次の関連タスク:** T-17（申請・打刻イベントの配線）
- **Status:** 完了（2026-07-04、[EP-20260704-notification-service-generalization](./active/EP-20260704-notification-service-generalization.md) / commit 92ca21a。generic `NotificationJob` envelope + legacy fallback decode、`worker-once` stage 新設、tech-debt #7 返済。検証実測: unit 387 / lockout integration 10 / clippy 0 warnings / worker-once pass）

### T-19: 勤怠記録の保存期間 retention policy design doc（G14）

- **前提タスク:** なし
- **目的:** 出勤簿・賃金台帳相当データの法定保存（労基法 109 条: 5 年、当分の間 3 年）と、subject request（削除要求）承認の優先関係を先に文書で確定し、誤った物理削除の実装を防ぐ
- **指示:**
  1. `docs/design-docs/data-retention.md` を新規作成する
  2. 対象データ分類（attendance / break / correction / request / ledger / audit log / archived user PII）ごとに保存年限と起算点、期限後の扱い（アーカイブ / 匿名化 / 物理削除）を決定する
  3. subject request 承認処理における「法定保存義務が優先されるデータは削除せず匿名化・アクセス制限で応える」等の例外規定を決定し、既存の `subject_requests` 承認フロー（`handlers/subject_requests.rs`）への影響を洗い出す
  4. 実装（自動 purge バッチ等）は本タスクに含めない。方針確定後に別 EP として親 EP へ追記する
- **ゴール:** design doc が存在し、分類別の保存方針と subject request 例外規定が決定済み。docs-check green
- **次の関連タスク:** なし（実装 EP は方針確定後に起票）
- **Status:** 完了（2026-07-04、[EP-20260704-data-retention-policy](./active/EP-20260704-data-retention-policy.md) / commit 8d5101a。成果物: [data-retention.md](../design-docs/data-retention.md)。subject request approve が実データに触れない事実と crypto-shredding 不可の判断を記録済み）

---

## Phase 1: P1 実装

### T-03: 日次労働時間区分の read-model 実装（G1 実装）

- **前提タスク:** T-01（design doc の決定に従うこと。doc と実装が食い違う場合は doc を先に修正する）
- **個別 EP:** [EP-20260705-attendance-classification-read-model](./active/EP-20260705-attendance-classification-read-model.md)
- **Status:** 完了（EP: [EP-20260705-attendance-classification-read-model](./active/EP-20260705-attendance-classification-read-model.md)、commit: `feat(attendance): add daily classification read-model`）
- **目的:** 割増賃金計算の入力となる日次区分（所定内 / 法定内残業 / 法定外残業 / 深夜 / 法定休日）を導出値として提供する。T-07 / T-08 / T-13 / T-14 / T-15 がこの出力に依存する
- **指示:**
  1. `crates/domain` に区分計算の純ロジック（resolved workday snapshot + effective 打刻 → 日次区分分値）を実装し、夜勤・boundary 前打刻・休日・flex の各ケースを unit test で固定する
  2. `crates/app` に read use case（settlement balance と同型: DB 保存しない導出値、fail-closed）を実装する
  3. API を追加する: `GET /api/attendance/me/classification?year=&month=`（本人）、`GET /api/admin/users/{user_id}/classification?year=&month=`（Scoped Manager+。既存 resolved-workdays と同じ部署スコープ認可）。応答は日次内訳 + 月次合計の分値（整数分）
  4. 就業規則パラメータ（法定労働時間・深夜帯等）が必要な場合のみ新規 migration でマスタテーブルを追加する
  5. 既存 `AttendanceSummary` / CSV の互換は壊さない（置換ではなく追加）
- **ゴール:** 上記 API 2 本が実装され、contract round-trip テストと integration test（夜勤・休日出勤・flex 日を含む）が green。api-catalog / OpenAPI 同期済み。`bash scripts/harness.sh lint` green
- **次の関連タスク:** T-07 / T-08 / T-13 / T-14 / T-15

### T-04: 有給付与・残高台帳の実装（G2 実装 前半）

- **前提タスク:** T-02
- **目的:** 有給の付与と残高参照をシステム内で成立させる（消化引当は T-05）
- **指示:**
  1. 新規 migration: 付与ルールマスタ + `leave_ledger_entries`（append-only。`kind`, `amount_minutes`, `granted_at`, `expires_at`, 参照元 request id 等。T-02 の決定に従う）
  2. `crates/domain` に残高導出（FIFO 消化順・時効控除）の純ロジックを実装し、時効境界・複数付与の unit test で固定する
  3. 付与処理: 管理者起動の付与実行 API（例: `POST /api/admin/leave-grants/run`、System Admin、対象基準日指定、dry-run オプション付き）として実装する。cron 常駐は本タスクに含めない
  4. 残高参照 API: `GET /api/leave-balances/me`（本人）、`GET /api/admin/users/{user_id}/leave-balances`（Scoped Manager+）。残高・時効予定・年 5 日義務の消化状況を返す
  5. 初期残高投入用の `adjust` を管理 API または CLI として用意する
- **ゴール:** 付与実行 → 残高参照の一連が integration test で green。api-catalog / OpenAPI 同期済み。lint green
- **ExecPlan:** [EP-20260705-leave-entitlement-ledger](./active/EP-20260705-leave-entitlement-ledger.md)
- **Status:** 実装完了・検証済み（2026-07-08）。付与・残高 read・adjust・hire-date API を追加。消化引当は予定通り T-05
- **次の関連タスク:** T-05（消化引当）、T-11（単位拡張）、T-13（台帳の器を代休へ流用）

### T-06: 休暇承認→勤怠反映と打刻矛盾検知（G3 前半）

- **前提タスク:** T-01（休暇日の集計上の扱いの決定に従う）。T-02 / T-04 とは独立（残高と勤怠反映は別関心）
- **ExecPlan:** [EP-20260709-leave-attendance-integration](./active/EP-20260709-leave-attendance-integration.md)
- **Status:** 完了（2026-07-09、[EP-20260709-leave-attendance-integration](./active/EP-20260709-leave-attendance-integration.md) / commit: `feat(attendance): surface approved leave in read paths`）。承認済み休暇を read-time join で attendance 一覧・月次サマリ・本人/管理者 CSV・work schedule calendar に反映し、休暇日打刻は `leave_conflict` anomaly として検出する。resolved workday への休暇状態書き込みと打刻 reject はしない
- **目的:** 承認済み休暇が出勤簿に一切現れない分断を解消し、「休暇日」を欠測日と区別できるようにする
- **指示:**
  1. 承認済み leave request を日次ステータス（休暇区分付き）として読み取り経路へ反映する: `GET /api/attendance/me`、月次サマリ、本人/管理者 CSV export、`work-schedule-calendar`。反映は read 時 join（導出）を第一候補とし、resolved workday への書き込みは T-01 の決定がある場合のみ行う
  2. 休暇承認日への出勤打刻を anomaly（例: `leave_conflict`）として `GET /api/admin/work-schedule-anomalies` に追加する。打刻自体は拒否しない（実態優先。扱いは T-01 の決定に従う）
  3. 休暇申請の承認 / 取消と日次ステータスの整合（承認後キャンセル時の表示戻り）を integration test で固定する
- **ゴール:** 承認済み休暇がカレンダー・一覧・サマリ・CSV に休暇区分として現れ、休暇日への打刻が anomaly になる。既存レスポンス互換維持。lint green
- **次の関連タスク:** T-09（欠勤判定は本タスクの休暇反映が前提）、T-11（半休の表示拡張）

---

## Phase 2: P1 完結・運用品質

### T-05: 休暇申請の残高検証・消化引当（G2 実装 後半）

- **前提タスク:** T-04
- **ExecPlan:** [EP-20260708-leave-request-ledger-consumption](./active/EP-20260708-leave-request-ledger-consumption.md)
- **Status:** 完了（2026-07-08）。annual 申請の残高不足 reject、承認時 consume、承認済み annual 取消時 release を実装・検証済み。pending は引当せず承認時に同一 DB transaction で再検証する。
- **目的:** 残高不足の申請を入口で止め、承認・取消と台帳を同期させる
- **指示:**
  1. `POST /api/requests/leave`（`leave_type = annual`）で残高不足なら reject する（既存エラー envelope。エラー code を新設し contract に固定）
  2. 承認時に ledger へ `consume`、却下・取消時に `release` を記録する。承認と ledger 書き込みは同一トランザクションにする
  3. 申請中（pending）の引当をどう扱うか（申請時引当 or 承認時消化）は T-02 の決定に従う
  4. annual 以外の leave_type は残高非連動のまま挙動を変えない
- **ゴール:** 残高不足申請の reject、承認→残高減、取消→残高復帰が integration test で green。api-catalog のエラー欄更新済み
- **次の関連タスク:** T-11

### T-07: 残業申請と実績の突合（G3 後半）

- **前提タスク:** T-03（法定外実績の定義を使う）
- **目的:** 「申請なしの残業」「申請超過の残業」を検知し、申請ワークフローを形骸化させない
- **指示:**
  1. anomaly kind を追加する（例: `unapproved_overtime` / `overtime_exceeds_request`）。判定は日次: 承認済み overtime request の `planned_hours` と T-03 の法定外実績分を比較し、許容差（分、設定パラメータ）を超えたら検出
  2. `GET /api/admin/work-schedule-anomalies` と work-schedule-calendar へ露出する
  3. 検出のみとし、打刻や締めをブロックしない
- **ゴール:** 未申請残業・超過残業が anomaly として検出される integration test green。api-catalog 更新済み
- **次の関連タスク:** T-08

### T-08: 36協定上限の実績監視 API（G4）

- **前提タスク:** T-03（T-07 完了が望ましいが必須ではない）
- **目的:** 時間外労働の上限（月 45h / 年 360h、特別条項、単月 100h 未満・2〜6 ヶ月平均 80h）への接近・超過を発生前に可視化する
- **指示:**
  1. 閾値マスタ（36協定設定: 一般上限・特別条項上限・警告水準%）を新規 migration で追加する。System Admin が CRUD できる最小 API を付ける
  2. `GET /api/admin/overtime-monitor?year=&month=`（Scoped Manager+、部署スコープ）を追加し、ユーザーごとに: 当月法定外累計、年度累計、直近 2〜6 ヶ月平均、各上限に対する判定（`ok` / `warning` / `exceeded`）を返す
  3. 集計は T-03 の read-model を再利用する（独自の時間計算を実装しない）
  4. 年度境界（4 月起算か設定可か）は閾値マスタの属性として決定・文書化する
- **ゴール:** 監視 API が閾値超過・警告を正しく返す integration test green（境界値: ちょうど 45h、平均 80h 跨ぎ）。api-catalog 更新済み
- **次の関連タスク:** T-15（ダッシュボード表示に統合）

### T-09: 遅刻・早退・欠勤判定（G5）

- **前提タスク:** T-06（休暇日を欠勤誤検知から除外するため必須）、T-01（判定猶予・丸めの決定に従う）
- **目的:** 予定と実績の乖離（遅刻・早退・欠勤）を判定・集計可能にする
- **指示:**
  1. anomaly kind を追加する: `late` / `early_leave`（fixed schedule のみ。flex はコアタイム逸脱として別 kind か対象外かを T-01 の決定に従い明示）、`absent`（予定勤務日に打刻も承認済み休暇もない過去日）
  2. `GET /api/admin/work-schedule-anomalies` / work-schedule-calendar に露出し、月次サマリへ件数（遅刻回数・早退回数・欠勤日数）を追加する
  3. 判定猶予（grace 分）は設定パラメータとする
  4. 当日・未来日を `absent` にしない（過去日のみ）。打刻漏れ（既存 anomaly）との重複判定を整理する
- **ゴール:** 3 種の判定が unit + integration test で固定され、休暇承認日が `absent` にならないことをテストで保証。api-catalog 更新済み
- **次の関連タスク:** T-18（同じ判定基盤に相乗り）

### T-10: 休憩の法定下限チェック（G6）

- **前提タスク:** T-01（警告扱い・どの労働時間で判定するかの決定に従う）。T-03 と独立に実装可
- **目的:** 労働 6h 超で休憩 45 分未満 / 8h 超で 60 分未満の日を可視化する
- **指示:**
  1. anomaly kind `insufficient_break` を追加する。入力は effective 打刻（修正承認後の値）
  2. 検出のみとし、打刻・締めを hard reject しない
  3. 閾値（6h→45 分 / 8h→60 分）は法定値だが、設定マスタに既定値として持たせる（共通規約 8）
- **ゴール:** 境界値（労働ちょうど 6h / 8h、休憩ちょうど 45 / 60 分）を含む test green。api-catalog 更新済み
- **次の関連タスク:** T-14（給与側での扱いは T-01 の policy に従う）

### T-12: 月次締めの承認ワークフロー（G8）

- **前提タスク:** 設計は独立で開始可。実装は T-03 完了後を推奨（締め確定値の固定拡張を見込むため）
- **目的:** 現状の「system_admin による一方的な lock」を、本人確認 → 上長承認 → 締め確定 → 再締めの追跡可能なワークフローへ拡張する
- **指示:**
  1. まず `docs/design-docs/monthly-closing.md`（Follow-up Designs 2 の具体化）を作成し、状態遷移（`open` → `self_confirmed` → `approved` → `closed`、`closed` → 再締め `reopened` → 再 `closed`）、各遷移の権限（本人 / Scoped Manager / System Admin）、再締め時の監査証跡を決定する
  2. 既存 `POST /api/admin/work-schedule-closures/monthly`（lock）は `closed` 遷移の効果として統合し、既存 DB trigger による lock 挙動と互換にする
  3. 状態テーブルを新規 migration で追加し、遷移 API を実装する。承認は既存の部署スコープ認可（`check_approval_authorization` 相当）に合わせる
  4. `/admin` 画面への UI 配線は最小（状態表示 + 遷移ボタン）とし、リッチな一覧は T-15 に委ねる
- **ゴール:** design doc + 状態遷移 API が実装され、不正遷移（open から直接 closed 等、権限外遷移）が reject される integration test green。既存 closure API との互換テスト green
- **次の関連タスク:** T-14（closed 状態の確定値がエクスポートの前提）

### T-17: 申請・打刻イベントの通知配線（G12 後半）

- **前提タスク:** T-16
- **目的:** 申請の放置・打刻漏れの放置を通知で減らす
- **指示:**
  1. 通知イベントを配線する: 申請提出（承認可能な Scoped Manager へ）、承認・却下（申請者へ）、打刻漏れ（前営業日の `missing clock-out` 検知時に本人へ）
  2. 送信は T-16 の汎用 queue 経由とし、handler から直接 SMTP を呼ばない
  3. 通知失敗が申請処理本体を失敗させないこと（enqueue のみ同期、送信は worker）
  4. 打刻漏れリマインダーの起動方式（worker の定期スキャン等）は T-16 の worker 運用設計に従う
  5. 通知メールの文面は locale 対応（ja / en）とし、既存メールテンプレートの流儀に合わせる
- **ゴール:** 3 種の通知が enqueue される integration test green（SMTP は既存の skip / failure profile を利用）。RUNBOOK に通知種別一覧を追記
- **次の関連タスク:** なし

---

## Phase 3: 拡張

### T-11: 半休・時間単位休暇と休暇種別マスタ（G7）

- **前提タスク:** T-04, T-05（残高消化を分単位へ一般化する対象があるため）
- **目的:** 日単位固定の休暇申請を半休・時間単位へ拡張し、コード内 enum 固定の休暇種別を会社固有に定義可能にする
- **指示:**
  1. 休暇種別マスタを新規 migration で追加する（属性: 名称、有給/無給、残高連動有無、許可単位）。System Admin 向け CRUD API を付ける
  2. 既存 `LeaveType` 4 種（annual / sick / personal / other）は seed migration でマスタ行へ移行し、既存 API の文字列 `leave_type` は移行期間中互換維持する（contract test で固定）
  3. 申請 DTO に取得単位（`day` / `half_am` / `half_pm` / `hour`）と時間指定を追加し、ledger 消化を分単位で行う
  4. T-06 の勤怠反映を半休対応（半日勤務 + 半日休暇の同日共存）へ拡張する
- **ゴール:** 半休・時間単位の申請 → 承認 → 残高減 → 勤怠表示の一連が integration test green。既存日単位申請の互換テスト green
- **次の関連タスク:** なし

### T-13: 振替休日・代休の管理（G9）

- **前提タスク:** T-03（休日労働の区分判定）、T-04（付与台帳の器）
- **目的:** 休日出勤の対価（事前振替 = 振休、事後付与 = 代休）を付与・消化・期限まで追跡可能にする
- **指示:**
  1. 休日出勤申請（事前）のワークフローを追加し、承認時に振替先の休日指定（振休）または代休付与を選択できるようにする
  2. 代休は T-04 の ledger を種別拡張（`compensatory`）して残高・期限（設定可、例: 2 ヶ月）を管理する
  3. 振替は resolved workday の override（既存 `workday-overrides` API）と整合させる: 出勤日 ↔ 休日の入れ替えを 1 トランザクションで表現し、片側だけの状態を作らない
  4. T-01 の決定に従い、事前振替が成立した休日出勤は休日労働区分にしない
- **ゴール:** 振休（事前）・代休（事後）それぞれの申請 → 承認 → workday / 台帳反映が integration test green。期限切れ代休が残高から消えることをテストで固定
- **次の関連タスク:** T-14（休日労働区分の確定値に影響）

### T-14: 給与エクスポート契約（G10）

- **前提タスク:** T-03（区分集計）、T-12（closed 状態の確定）
- **目的:** 給与システムが必要とする「従業員 × 月 × 賃金項目」の確定値を、締め済み月についてのみ出力する
- **指示:**
  1. まず `docs/design-docs/payroll-export.md`（Follow-up Designs 4 の具体化）を作成し、出力項目（所定内 / 法定内残業 / 法定外残業 / 深夜 / 法定休日 / 欠勤控除日数 / 有給取得日数）、フォーマット（CSV 列順・エンコーディング）、締め時点固定の方式（closed 遷移時に集計値を snapshot 保存するか、closed 月は入力が不変なので都度導出で足りるか）を決定する
  2. `GET /api/admin/payroll-export?year=&month=`（System Admin）を実装する。`closed` でない月・対象ユーザーを含む場合は per-user の失敗として返し、全体は 200 とする（bulk assignment API の created/failed 形式を踏襲）
  3. 締め後に勤怠修正が承認された場合の扱い（再締め必須 → T-12 の `reopened` フローへ誘導）を design doc に明記し、エクスポート値がぶれないことをテストで固定する
- **ゴール:** design doc + API が実装され、締め済み月のみ出力・未締め月は per-user failure になる integration test green。api-catalog 更新済み
- **次の関連タスク:** なし（末端）

### T-15: 管理者向け月次レポート / 長時間労働ダッシュボード（G11）

- **前提タスク:** T-03（T-08 完了後の統合を推奨）
- **目的:** 管理者が raw CSV 以外で部署・全社の月次状況（労働時間、残業、anomaly、36協定判定）を一覧できるようにする
- **指示:**
  1. `GET /api/admin/attendance-report?year=&month=&department_id=`（Scoped Manager+、部署スコープ）を追加し、ユーザーごとの月次集計行（総労働・区分内訳・遅刻早退欠勤件数・anomaly 件数・36協定判定）を返す。ページング必須
  2. 集計は T-03 / T-08 / T-09 の read-model を合成する（新規の時間計算を実装しない）
  3. `/admin` 画面にレポートセクションを追加する。frontend は現行 `frontend/src/pages/admin/` の MVVM 構成に従い、`api/client.rs` の肥大化を避けて feature 単位のモジュールに置く（tech-debt #3 のガードに留意）
  4. 36協定 `warning` / `exceeded` のユーザーを上位に表示するソートを既定にする
- **ゴール:** レポート API + 画面が動作し、部署スコープ外が 403 になる integration test green。frontend host test green。lint green
- **次の関連タスク:** なし

### T-18: 勤務間インターバルチェック（G13）

- **前提タスク:** T-09（anomaly 判定基盤に相乗りする）
- **目的:** 前日の退勤から当日の出勤までの休息時間の不足（努力義務、目安 11h）を可視化する
- **指示:**
  1. anomaly kind `insufficient_rest` を追加する。判定: 直前の勤務終了（effective clock_out）から次の勤務開始（effective clock_in）までが閾値（既定 660 分、設定パラメータ）未満
  2. 夜勤（`end_day_offset = 1`）跨ぎで誤検知しないこと（暦日ではなく実時刻差で判定する）をテストで固定する
  3. 検出のみ。打刻はブロックしない
- **ゴール:** 通常勤務・夜勤・連続勤務のケースを含む test green。api-catalog 更新済み
- **次の関連タスク:** なし

---

## 完了の記録

- 各タスク完了時、本ファイルの該当タスクへ「**Status:** 完了（EP リンク / commit）」行を追記し、親 EP の Task Breakdown / Done Criteria のチェックボックスを更新すること
- タスクを分割・追加した場合は本ファイルと親 EP の両方を同じ commit で更新すること
