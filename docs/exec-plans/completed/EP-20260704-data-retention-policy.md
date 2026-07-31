# EP-20260704-data-retention-policy

**Updated:** 2026-07-04
**Kind:** 設計のみ（design doc 確定）。実装は Out。
**親 EP:** [EP-20260704-attendance-domain-gap-backlog](./EP-20260704-attendance-domain-gap-backlog.md)（Gap G14）
**タスク:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-19
**成果物:** [docs/design-docs/data-retention.md](../../design-docs/data-retention.md)

## Goal
- 出勤簿・賃金台帳相当データの法定保存（労基法 109 条: 5 年、当分の間 3 年）と、
  subject request（削除要求）承認の優先関係を design doc で確定し、誤った物理削除の実装を防ぐ
- データ分類別の保存年限・起算点・期限後の扱い（アーカイブ / 匿名化 / 物理削除）を「決定」として固定する
- crypto-shredding を匿名化手段に使えるかを既存 KMS 実装の確認に基づき判断する

## Scope
- In: `docs/design-docs/data-retention.md` の新規作成（設計判断のみ）、既存実装
  （`handlers/admin/subject_requests.rs`・`repositories/subject_request.rs`・`utils/kms.rs`・
  `utils/encryption.rs`・migration 019/022/031–034）の挙動確認と影響洗い出し
- Out: 自動 purge バッチ・匿名化ジョブ・approve セマンティクス拡張・保存期間設定マスタ（テーブル/config）の
  **実装**、migration 追加、コード変更、per-subject DEK 方式への暗号化設計変更

## Done Criteria (Observable)
- [x] `docs/design-docs/data-retention.md` が存在し、分類別（attendance / break / correction /
      leave・overtime request / 将来 leave ledger / audit_logs / consent_logs / subject_requests /
      archived_users PII）の保存年限・起算点・期限後の扱いが決定として記述されている
- [x] 労基法 109 条（5 年、当分の間 3 年）を出勤簿相当データの背景として明記し、実値は設定マスタ化する方針が書かれている
- [x] subject request 承認時の例外規定（法定保存優先データは物理削除せず匿名化・アクセス制限で応える）が決定されている
- [x] 現行 `approve_subject_request` の実挙動（status 更新のみ・データ削除しない）と影響が洗い出されている
- [x] crypto-shredding の可否が既存 KMS 実装確認に基づき判断されている（＝現設計では不採用）
- [x] 実装は別 EP として親 EP へ追記する旨が明記されている
- [x] `bash scripts/harness.sh docs-check` が green

## Constraints / Non-goals
- 本 EP はコード変更・migration 追加を行わない（docs のみ）
- 法令記述は設計判断の背景であり法的助言ではない。実値（保存年限）はマスタ設定として外部化する前提
- `.agent/PLANS.md` / `attendance-domain-gap-tasks.md` / 既存ファイルは編集しない（新規ファイル 2 つのみ）

## Task Breakdown
1. [x] 既存実装の確認（subject_requests approve フロー / archived-users 完全削除 / KMS 封筒 / PII 暗号化列 / 既存 config retention）
2. [x] `docs/design-docs/data-retention.md` に分類別保存方針・subject request 例外規定・crypto-shredding 判断を決定として記述
3. [x] 個別 EP（本ファイル）を `.agent/PLANS.md` テンプレート形式で作成
4. [x] （後続・別 EP）保存期間設定マスタ + purge バッチ + approve 匿名化ジョブを [EP-20260731-data-retention-enforcement](../active/EP-20260731-data-retention-enforcement.md) として登録

## Validation Plan
- [x] `bash scripts/harness.sh docs-check`
- コード変更なしのため fmt / clippy / test は対象外

## Git Snapshot Log
- [x] `git status --short`
- [x] `docs-check` pass
- [x] `git commit -m "docs: add data retention policy design (T-19)"`（`8d5101a`）

## Progress Notes

- 2026-07-31: 設計 commit `8d5101a` と後続実装 EP の登録を確認し、設計 EP を完了扱いとした。
- 2026-07-04: 実装確認に基づき `data-retention.md` を作成。
  - 決定 1: 分類別保存方針表（9 分類）。労基法 109 条 5 年（当分の間 3 年）・労基則 24 条の 7 管理簿 3 年を
    背景として明記し、実値は保存期間設定マスタ（既存の `*_RETENTION_DAYS` config を前例に）へ外部化する方針。
  - 決定 2: subject request 承認例外規定を 3 層（法定保存データ=物理削除せずアクセス制限+匿名化 /
    法定義務なし PII=tombstone 匿名化 / 満了後=purge）で確定。
  - 実装確認: `approve_subject_request` は `subject_requests.status` を `approved` に更新するのみで、
    ユーザーデータの削除・匿名化を一切行わない（実データ削除は archived-users 手動フローで分離）。
    誤削除リスクは低いが「承認したのにデータ側で何も起きない」運用ギャップを実装 EP で埋める必要がある。
  - 決定 3: KMS 封筒は「プロバイダ × key_version」共有鍵（`pseudo` は jwt_secret 由来、`aws`/`gcp` は単一 KMS 鍵）で
    per-subject DEK が無いため、per-subject crypto-shredding は不可能と判断。匿名化は `*_enc`/`email_hash` 列の
    tombstone 上書きを第一手段とし、crypto-shredding は将来 per-subject DEK 導入時のみ再検討。
  - 実装（purge/匿名化/approve 拡張）は Out。方針確定後に別 EP として親 EP へ追記する。
</content>
