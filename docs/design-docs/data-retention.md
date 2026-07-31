# データ保存期間ポリシー（Data Retention Policy）

**Updated:** 2026-07-31
**Project:** Timekeeper - 勤怠管理システム
**Status:** 設計確定（Design Decisions）。実装は [EP-20260731-data-retention-enforcement](../exec-plans/active/EP-20260731-data-retention-enforcement.md) で計画済み。
**親 EP:** [EP-20260704-attendance-domain-gap-backlog](../exec-plans/completed/EP-20260704-attendance-domain-gap-backlog.md)（Gap G14）
**個別 EP:** [EP-20260704-data-retention-policy](../exec-plans/completed/EP-20260704-data-retention-policy.md)
**関連タスク:** [attendance-domain-gap-tasks.md](../exec-plans/attendance-domain-gap-tasks.md) T-19

## Purpose

出勤簿・賃金台帳相当データの法定保存義務と、データ主体からの削除要求（subject request）の
優先関係を「先に文書で確定」し、誤った物理削除の実装を防ぐ。

本 doc は次を決定する（選択肢の列挙で終わらせない）。

1. データ分類ごとの保存年限・起算点・期限後の扱い（アーカイブ / 匿名化 / 物理削除）
2. subject request（削除要求）承認時の例外規定（法定保存優先データの扱い）
3. 匿名化手段として crypto-shredding（暗号鍵破棄）を採用するか
4. 現行実装への影響と、実装 EP へ引き継ぐ未確定事項

## 法令背景（設計判断の背景であり法的助言ではない）

- **労基法 109 条（記録の保存）**: 労働者名簿・賃金台帳および雇入れ・解雇・災害補償・賃金その他
  労働関係に関する重要な書類を **5 年間**（附則 143 条により「当分の間 3 年」）保存する義務。
  出勤簿・タイムカード等の打刻記録は「賃金その他労働関係に関する重要な書類」に該当し、本義務の対象。
- **起算点（労基則 56 条）**: 記録・書類の「完結の日」から起算する。出勤簿については最後の記入がされた日、
  賃金台帳については最後の記入をした日、労働者名簿については退職・解雇・死亡の日を起算日とする。
- **労基法 39 条 / 労基則 24 条の 7（年次有給休暇管理簿）**: 年休の付与・取得・基準日を記録した管理簿を
  当該期間中および期間満了後 **3 年間** 保存する義務。
- **個人情報保護法**: 保有個人データについて、本人からの利用停止・消去等の請求への対応義務。
  ただし「法令に基づく保存義務」がある場合は当該義務が優先し、削除に応じない正当理由となる。

> **実値のマスタ化（共通規約 8）**: 上表の「5 年 / 3 年 / 3 年」は法令由来の既定値だが、
> ハードコードせず **保存期間設定マスタ（新規テーブルまたは config）** に既定値として持たせ、
> 運用側（就業規則・社内規程）が上書き設定できることを本 doc の方針とする。
> 既存の `AUDIT_LOG_RETENTION_DAYS` / `CONSENT_LOG_RETENTION_DAYS`（config、既定 1825 日 = 5 年、
> `*_FOREVER` / `days=0` で無効化）が config 主導 retention の既存前例であり、これに倣う。

## 決定 1: データ分類別の保存方針

各分類の保存年限は「保存期間設定マスタの既定値」を指す。実値は運用側が設定する前提。

