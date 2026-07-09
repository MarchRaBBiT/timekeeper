-- M-5 (docs/design-docs/backend-api-catalog.md): 053 の
-- `leave_ledger_grant_lot_fields` CHECK は kind='grant' の行にしか
-- granted_at/expires_at/grant_base_date の必須化を強制していなかった。
-- kind='adjust' には 2 通りの正当な形がある:
--   (a) 既存ロットへの調整: lot_id が既存の grant/adjust 行を参照し、
--       granted_at/expires_at/grant_base_date は NULL のまま
--       (crates/app/src/leave_ledger.rs::build_adjust_entry の `Some(lot_id)` 枝)。
--   (b) 新規ロット投入（初期移行等）: granted_at/expires_at/day_equivalent_minutes
--       が必須、grant_base_date は任意
--       (同 `None` 枝。grant_base_date は年5日義務の window 起算日で、
--       義務追跡対象外のロットでは意図的に省略できる仕様のため必須にしない)。
--
-- 制約の限界:
-- 「lot_id が既存の grant/adjust 行を実際に参照しているか」は、対象行が
-- 挿入される時点で同一テーブルの既存行を参照する必要があり、単一行の
-- CHECK 制約（同じ行の列だけを見る）では表現できない。これを DB 側で
-- 完全に強制するには、他行を SELECT する制約トリガー（deferrable constraint
-- trigger）が必要になる。今回はそこまでは行わず、CHECK で表現できる範囲の
-- 部分的な不変条件のみを追加する:
--   「kind='adjust' かつ granted_at IS NOT NULL（=新規ロットのつもり）なら、
--    expires_at も day_equivalent_minutes も NOT NULL でなければならない」
-- grant_base_date は上記の理由で意図的にこの CHECK の対象外にする
-- （app 層の build_adjust_entry の実際の必須項目と食い違わないようにするため）。
-- expires_at > granted_at の順序は既存の `leave_ledger_expiry_after_grant`
-- (053) がすでに強制している。
--
-- 既存データが新しい制約に違反しないことの確認クエリ（適用前に実行して
-- 0 件であることを確認する想定。過去データは無いはずだが、移行環境で
-- 事前に流しておくこと）:
--   SELECT id FROM leave_ledger_entries
--   WHERE kind = 'adjust' AND granted_at IS NOT NULL
--     AND (expires_at IS NULL OR day_equivalent_minutes IS NULL);

ALTER TABLE leave_ledger_entries
ADD CONSTRAINT leave_ledger_adjust_new_lot_fields CHECK (
    kind <> 'adjust'
    OR granted_at IS NULL
    OR (expires_at IS NOT NULL AND day_equivalent_minutes IS NOT NULL)
);
