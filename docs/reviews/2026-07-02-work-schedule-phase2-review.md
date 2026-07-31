# Review: 勤務体系マスタ Phase2 運用管理機能

**Date:** 2026-07-02
**Reviewer:** Claude Code（rust-reviewer / database-reviewer / security-reviewer 並列レビューを統合）
**対象コミット:** `b54c6cd` (`feat: add work schedule phase2 controls`)
**対象ExecPlan:** [`docs/exec-plans/completed/EP-20260702-work-schedule-phase2-operational-controls.md`](../exec-plans/completed/EP-20260702-work-schedule-phase2-operational-controls.md)

## 総合判定: **Block**（Critical 1件の解消が必須）

Critical 1件（manager権限でのanomaly全社閲覧）は認可設計の穴であり、マージ前に必ず修正が必要です。
High 4件はPhase2がbackend/API MVP（frontend・cron daemonはOut of scope）である点を踏まえても、本番運用前に対応すべき整合性・可用性リスクです。

対象範囲:
- `backend/migrations/047_create_work_schedule_monthly_closures.sql`
- `backend/src/repositories/work_schedule/operations.rs`
- `backend/src/handlers/admin/work_schedules.rs`
- `backend/src/main.rs`
- `crates/contract/src/work_schedules.rs`
- `backend/src/docs.rs`
- `backend/tests/work_schedule_phase2_api.rs`

Done Criteriaのうち機能面（projection生成／カレンダー取得／anomaly検出／bulk assignment／月次締めlock／API catalog同期／coverage 80%超）はテストで満たされていることを確認しましたが、認可・整合性・性能の観点で見落としがあります。

---

## Critical

### C1. manager が `user_id` を省略すると全社員のanomalyを閲覧できる（認可バイパス）

- **File:** `backend/src/handlers/admin/work_schedules.rs:469-492`（`list_work_schedule_anomalies`）+ `backend/src/repositories/work_schedule/operations.rs:60-73`（`list_anomalies`）
- **事実確認:** `list_work_schedule_anomalies` は `require_manager(&user)?` のみでroleチェックを行い、`query.user_id` が指定された場合にだけ `authorize_scope`（部署スコープ認可）を呼びます。`query.user_id` が `None` のときは `authorize_scope` を一切通らず `work_schedule::list_anomalies(pool, None, ...)` が呼ばれ、repository側は `user_ids: None` を「全ユーザー対象」として `SELECT id FROM users ORDER BY id` で全社員をロードします（`operations.rs:66-73`）。
- **なぜ問題か:** ExecPlanのConstraints「manager readは既存部署スコープ認可を再利用する」に反し、部署スコープを持たないmanagerロールのユーザーが `user_id` クエリパラメータを省略するだけで、自部署外を含む全社員のanomaly（未設定日・予定外勤務・打刻漏れ）を取得できてしまいます。カレンダーAPI（`get_work_schedule_calendar`）はPathパラメータで`user_id`必須のため常に`authorize_scope`が通りますが、anomaly一覧だけがこの穴を持っています。
- **どう直すべきか:** `user_id`未指定時は「system adminなら全社」「managerなら配下ユーザーIDの一覧に限定」で分岐する。具体的には、managerの場合は事前に配下ユーザーID集合を取得し `list_anomalies(pool, Some(subordinate_ids), ...)` を渡すよう変更する（`can_manager_approve`が使っている部署再帰ロジックを流用できるはず）。system_admin以外が無制限の`None`を渡せないようにする。

---

## High

### H1. `generate_work_schedule_projections` が `user_ids × 日数` の直列N+1ループで、件数上限がない

- **File:** `backend/src/handlers/admin/work_schedules.rs:331-393`
- **事実確認:** `dates_inclusive(from, to)`（最大366日）と `payload.user_ids` の二重ループで `ResolveWorkday::execute()` を1件ずつ `await` しています。`ResolveWorkday::execute`（`crates/app/src/work_schedules.rs:201-292`）は1呼び出しあたり約10クエリ（override/assignment/department_lineage再帰CTE/published_version/day_rule/holiday判定/upsert/再ロード）を発行します。`user_ids` に件数上限バリデーションがないため、数百ユーザー×366日で数十万クエリが単一HTTPリクエスト内に直列発行され、コネクションプールを長時間占有します。
- **どう直すべきか:** 最低限 `user_ids.len()` に上限（例: 500件）を設けて400を返す。中期的には日付×ユーザーをセットベースSQL（`UNNEST` + `INSERT ... ON CONFLICT`）にリファクタする。`bulk_create_work_schedule_assignments` の `targets` も同様に上限がないため合わせて対応する。