| # | データ分類 | 実体（テーブル / 列） | 保存年限（既定） | 起算点 | 期限後の扱い | 削除要求の扱い |
|---|-----------|----------------------|-----------------|--------|-------------|---------------|
| 1 | 打刻原本（出勤簿相当） | `attendance` | 5 年（当分の間 3 年） | 当該勤務日の記録完結日（=`work_date` の最終更新日 or 締め確定日） | 一定期間後に `archived_attendance` へアーカイブ → 保存期間満了後に匿名化/物理削除 | **法定保存優先。物理削除しない**（決定 2 参照） |
| 2 | 休憩実績 | `break_records` | 分類 1 に従属（同年限） | 従属する `attendance` の起算点 | 分類 1 と一体でアーカイブ/削除 | 法定保存優先 |
| 3 | 勤怠修正（承認済み補正値） | `attendance_corrections` | 分類 1 と同年限 | 対象勤務日の記録完結日 | 分類 1 と一体（補正値は出勤簿の「正」を構成するため分離しない） | 法定保存優先 |
| 4 | 休暇・残業申請 | `leave_requests` / `overtime_requests`（および `archived_*`） | 5 年（当分の間 3 年） | 申請対象日 / 承認・却下日 | アーカイブ → 満了後に匿名化/物理削除 | 法定保存優先（賃金・労働関係の付随記録） |
| 5 | 有給台帳 | `leave_ledger_entries`（T-04 で実装済み） | 年休管理簿相当 3 年 + 残高整合に必要な範囲 | 付与基準日から 1 年経過時（管理簿の完結） | 満了後もアーカイブ保持。**残高導出に必要な未消滅付与は保存期間より長く保持** | 法定保存優先。残高整合を壊す物理削除は不可 |
| 6 | 監査ログ | `audit_logs` | config `AUDIT_LOG_RETENTION_DAYS`（既定 1825 日 = 5 年、`FOREVER`/`0` 対応） | イベント発生時刻 | 既存の cutoff 削除で物理削除 | **対象外**（不正調査・法的請求の防御という正当利益。削除要求では消さない） |
| 7 | 同意ログ | `consent_logs` | config `CONSENT_LOG_RETENTION_DAYS`（既定 1825 日 = 5 年） | 同意記録日時 | 既存の `ConsentLogService::delete_logs_before` で物理削除 | **対象外**（同意・撤回の証跡自体は残す） |
| 8 | データ主体請求の対応記録 | `subject_requests` | 5 年（設定マスタ） | 対応完了日（approved / rejected / cancelled の日時） | 満了後に物理削除 | **対象外**（請求対応義務の証跡は残す。ただし `details` 内の PII は受理時に最小化） |
| 9 | アーカイブ済みユーザーの PII | `archived_users.full_name_enc` / `email_enc` / `email_hash` / `mfa_secret_enc` | 分類 1–4 の法定保存期間に合わせる（付随 PII） | 退職・アーカイブ日 | 満了 or 削除要求承認時に **匿名化（tombstone 化）または物理削除** | 匿名化で応える（法定保存が残る間は物理削除しない） |

補足:

- **起算点の実装方針**: 労基則 56 条の「完結の日」を厳密に個別追跡するのは実装負荷が高いため、
  第一実装では「対象月の締め確定日（`work_schedule_monthly_closures` の `closed`）または当該レコードの
  最終更新日のうち遅い方」を起算点とする。より精密な起算点が必要になった場合の拡張点として残す。
- **アーカイブと保存の関係**: `archived_*` テーブル（migration 019 / 036）は「退職者のデータ退避先」であり、
  それ自体が保存期間の終端ではない。アーカイブ済みでも法定保存期間内は保持対象。
- **PII 暗号化列（migration 031–034）**: 分類 9 の PII は KMS 封筒暗号（`*_enc`）+ 検索用ハッシュ（`email_hash`）
  として保存済み。匿名化はこれらの列を対象に行う（決定 3 参照）。

## 決定 2: subject request（削除要求）承認時の例外規定

**原則: 法定保存義務が優先されるデータは、削除要求を承認しても物理削除しない。
アクセス制限と PII 匿名化で応える。**

承認時の処理を対象データの性質で 3 層に分けることを決定する。

- **(a) 法定保存義務のあるデータ**（分類 1–5、および 6–8 の証跡）
  → 保存期間満了まで **物理削除しない**。削除要求への応答は
  「本人向けアクセス経路の停止（アクセス制限）」+「氏名・email 等の直接識別子の匿名化」で行う。
  勤怠実績そのもの（時刻・時間区分）は賃金・労働時間の証跡として保持する。
- **(b) 法定保存義務のない PII**（分類 9 の氏名・email・MFA secret 等の識別子）
  → **匿名化（tombstone 化）** で応える。`*_enc` / `email_hash` を NULL または不可逆なプレースホルダで上書きする。
- **(c) 保存期間満了後**
  → 満了したデータは purge バッチ（**別 EP**）で **物理削除** する。

### 現行 `subject_requests` 承認フローの事実（実装確認結果）

`handlers/admin/subject_requests.rs::approve_subject_request` および
`repositories/subject_request.rs::approve_subject_request` を確認した結果:

- approve は **`subject_requests` 行の `status` を `approved` に更新し、`approved_by` / `approved_at` /
  `decision_comment` を記録するだけ**。`UPDATE subject_requests SET status='approved' ...` の 1 文で完結する。
- **いかなるユーザーデータ（`users` / `attendance` / PII 列等）の削除・匿名化も行わない**。
  `request_type = delete` を承認しても、実データには一切触れない。
