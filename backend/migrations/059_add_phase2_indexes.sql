-- Phase2 review follow-up: indexes for FK columns and hot query paths that
-- were missed when 058_create_monthly_closing_workflows.sql and the
-- overtime-monitor read paths were introduced. Existing migrations are
-- immutable, so these are added here instead of editing 058.

-- monthly_closing_workflows.*_by are nullable FKs to users(id) ON DELETE
-- RESTRICT. Without an index, deleting a user forces a full table scan of
-- monthly_closing_workflows for each FK column to check for restricted
-- references. Partial indexes (WHERE ... IS NOT NULL) keep them small since
-- most rows only have self_confirmed_by/approved_by populated over time.
CREATE INDEX idx_monthly_closing_workflows_self_confirmed_by
    ON monthly_closing_workflows (self_confirmed_by)
    WHERE self_confirmed_by IS NOT NULL;
CREATE INDEX idx_monthly_closing_workflows_approved_by
    ON monthly_closing_workflows (approved_by)
    WHERE approved_by IS NOT NULL;
CREATE INDEX idx_monthly_closing_workflows_closed_by
    ON monthly_closing_workflows (closed_by)
    WHERE closed_by IS NOT NULL;
CREATE INDEX idx_monthly_closing_workflows_reopened_by
    ON monthly_closing_workflows (reopened_by)
    WHERE reopened_by IS NOT NULL;

-- monthly_closing_workflow_events.workflow_id is a NOT NULL FK with
-- ON DELETE CASCADE; also supports the (future) per-workflow audit history
-- listing ordered by created_at. acted_by is a NOT NULL FK to users(id) ON
-- DELETE RESTRICT and needs an index for the same FK-scan reason as above.
CREATE INDEX idx_monthly_closing_workflow_events_workflow_id_created_at
    ON monthly_closing_workflow_events (workflow_id, created_at);
CREATE INDEX idx_monthly_closing_workflow_events_acted_by
    ON monthly_closing_workflow_events (acted_by);

-- overtime_requests has no index covering `date`, but
-- list_approved_overtime() filters
-- WHERE user_id = ANY($1) AND status = 'approved' AND date BETWEEN $2 AND $3
-- which otherwise falls back to idx_overtime_requests_user_id (or a seq scan)
-- and post-filters status/date in Postgres. A partial index on the approved
-- subset covers this predicate directly.
CREATE INDEX idx_overtime_requests_user_id_date_approved
    ON overtime_requests (user_id, date)
    WHERE status = 'approved';