### H2. `close_month` がトランザクション化されておらず、UPDATE成功後にINSERT失敗すると監査ログが欠落する

- **File:** `backend/src/repositories/work_schedule/operations.rs:168-220`
- **事実確認:** `UPDATE resolved_workdays SET locked_at = NOW() ...`（180-200行目）と `INSERT INTO work_schedule_monthly_closures ...`（202-217行目）が別々の `pool` 呼び出しで実行されており、`pool.begin()` によるトランザクションでまとめられていません。同じ `operations.rs` と隣接する `versions.rs`（`create_version`/`replace_version`）は複数DMLを`transaction.commit()`でまとめる既存パターンを持っており、このパターンから逸脱しています。
- **なぜ問題か:** UPDATE成功後、INSERT（`user_ids`カラムへのバインドや制約違反、DB切断など）が失敗すると、`resolved_workdays`はlock済みなのに`work_schedule_monthly_closures`に監査レコードが存在しない不整合状態になります。月次締めは監査要件の強いオペレーションのため、この不整合は運用上致命的になり得ます。
- **どう直すべきか:** `pool.begin()` で1トランザクションにまとめ、両方成功した場合のみ`commit()`する。

### H3. bulk assignment / projection生成のエラー経路でDB内部エラー文字列（SQLエラー含む）がAPIレスポンスに漏洩する

- **File:** `backend/src/handlers/admin/work_schedules.rs:718-751`（`repository_error_code_message`）、同ファイル 266-273行目（projection生成のエラー処理）
- **事実確認:** `repository_error_code_message` は `WorkScheduleRepositoryError::Sqlx(error)` を `error.to_string()` のまま `BulkWorkScheduleAssignmentFailure.message` に格納し、200 OKのJSON本文としてクライアントへ返します。同様に `generate_work_schedule_projections` も `ResolveWorkdayError::Repository(String)` を `error.to_string()` のままレスポンスに含めます。他の全経路（`map_repository_error`）は`Sqlx`エラーを`AppError::InternalServerError`として汎用メッセージに丸めているのに、この2箇所だけ例外的にDB内部情報を露出する非対称な実装になっています。
- **なぜ問題か:** テーブル名・制約名などの内部スキーマ情報の偵察に使われる可能性があります（`common/security.md`「エラーメッセージは内部情報を漏らさない」に抵触）。
- **どう直すべきか:** `Sqlx`ケースも固定の汎用メッセージ（例: `"internal error while processing this target"`）へマップし、詳細は`tracing::error!`でサーバ側ログにのみ残す。

### H4. 月次締めが対象0件でも監査レコードを作成し、再実行のたびに重複が蓄積する

- **File:** `backend/src/repositories/work_schedule/operations.rs:168-220`、`backend/migrations/047_create_work_schedule_monthly_closures.sql`
- **事実確認:** `close_month` は `rows_affected == 0`（既に全部lock済み、または対象日にresolved_workdaysが存在しない）でも常に`work_schedule_monthly_closures`へINSERTします。同一月を複数回締めると`locked_count=0`の重複closure履歴が積み上がります。`047`migrationには`(year, month)`や`(year, month, user_idsキー)`に対するUNIQUE制約もありません。
- **どう直すべきか:** `rows_affected == 0` の場合はclosure行を作らず既存closureを返す設計にする、または`(year, month, user_ids正規化キー)`にUNIQUE制約を張り冪等性をDBレベルで保証する。

---

## Medium

### M1. `close_month` に存在しない `user_id` を渡してもエラーにならず静かに0件ロックされる

- **File:** `backend/src/repositories/work_schedule/operations.rs:189-200`、`backend/src/handlers/admin/work_schedules.rs:554-560`
- handler側は`UserId::from_str`による形式チェックのみで、DB上の実在性は確認していません。誤入力に気づけないため、レスポンスに`unknown_user_ids`のような警告フィールドを追加することを検討してください。

### M2. `list_anomalies` の全ユーザーフォールバックがページネーションなしでスケールしない

- **File:** `backend/src/repositories/work_schedule/operations.rs:66-73`
- `user_ids=None`のとき全ユーザーを`resolved`/`attendance`とともにin-memoryで`users × dates`の二重ループ突合しており、大規模組織では線形以上にメモリ・レイテンシが増加します（C1の認可修正後も、system_admin向けの全社閲覧としては引き続き課題です）。ページネーションもしくは日数上限のさらなる引き下げを検討してください。

