# 有給休暇付与・残高台帳設計

**Status:** Design only（実装未着手）。本 doc は [attendance-domain-gap-tasks.md](../exec-plans/attendance-domain-gap-tasks.md) の T-02（G2 設計）の成果物であり、[work-schedule-master.md](./work-schedule-master.md) の Follow-up Designs 3「有給休暇付与・残高台帳」の具体化である。実装は T-04（付与・残高参照）/ T-05（消化引当）/ T-11（半休・時間単位・種別マスタ）/ T-13（代休への流用）で行う。

**Updated:** 2026-07-04

**Scope:** 年次有給休暇（`LeaveType::Annual`）の付与、残高台帳、消化引当、時効、年5日取得義務の追跡を「台帳（ledger）」として成立させる設計を確定する。

## Decision Summary

現状は `LeaveType::Annual` の申請・承認はできるが、付与・残高・時効・取得義務の概念が一切なく、**残高ゼロでも申請・承認できる**（[EP-20260704-attendance-domain-gap-backlog](../exec-plans/active/EP-20260704-attendance-domain-gap-backlog.md) G2）。本設計はこれを次の 5 決定で解消する。

1. **残高は保存しない導出値**とし、正は **append-only の ledger イベント列**（`grant` / `consume` / `release` / `expire` / `adjust`）のみとする。残高スナップショットを主データにしない。
2. **付与は勤続年数テーブル**で行う（入社 6 ヶ月で 10 日 → 以降テーブル）。比例付与は第一増分の対象外とし、拡張点だけ設計する。
3. **消化は時効の近い付与ロットから（FIFO）**、**時効は付与日から 2 年**。
4. **ledger の数量は「分（integer minutes）」で持つ**。日単位付与も分へ換算して記録し、半休・時間単位（T-11）への拡張で単位系を変えない。
5. **`Annual` 以外（sick / personal / other）は残高非連動**とする。種別ごとの残高連動有無のマスタ化は T-11 が担う。
6. **消化対象日は resolved workday が稼働日と判定した日のみ**（[Consumption Target Days](#consumption-target-daysの消化対象日)）。所定休日・祝日は消化されず、稼働日 0 件の申請は拒否、勤務予定が解決できない日を含む場合も fail-closed で拒否する。read 表示（休暇の attendance surfacing）は暦日ベースのまま変えない。

これらは既存の勤怠読み取り経路（settlement balance の read-model パターン）と同型に、残高・時効予定・年5日消化状況を **read API** として導出する方針で一貫させる。

## Goals

- 付与・消化・時効・調整を追跡可能な台帳として成立させ、承認者が系外（紙・Excel）で残高確認する運用を解消する
- 消化順序と時効を決定的にし、同一入力から誰が計算しても同じ残高になる再現性を持たせる
- 年5日取得義務（労基法 39 条 7 項相当）の消化状況を監視 read API として提供する
- 半休・時間単位（T-11）・代休（T-13）へ台帳モデルを壊さず拡張できる余地を残す
- 稼働中テナントの既存残高を安全に初期投入できる移行手順を持つ

## Non-goals

- 割増賃金額・有給取得時の賃金額（平均賃金 / 標準報酬日額）の計算
- 比例付与（週所定労働日数 4 日以下）の日数計算の実装（拡張点の設計のみ）
- 半休・時間単位年休の申請 DTO・消化ロジックの実装（T-11）
- 代休（compensatory）の付与ワークフロー実装（T-13。台帳流用の余地のみ言及）
- 常駐 cron による自動付与（付与は管理者起動の実行 API とする。T-04）
- 出勤率 8 割要件の自動判定（付与可否の実データ判定。第一増分では付与実行時の対象者選定は運用入力とし、拡張点として記す）

## Terminology

| 用語 | 意味 |
| --- | --- |
| 付与ロット（grant lot） | 1 回の `grant` イベントで生まれた、付与日・時効日・数量を持つ有給の単位。消化・時効の対象単位 |
| 残高（balance） | ある時点で消化・時効に未使用の分数。ledger から導出する派生値 |
| 引当（allocation） | pending / approved な申請が特定ロットの数量を占有すること。`consume` で確定、`release` で解放 |
| 時効（expiry） | 付与日から 2 年経過で未消化分が失効すること。`expire` イベントで台帳へ反映 |
| 基準日（grant base date） | 付与の起算日。入社日 + 6 ヶ月が最初の基準日。以後 1 年ごと |
| 年5日取得義務 | 10 日以上付与された労働者に、付与基準日から 1 年以内に 5 日取得させる使用者の義務 |
| 分換算（day-to-minutes） | 「1 日 = N 分」の換算値。ロットごとに `day_equivalent_minutes` として固定する |

## Grant Rules（付与ルール）

### 勤続年数テーブル（第一増分の対象）

労基法 39 条の通常付与（週所定労働日数 5 日以上 / 週 30 時間以上）を既定とする。付与日数は**ハードコードせず付与ルールマスタ（config / テーブル）として外部化**する（共通作業規約 8）。既定シード値は次のとおり。

| 継続勤務年数（基準日） | 付与日数 |
| --- | --- |
| 0.5 年（入社 6 ヶ月） | 10 |
| 1.5 年 | 11 |
| 2.5 年 | 12 |
| 3.5 年 | 14 |
| 4.5 年 | 16 |
| 5.5 年 | 18 |
| 6.5 年以上 | 20 |

- **基準日（grant base date）**は入社日 + 6 ヶ月を最初の基準日とし、以後 1 年ごとに更新する（一律基準日方式＝全社同一日への統一は拡張点。第一増分は個人別入社日起算）。
- **入社日の source of truth は `users.hire_date`（T-04 で `DATE NULL` 列として追加）**とする。System Admin が `PUT /api/admin/users/{user_id}/hire-date` で投入し、`hire_date` 未設定のユーザーは付与実行で `hire_date_not_set` として skip する（対象者選定を運用入力とする本節の決定の具体化）。月末入社の基準日は暦月加算（月末クランプ。例: 8/31 入社 + 6 ヶ月 = 2/28）で決定的にする。
- 付与実行は**管理者起動の実行 API**（T-04 の `POST /api/admin/leave-grants/run`、基準日指定・dry-run 付き）とする。cron 常駐は含めない。
- **出勤率 8 割要件**の自動判定は第一増分では行わない。付与実行時に対象者を選定する入力（全員 / 明示リスト / 除外リスト）を受け、8 割未満での付与除外は運用判断とする。自動判定は勤怠実績（T-03 read-model）を入力とする拡張点として `Grant Rules Extension Points` に記す。

### 比例付与（第一増分の対象外・拡張点のみ設計）

週所定労働日数 4 日以下かつ週 30 時間未満の労働者への比例付与は、**第一増分の対象外**とする。理由は、週所定労働日数の source of truth（勤務体系マスタの曜日別 `day_kind` から導出するか、雇用契約属性として別途持つか）が未確定であり、労務判断を要するため。拡張余地は下記に閉じ込める。

### Grant Rules Extension Points

- 付与ルールマスタに `accrual_kind`（`standard` / `proportional`）列と、比例付与テーブル（週所定労働日数 × 継続年数 → 日数）を追加できる形にする。第一増分は `standard` 行のみをシードする。
- 出勤率 8 割判定は、付与実行 API に「実績参照モード」を追加し、T-03 の日次区分 read-model から対象期間の出勤率を導出する拡張点とする。
- 一律基準日（斉一的取扱い）への切り替えは、基準日算出関数を差し替える拡張点とする（ロット自体のモデルは変えない）。

## Ledger Model（台帳モデル）

### イベント種別

台帳は **append-only** とする。行の更新・削除をせず、常に追記でのみ状態を変える。イベント種別は 5 つ。

| kind | 意味 | amount_minutes の符号 | 参照 |
| --- | --- | --- | --- |
| `grant` | 付与。新しい付与ロットを生む | 正（+） | 付与ルール・基準日 |
| `consume` | 消化確定。承認された申請が特定ロットを減じる | 負（−） | `leave_request_id` + `lot_id` |
| `release` | 引当解放。却下・取消で確定前後の占有を戻す | 正（+） | `leave_request_id` + `lot_id` |
| `expire` | 時効失効。ロットの未消化分を失効させる | 負（−） | `lot_id` |
| `adjust` | 手動調整・初期移行。過不足を補正する | 正/負 | `reason` 必須 |

- **残高 = 対象ロット群の `grant` + `release` + `adjust`（正）− `consume` − `expire`（負）の総和**（分）。特定時点残高はイベントの `effective_at` / `work_date` 以前を集計して導出する。
- `grant` は必ず新しい `lot_id` を採番する。`consume` / `release` / `expire` は必ず既存 `lot_id` を参照する（どのロットから引いたかを FIFO で決定するため）。
- `adjust` はロットに紐付く場合（特定ロットの補正）と、初期移行のように新規ロットを生む場合（`lot_id` を新規採番、`expires_at` を指定）の両方を許す。移行用途は Migration 節で規定する。
- イベントは会計仕訳と同じく**逆仕訳（`release` は `consume` の、負の `adjust` は正の）で打ち消す**。既存イベントを書き換えない。

### 残高を導出値にする理由

- 締め・監査・時効・年5日義務のいずれも「ある基準日時点の残高」を必要とし、スナップショット 1 本では過去時点の再現ができない。
- settlement balance（[EP-20260703](../exec-plans/completed/EP-20260703-work-schedule-phase3-settlement-balance.md)）と同じく、**残高は保存せず read-model として都度計算**することで、後からの補正（`adjust`）・時効ルール変更を派生値の再計算だけで吸収できる。
- パフォーマンス上スナップショットが必要になった場合も、それは**キャッシュ**であって source of truth にしない（`grant base date` ごとの中間残高を materialized view 等で持つのは拡張点）。

## Consumption Order And Expiry（消化順序と時効）

### FIFO（時効の近いロットから消化）

- 消化は**時効日（`expires_at`）が近いロットから**引く（FIFO）。同一時効日のロットが複数ある場合は付与日（`granted_at`）が古い順、それも同じなら `lot_id` 昇順で決定的にする。
- 引当・消化のロット選択ロジックは `crates/domain` の純ロジックとして実装し、複数ロット跨ぎ・端数（分）消化を unit test で固定する（T-04）。
- 消化時点で残高不足なら、その申請は成立させない（残高検証は T-05）。

### 時効（付与日から 2 年）

- 時効は**付与日（`granted_at`）から 2 年**（`expires_at = granted_at + 2 years`）。時効日は**ハードコードせず付与ルールマスタの属性**として持つ（`expiry_months` 既定 24）。
- 失効は `expire` イベントで台帳へ反映する。失効の反映方式は**遅延評価を第一候補**とする: 残高導出時に「`expires_at <= 基準日` のロットの未消化分」を控除して残高を出し、`expire` イベントは付与実行 API・締め・夜間バッチのいずれかが**冪等に補記**する（同一ロットへ二重の `expire` を書かない一意制約で保証）。これにより「`expire` イベントの書き込み遅延で残高が過大に見える」ことを防ぐ。
- 当年付与分と前年繰越分が併存する場合、上記 FIFO により**繰越（時効が近い）分から先に消化**される。

## Consumption Target Days（消化対象日）

**Updated:** 2026-07-09（コードレビュー指摘 H-1 対応）。

初期実装は申請期間の**暦日数**（`(end_date - start_date).num_days() + 1`）をそのまま消化日数として扱っており、土日・所定休日を含む期間でも暦日分の残高を消費していた（例: 金〜月の 4 暦日申請で実労働日 2 日しかなくても 4 日分を消費）。本節でこれを是正する決定を記す。

1. **消化対象日は「resolved workday の day_kind が稼働日（`ScheduledWorkday`）である日」のみ**とする。所定休日（`ScheduledNonWorkingDay`）・法定休日/祝日（`PublicHoliday`）は残高を消費しない。稼働日判定は `work-schedule-master.md` / `EP-20260621-resolve-workday.md` の resolved workday の仕組み（`crates/app::work_schedules::ResolveWorkday` と `ResolvedDayKind`）をそのまま再利用し、消化計算専用の曜日判定を新設しない。
2. **申請期間内の稼働日数が 0 の場合、その申請・承認はエラーとして拒否する**。消化ゼロの annual 申請（全日が所定休日・祝日）は成立させない。
3. **申請期間中に勤務予定を解決できない日（work schedule 未割当・データ不整合等）が 1 日でも含まれる場合は fail-closed とする**。エラーで申請・承認を拒否し、解決できない日を暦日として消化対象に含める・除外するといった暗黙のフォールバックは行わない。
4. **承認済み休暇の read 表示（T-06 の attendance 日次区分への leave surfacing）は従来どおり申請期間の全日（暦日）を対象**とする。稼働日ベースに変えるのは**消化（残高計算）のみ**であり、表示・勤怠区分への反映は申請期間そのものを正とする（土日を挟む申請でも、表示上は申請期間全体が「承認済み休暇」として扱われる）。
5. 稼働日数から分への換算は、消化時と同じ FIFO 対象ロットの `day_equivalent_minutes` を用いる。**アクティブなロットが存在しない場合は換算基準がないため明示的なエラーとする**（固定値へのフォールバックはしない）。

この決定は T-04（`crates/app::leave_ledger`）の消化計算関数と、それを呼び出す申請作成時の残高検証・承認時の消化確定の両方に適用し、両者で同一の稼働日数計算を用いて乖離させない。

## Annual 5-Day Obligation（年5日取得義務）

- 対象は**10 日以上付与された労働者**。付与基準日（`grant base date`）から **1 年以内**の取得日数が 5 日に達しているかを判定する。
- 判定は「基準日 D から D + 1 年」の window に属する `consume`（`Annual`・**日および半休換算の日数**）を合算し、5 日（= 5 × `day_equivalent_minutes`）と比較する。
- **時間単位年休（T-11 の hour 単位）は年5日義務の取得日数に算入しない**（労基法の取扱いに合わせる）。半休は 0.5 日として算入する。この算入可否は種別・単位マスタ（T-11）の属性で切り替えられるようにする。
- 判定は**保存せず read API として導出**する。アラートは T-08 / T-15 系の監視 read API と同型とし、`ok` / `warning`（期限接近・未達）/ `at_risk`（残り期間で達成困難）/ `fulfilled` を返す設計とする（閾値・warning 水準は設定パラメータ）。
- 出力単位: ユーザー × 付与基準日サイクルごとに「対象義務日数（5）・当該 window の取得日数・残必要日数・window 終了日」。

## Quantity Unit（数量の単位）

- **ledger の数量は分（`amount_minutes: integer`）で持つ**。日単位付与も分へ換算して記録する。
- **1 日 = N 分の換算はロットごとに `day_equivalent_minutes` として固定**する（`grant` 時に採番。既定は就業規則の 1 日所定労働時間分、例: 8h = 480 分）。理由:
  - 半休（0.5 日）・時間単位（T-11）を同一単位系で表現でき、`consume` を分で行える。
  - 付与後に就業規則の所定労働時間が変わっても、既存ロットの日数換算が**付与時点の値で固定**され、過去残高の再現性が保たれる。
- 日数表示への逆換算（残高 = X 日 Y 時間）は read-model の表示層で `day_equivalent_minutes` を使って行う。**分が正、日は表示の派生**とする。
- 年5日義務・付与日数のような「日」で定義される法令値は、`day_equivalent_minutes` を介して分と相互変換する。**丸め**（例: 1 日未満の端数消化を許すか）の規則は T-11 で単位を拡張する際に確定し、第一増分は日単位消化のみ（半休・時間は Out）とする。

## Type Boundary（種別の境界）

- **`LeaveType::Annual` のみ残高連動**とする。`sick` / `personal` / `other` は**残高非連動**で現行挙動（残高検証なし）を変えない。
- 種別ごとの「残高連動有無・有給/無給・許可単位」の**マスタ化は T-11**。第一増分の台帳・残高検証は `Annual` 固定でよい。
- T-05 の残高検証・消化引当も `leave_type = annual` のみを対象とし、それ以外は素通しする。

## Table Sketch（DDL スケッチ）

> 実際の migration は T-04 で新規追加する（既存 migration 編集禁止、次番号は 050 以降。**本 doc では migration を作らない**）。以下は設計スケッチであり、型・制約は実装時に PostgreSQL 方言・既存 `types` に合わせて確定する。

### 付与ルールマスタ

```sql
-- 付与日数・時効・付与種別を外部化する（法定値をハードコードしない）
CREATE TABLE leave_grant_rules (
    id                   UUID PRIMARY KEY,
    accrual_kind         TEXT NOT NULL,          -- 'standard' | 'proportional'（第一増分は standard のみ）
    tenure_months        INTEGER NOT NULL,       -- 基準日の継続勤務月数（6, 18, 30, ...）
    weekly_working_days  INTEGER,                -- proportional 用。standard は NULL
    granted_days         INTEGER NOT NULL,       -- 付与日数（10, 11, 12, ...）
    expiry_months        INTEGER NOT NULL DEFAULT 24,
    effective_from       DATE NOT NULL,          -- ルール自体の適用開始（就業規則改定に対応）
    created_at           TIMESTAMPTZ NOT NULL,
    UNIQUE (accrual_kind, tenure_months, weekly_working_days, effective_from)
);
```

### 台帳イベント（append-only）

```sql
CREATE TABLE leave_ledger_entries (
    id                     UUID PRIMARY KEY,
    user_id                UUID NOT NULL REFERENCES users(id),
    leave_type             TEXT NOT NULL,        -- 第一増分は 'annual' のみ。将来 'compensatory' 等
    kind                   TEXT NOT NULL,        -- 'grant' | 'consume' | 'release' | 'expire' | 'adjust'
    lot_id                 UUID NOT NULL,        -- 付与ロット識別子。grant で採番、他 kind は既存を参照
    amount_minutes         INTEGER NOT NULL,     -- grant/release/adjust(+) は正、consume/expire は負
    day_equivalent_minutes INTEGER NOT NULL,     -- このロットの 1 日 = N 分（grant 時固定、日⇔分換算）
    granted_at             DATE,                 -- grant/adjust(新規ロット) で必須。ロットの付与日
    expires_at             DATE,                 -- grant/adjust(新規ロット) で必須。granted_at + expiry
    grant_base_date        DATE,                 -- grant で必須。年5日義務 window の起算日
    leave_request_id       UUID REFERENCES leave_requests(id),  -- consume/release で必須
    reason                 TEXT,                 -- adjust で必須。監査用
    created_by             UUID REFERENCES users(id),
    effective_at           TIMESTAMPTZ NOT NULL, -- イベント発生時刻（残高の時点集計キー）
    created_at             TIMESTAMPTZ NOT NULL
);

-- 消化・時効は既存ロット参照、grant は新規ロット、を保証する補助
CREATE INDEX idx_leave_ledger_user_type      ON leave_ledger_entries (user_id, leave_type);
CREATE INDEX idx_leave_ledger_lot            ON leave_ledger_entries (lot_id);
-- 同一ロットへの二重 expire を防ぐ（時効の冪等補記）
CREATE UNIQUE INDEX uq_leave_ledger_expire   ON leave_ledger_entries (lot_id) WHERE kind = 'expire';
```

主要制約・不変条件（実装時に application + DB で二重化）:

- `grant` は新規 `lot_id`、`consume` / `release` / `expire` は既存 `lot_id` を参照する。
- `consume` / `release` は同一 `leave_request_id` の承認・取消と**同一トランザクション**で書く（T-05）。
- 残高導出クエリは `(user_id, leave_type)` でロット別に集計し、負残高になる `consume` は application 層で拒否する（残高検証は T-05）。
- ロットは `granted_at` / `expires_at` / `day_equivalent_minutes` を**最初の `grant`（または移行 `adjust`）で固定**し、以後のイベントでは変更しない（append-only により物理的にも変更されない）。

## Migration Of Existing Data（既存データの初期移行）

稼働中テナントは既に「系外で管理された現在残高」を持つため、台帳を後から真実にするには初期投入が要る。

1. 各ユーザーの**現在残高（系外の日数）と、その時効内訳**（いつ付与された何日が残っているか）を運用側が用意する。
2. 内訳の各行を**`adjust` イベントで 1 ロットずつ投入**する（`kind='adjust'`・新規 `lot_id`・`granted_at` / `expires_at` / `grant_base_date` / `day_equivalent_minutes` を実際の付与実績に合わせて指定・`reason='initial migration'`）。時効内訳が不明で単一残高しか無い場合は、**保守的に直近付与相当の `expires_at` を設定**するか、運用合意した時効日を用いる（過大な繰越を作らない）。
3. 投入後の導出残高が系外残高と一致することを**dry-run 照合**してから確定する（付与実行 API と同様に dry-run を持つ。T-04）。
4. 移行完了後は、以降の付与を勤続年数テーブルの `grant` 実行 API に切り替える。移行 `adjust` と通常 `grant` はどちらも同じロット構造なので、消化・時効・年5日義務の導出は移行分にもそのまま効く。

移行は `adjust` のみで表現し、`grant` の付与ルール判定を過去に遡って動かさない（過去の付与実績を推測して再現しない）。

## Read-Model And APIs（導出値と API）

残高・時効予定・年5日義務はいずれも**保存せず read API として導出**する（settlement balance と同型）。実装は T-04 / T-05。

| Method | Path | Auth | Semantics |
| --- | --- | --- | --- |
| `GET` | `/api/leave-balances/me` | user | 本人の残高・時効予定・年5日義務の消化状況 |
| `GET` | `/api/admin/users/{user_id}/leave-balances` | scoped manager+ | 配下ユーザーの同上（既存 resolved-workdays と同じ部署スコープ認可） |
| `POST` | `/api/admin/leave-grants/run` | system admin | 基準日指定の付与実行（dry-run オプション）。T-04 |
| `POST` | `/api/admin/leave-ledger/adjust` | system admin | 初期移行・手動調整の `adjust` 投入（dry-run 照合付き）。T-04 |
| `PUT` | `/api/admin/users/{user_id}/hire-date` | system admin | 付与基準日の起点となる入社日（`users.hire_date`）の投入。T-04 |

- 残高 read の応答は分（integer）を正とし、日数は `day_equivalent_minutes` から導出した表示値を併記する。
- 計算不可・境界（例: 付与ルール未設定、種別が非連動）は settlement balance と同様に **tagged union / 明示ステータス**で表現し、リクエスト不正（400）と区別する（実装時に contract round-trip / OpenAPI へ固定）。
- API 契約（route / method / request / response / error / 認可）を追加する T-04 / T-05 は、同一変更で [backend-api-catalog.md](./backend-api-catalog.md) と OpenAPI（`backend/src/docs.rs`）を更新する（共通作業規約 6）。

## Architecture Placement

[work-schedule-master.md](./work-schedule-master.md) と同じ rebuild target 配置に従う。

| Boundary | Responsibility |
| --- | --- |
| `crates/domain` | ロット・FIFO 消化順・時効控除・残高導出・年5日義務判定の純ロジック（時効境界・複数ロット・端数分を unit test で固定） |
| `crates/app` | 付与実行・残高参照・消化引当（consume / release）の use case。read use case は `ListUserWorkdays` / `CalculateSettlementBalance` と同型 |
| `crates/contract` | 残高・年5日義務・付与実行の request/response DTO、エラー code、round-trip テスト |
| `crates/infra-postgres` | `leave_ledger_entries` / `leave_grant_rules` repository、時効の冪等補記、consume/release の同一トランザクション |
| `backend/src/handlers/` | 薄い配線（認可・DTO 変換・HTTP error mapping）。台帳ロジックを handler に溜めない |

## References For Downstream Tasks（後続タスクの参照項目）

### T-04（付与・残高台帳の実装）が参照する項目

- 勤続年数テーブル（Grant Rules）と `leave_grant_rules` マスタ（法定値を外部化）
- `leave_ledger_entries` の DDL スケッチと append-only 不変条件
- 残高導出式（`grant + release + adjust(+) − consume − expire`）と FIFO 消化順（`crates/domain` 純ロジック）
- 時効の遅延評価 + 冪等な `expire` 補記
- `day_equivalent_minutes` によるロット単位の日⇔分換算
- 付与実行 API（`POST /api/admin/leave-grants/run`、dry-run）・残高 read API 2 本
- 初期移行の `adjust` 投入手順と dry-run 照合

### T-05（消化引当）が参照する項目

- `consume`（承認時）/ `release`（却下・取消時）を承認と**同一トランザクション**で書く
- 残高不足の申請を入口で reject（新設エラー code を contract に固定）
- pending の引当方式（**承認時消化**を第一候補とし、申請時引当が要る場合の拡張点）: 第一増分は「pending は引当せず承認時に FIFO で `consume`、承認時に残高再検証」を推奨とする
- `annual` 以外の leave_type は残高非連動のまま素通し

### T-11（半休・時間単位・種別マスタ）が参照する項目

- 数量が分（`amount_minutes`）で統一されていること（半休 = 0.5 日 × `day_equivalent_minutes`、時間単位 = 実時間分）
- 種別マスタ（有給/無給・残高連動有無・許可単位）で `Annual` 固定を一般化する境界
- 年5日義務の算入可否（半休 = 算入、時間単位 = 非算入）を種別・単位マスタ属性で切り替える設計
- 端数消化・丸め規則を単位拡張時に確定すること（第一増分は日単位のみ）

## Extension For T-13 Compensatory Leave（代休への台帳流用）

T-13（振替休日・代休）は、休日出勤の事後付与（代休）を**本設計の台帳へ `leave_type = 'compensatory'` として流用**できる。

- `leave_ledger_entries` の `leave_type` を `annual` 以外にも開き、代休付与を `grant`（`granted_at` = 休日労働日、`expires_at` = 設定可の期限、例: 2 ヶ月）として記録する。
- FIFO 消化・時効（`expire`）・残高導出・引当（consume / release）の**純ロジックはそのまま再利用**でき、代休固有なのは「付与の起点が休日労働の承認である」「時効月数が短い」点のみ。
- 代休は年5日義務・比例付与の対象外なので、義務判定と付与ルールマスタは `leave_type = annual` に限定し、代休は付与ルールマスタを経由せず休日出勤ワークフローから直接 `grant` する。
- 振替休日（事前振替）は残高ではなく resolved workday の override で表現する（[work-schedule-master.md](./work-schedule-master.md) の `WorkdayOverride`）ため台帳の対象外。台帳へ載るのは**事後付与の代休のみ**とする、という境界を T-13 で確定する。
- `holiday_work_request_id` を grant source として一意化し、承認再実行でも二重付与しない。期限は休日労働日以前で最新の `compensatory_leave_settings.effective_from` から決定する。
- `compensatory` は通常の勤続年数付与batchと年5日義務の対象外だが、残高照会、FIFO consume、取消release、遅延expireは種別コードを引数に取る共通台帳処理を利用する。

## Acceptance Criteria（設計 doc として）

- 付与ルール（勤続年数テーブル / 比例付与の Out 明記 / 拡張点）が決定として書かれている
- ledger 5 イベント（grant / consume / release / expire / adjust）と「残高＝導出値」が定義されている
- FIFO 消化順と時効 2 年（遅延評価 + 冪等 expire）が決定されている
- 年5日義務の判定方法（window・単位・時間単位年休の非算入）が決定されている
- 数量単位（分・`day_equivalent_minutes`）と `Annual` 限定の境界が決定されている
- 既存データの `adjust` 初期移行手順が書かれている
- テーブル案（DDL スケッチ）と T-04 / T-05 / T-11 の参照項目が明示されている
- T-13 の代休流用の拡張余地が 1 節で言及されている
- `bash scripts/harness.sh docs-check` green

## Follow-up

- 実装は T-04 → T-05 → T-11 の順で増分化する（[attendance-domain-gap-tasks.md](../exec-plans/attendance-domain-gap-tasks.md)）。
- 出勤率 8 割判定・比例付与・一律基準日は本 doc の Extension Points を起点に、必要が生じた時点で別 EP を親 EP へ追記する。
- 締め時点の残高固定は給与エクスポート契約（Follow-up Designs 4 / T-14）の設計時に扱う。残高は導出値であり、固定が要るのはエクスポート境界のみ。