- 実データの物理削除は、`subject_requests` とは独立した **管理者手動フロー**
  （`DELETE /api/admin/archived-users/{id}` = アーカイブ済みユーザーの完全削除、
  復元は `POST /api/admin/archived-users/{id}/restore`）で行う分離設計になっている。
  archived-user の完全削除は `archived_*` 行の物理 DELETE。

### 上記事実の評価と影響

- **誤削除リスクは現状むしろ低い**: approve が即物理削除に直結しないため、
  「承認 = 即データ消去」による法定保存義務違反は現時点では発生しない。本ポリシーは安全側にある。
- **ただし運用ギャップがある**: 「削除要求を承認したのにデータ側で何も起きない」ため、
  データ主体への応答（アクセス制限・匿名化）が系外の手作業に依存している。
- **実装 EP へ引き継ぐ変更**（本 doc の範囲外・後続）:
  1. approve のセマンティクスを「法定保存判定を伴う匿名化ジョブの起票」へ拡張する
     （承認 → 対象データを層 (a)/(b) に分類 → (b) を匿名化、(a) はアクセス制限フラグ付与）。
  2. 保存期間設定マスタと、満了データの purge バッチを新設する。
  3. `delete` 型承認の監査証跡（誰がいつ何を匿名化したか）を `audit_logs` に残す。

## 決定 3: 匿名化手段としての crypto-shredding の可否

**決定: 現行実装では per-subject crypto-shredding を採用しない。
アプリ層の匿名化（`*_enc` / `email_hash` 列の tombstone 上書き）を第一手段とする。**

### 既存 KMS / 鍵ローテーション実装の確認結果

`utils/kms.rs` / `utils/encryption.rs` / migration 031–034 を確認した:

- PII は KMS 封筒（`kms:v1:<provider>:<key_version>:<nonce>:<ciphertext>`）として `*_enc` 列に格納。
- 鍵は **「プロバイダ × `key_version`」単位の共有鍵**。
  - `pseudo` プロバイダ: 鍵は `jwt_secret` + コンテキスト（provider id / key version / region / key id）から
    SHA-256 で導出される派生鍵。
  - `aws` / `gcp` プロバイダ: 単一の KMS キー（`AWS_KMS_KEY_ID` / `GCP_KMS_KEY_NAME`、バージョン別 env で切替）。
- `users.pii_key_version`（migration 031）はユーザー行の列だが、指すのは **共有鍵のバージョン**であり、
  ユーザー個別のデータ暗号鍵（per-subject DEK）ではない。
- 各レコードに「その行専用の鍵をラップして保存する」封筒方式にはなっていない
  （封筒は plaintext を共有鍵で直接暗号化した nonce+ciphertext のみを保持）。

### 判断

- **per-subject crypto-shredding は現設計では不可能**。1 ユーザー分の鍵を破棄する手段が存在せず、
  鍵を破棄すると **同一 `key_version` の全ユーザー PII が巻き添えで復号不能**になる。
  `pseudo` に至っては `jwt_secret` 破棄 = システム全体の破壊。
- したがって匿名化は **アプリ層で `full_name_enc` / `email_enc` / `email_hash` / `mfa_secret_enc` を
  NULL / 不可逆プレースホルダで上書き**する方式を採用する。これは分類 9 の物理列を対象に決定 2(b) を実現する。
- crypto-shredding は **将来 per-subject DEK（ユーザー毎データ鍵を KMS でラップして各行に保存する
  真の封筒方式）を導入した場合にのみ再検討**する。導入自体が別実装 EP の検討事項であり、本 doc では採用しない。

## Out of Scope（本 doc に含めない）

- 自動 purge バッチ、匿名化ジョブ、approve セマンティクス拡張、保存期間設定マスタ（テーブル/config）の実装
- 起算点の精密追跡ロジックの実装
- per-subject DEK 方式への暗号化設計変更

> **実装への接続**: G14 の方針確定は完了した。保存期間設定、削除要求の匿名化、purge は
> [EP-20260731-data-retention-enforcement](../exec-plans/active/EP-20260731-data-retention-enforcement.md)
> で実装する。

## References

- [attendance-domain-gap-tasks.md](../exec-plans/attendance-domain-gap-tasks.md) T-19
- [backend-api-catalog.md](./backend-api-catalog.md)（subject-requests / consents / archived-users 節）
- 実装: `backend/src/handlers/admin/subject_requests.rs` / `backend/src/repositories/subject_request.rs`
- 実装: `backend/src/utils/kms.rs` / `backend/src/utils/encryption.rs` / `backend/src/services/consent_log.rs`
- migration: `019_create_archived_tables.sql` / `022_create_subject_requests.sql` / `031`–`034`（PII 暗号化列）
</content>
</invoke>
