# Operational Runbook

## Redis Failure

### Symptoms
- Increased latency in API requests (especially authenticated ones).
- Tracing shows `redis_cache_token` or `redis_is_token_active` errors.
- System fallback to PostgreSQL for token validation.

### Recovery
The system implements a graceful fallback to PostgreSQL. If Redis is down, authentication will still work but with higher DB load.

1. **Verify Redis Status**: Check if the Redis container/server is running.
2. **Restore Redis**: Restart the Redis service.
3. **Cache Re-population**: The cache will be re-populated as users log in or perform requests (cache-aside pattern).

### Configuration
Disable caching if Redis remains unstable:
`FEATURE_REDIS_CACHE_ENABLED=false`

## Database Connection Issues

### Symptoms
- `db_pool_connect` errors in logs.
- API returns 500 status codes.

### Recovery
1. **Check primary DB**: Ensure PostgreSQL is reachable.
2. **Read Replica Fallback**: If `READ_DATABASE_URL` is configured, the system will still use it for GETs. If the primary is down, writes will fail but reads might still work.
3. **Scale Pool**: Increase `DB_MAX_CONNECTIONS` if the pool is exhausted.

## Top-Level Manager Self-Approval (Requests Stuck in `pending`)

### Symptoms
- A manager whose department is a root department (`departments.parent_id IS NULL`) submits a leave or overtime request, and it stays `pending` forever.
- No other manager can approve it: `check_approval_authorization` (`backend/src/handlers/admin/common.rs`) only grants approval to a manager who oversees the applicant's department chain (`department::can_manager_approve`), and a root-department manager has no superior department to supply such a manager.
- This is an accepted design limitation, not a bug — see `docs/design-docs/department-hierarchy.md` ("既知の制約と将来の課題" → 最上位マネージャーの自己申請), which states the intended workaround is manual approval by `is_system_admin`.

### Recovery (manual approval by a system admin)
`check_approval_authorization` always authorizes `is_system_admin` users regardless of department, so only an account with `is_system_admin = true` can unblock the request.

1. **Sign in as a system admin** and open `/admin` (page title `申請承認` / "Request Approval"). This is the page that lists leave/overtime requests for approval — component `AdminRequestsSection` (i18n key `admin_components.requests.title` = `申請一覧`).
2. **Find the stuck request**: use the ユーザー (user) filter to pick the top-level manager and/or the status filter to narrow to `pending`.
   - Underlying call: `GET /api/admin/requests?status=pending&user_id=<manager_user_id>` (see `docs/design-docs/backend-api-catalog.md`).
   - Note: unlike a regular manager's list (scoped to subordinates via `list_subordinate_user_ids`), a system admin's list is unscoped (`allowed_user_ids = None` in `backend/src/handlers/admin/requests.rs::list_requests`), so the top-level manager's own request shows up here even though a manager could never approve it normally.
3. Click 詳細 (details) to review, then click 承認 (Approve) or 却下 (Reject) and enter the required comment.
   - Underlying API: `PUT /api/admin/requests/{id}/approve` or `PUT /api/admin/requests/{id}/reject` with body `{"comment": "..."}`.
   - A manager can never approve/reject their own request (`approve_request`/`reject_request` reject when `applicant_id == user.id`), so this step must be performed from a system admin account, never the applicant's own account.
4. **Verify**: reload `/admin` (or have the applicant check `/requests`, backed by `GET /api/requests/me`) and confirm the request now shows `approved` or `rejected`.

### Notes
- Attendance correction requests (`/api/admin/attendance-corrections/{id}/approve|reject`) have the same `is_system_admin` override, enforced separately by `ApproveCorrectionUseCase` / `RejectCorrectionUseCase` in `backend/src/handlers/admin/attendance_correction_requests.rs`. There is currently no admin UI section wired up for these (no route lists them), so a stuck attendance correction from a top-level manager must be resolved by calling the API directly (e.g. via `curl` with an authenticated system admin session) rather than through `/admin`.
- Longer-term fix (delegated approver / automatic escalation to system admin) is not implemented yet — tracked as `docs/exec-plans/tech-debt-tracker.md` item #11, Recommended Fix 2.