### M3. `backend/src/handlers/admin/work_schedules.rs` が890行に達し、AGENTS.mdの「800 max」を超過

- Phase1+Phase2のhandlerが同一ファイルに混在しているため（Progress Notesにも既知の課題として記載あり）、Phase2関数群を`backend/src/handlers/admin/work_schedule_operations.rs`等へ分離することを推奨します。既存の`repositories/work_schedule/{master,versions,assignments,operations}.rs`分割パターンと対称にできます。

### M4. `close_work_schedule_month` の `year` に明示的な範囲検証がない

- **File:** `backend/src/handlers/admin/work_schedules.rs`（`close_work_schedule_month`）
- `month`は`1..=12`検証があるが`year`はhandlerで未検証。範囲外の`year`はDBのCHECK制約(`1900..=9999`)違反として扱われ400ではなく500になり、監視上のノイズになります。handler側で明示的に検証し400を返すよう統一してください。

---

## Low

- **`i64::try_from(rows_affected).unwrap_or(i64::MAX)`**（`operations.rs:213, 219`）— 実務上オーバーフローは考えにくいが、失敗時に`i64::MAX`へサイレントフォールバックするのは「決定的」を重視するPhase2の設計方針と整合しません。エラーを返す設計にすべきです。
- **`dates_inclusive`がhandler側とrepository側に重複定義**（`work_schedules.rs:770-782` / `operations.rs:222-231`）— 共有ユーティリティへ統合してDRY違反を解消してください。
- **`list_anomalies`が`Option<Vec<String>>`を値で受け取る一方、`close_month`は`&[String]`を受け取る**など、Phase2内でのAPIシグネチャの一貫性が揺れています。スライス受け取りに揃えると呼び出し側のclone/vec生成を削減できます。

---

## 良い点

- **047 migrationは既存トリガーと正しく整合**: `resolved_workdays.locked_at`を立てるだけで、既存の`044_create_resolved_workdays.sql`の`resolved_workdays_immutable_when_locked`トリガーと`046_lock_workday_override_mutations.sql`の`workday_overrides_immutable_when_locked`トリガーが「未lock→lock」の遷移自体は許可しつつ、ロック後のoverride upsert/deleteをERRCODE `23514`で拒否する設計と整合しており、handler側で409へ正しくマッピングされています（`backend/tests/work_schedule_phase2_api.rs:162-173`で確認済み）。lockのsource of truthをDBに一元化するというExecPlanのConstraintsを守れています。
- **`close_month`のUPDATE自体はセットベース**: 行ごとループではなく`UPDATE ... WHERE work_date BETWEEN $1 AND $2 AND locked_at IS NULL`の一括更新で、月次締め処理そのものはスケールする設計です。
- **全SQLがbind parameter経由**: `operations.rs`を含め文字列結合によるSQL構築はなく、`ANY($1)`を使った配列バインドも含めSQLインジェクションのリスクは見当たりません。
- **system admin mutation系のroute配線は正しい**: bulk assignment / projection生成 / monthly closeはいずれも`system_admin_routes`（`auth_system_admin` + CSRF + rate limit）配下に置かれており、ExecPlanのConstraintsを満たしています。
- **bulk assignmentは個別target単位でエラーを捕捉**: 1件の失敗が他のtargetの処理を止めない設計（`created`/`failed`に振り分け）で、Done Criteria「重複/不正参照を個別結果として確認できる」を満たしています。
- **anomaly判定ロジックは決定的**: `resolved`/`attendance`を事前にHashMapへロードしてから日付×ユーザーの直積をループし、`items.sort_by`でuser_id/work_date/kind順にソートしているため、テストの期待値比較が安定します。

---

## 推奨対応順序

1. **C1** を最優先で修正（認可バイパスのため、マージブロッカー）
2. **H2**（close_monthのトランザクション化）と**H4**（冪等性）はセットで対応すると効率的（どちらも`close_month`関数内の変更）
3. **H3**（エラーメッセージ漏洩）は`map_repository_error`と同様のパターンへ揃えるだけの小さい修正
4. **H1**（N+1・上限なし）は本番投入前に最低限の上限バリデーションだけでも先に入れ、セットベース化は別ExecPlanとして切り出すことを推奨
5. Medium/Lowは次回リファクタリングExecPlanにまとめて計画する
