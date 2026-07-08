-- T-03 (attendance-calculation-policy.md Decision 11):
-- 就業規則マスタ。組織全体で 1 系列の effective-dated 行。
-- 区分計算は対象日時点で有効な行を参照し、行が存在しない期間は fail-closed とする。
CREATE TABLE work_rule_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    valid_from DATE NOT NULL UNIQUE,
    statutory_daily_minutes INTEGER NOT NULL CHECK (statutory_daily_minutes > 0),
    statutory_weekly_minutes INTEGER NOT NULL CHECK (statutory_weekly_minutes > 0),
    night_start TIME NOT NULL,
    night_end TIME NOT NULL,
    -- ISO weekday: 1 = Monday .. 7 = Sunday
    week_start_weekday SMALLINT NOT NULL CHECK (week_start_weekday BETWEEN 1 AND 7),
    legal_holiday_weekday SMALLINT NOT NULL CHECK (legal_holiday_weekday BETWEEN 1 AND 7),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 既定値 seed（法定労働時間 8h/日・40h/週、深夜帯 22:00-5:00、週起算 日曜、法定休日 日曜）。
-- 実値の設定責任は運用側にある（法定値のハードコード回避のための既定行）。
INSERT INTO work_rule_settings (
    valid_from,
    statutory_daily_minutes,
    statutory_weekly_minutes,
    night_start,
    night_end,
    week_start_weekday,
    legal_holiday_weekday
) VALUES ('1900-01-01', 480, 2400, '22:00:00', '05:00:00', 7, 7);
