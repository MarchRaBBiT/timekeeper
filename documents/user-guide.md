# Timekeeper User Guide

This guide summarizes the usage flows for employees, administrators, and system administrators. It is based on `README.md`, `API_DOCS.md`, and the latest admin panel specifications.

## 1. Roles and Permissions

| Role | Main Permissions |
| --- | --- |
| Employee (`role=employee`) | Clock in/out, break recording, create leave/overtime/correction requests, view history, MFA registration/removal, password change, export own data |
| Administrator (`role=admin`, `is_system_admin=false`) | All employee functions plus: approve/reject overtime/leave/correction requests, holiday management, CSV export |
| System Administrator (`is_system_admin=true`) | All admin functions plus: user management, direct attendance data editing, force break end, MFA reset, holiday management, CSV export, password reset, etc. |

> MFA reset and password reset must be performed by a system administrator.

## 2. Login and MFA

1. Access the frontend (default: `http://localhost:8000`) and navigate to `/login`.
2. Enter username and password. If MFA is enabled, a 6-digit TOTP code is also required.
3. Upon successful login, access and refresh tokens are stored in the browser's `localStorage`.

### MFA Registration Procedure

1. Navigate to `/mfa/register` and press the **MFA Setup** button to generate a QR code and secret.
2. Scan the QR code with an authenticator app and enter the displayed confirmation code.
3. To disable, submit the current TOTP code on the same page.
4. If you lose your device, request **Admin -> MFA Reset** from a system administrator and re-register.

## 3. Basic Operations for Employees

### Attendance Actions

| Operation | UI | API |
| --- | --- | --- |
| Clock In | Dashboard -> **Clock In** | `POST /api/attendance/clock-in` |
| Break Start/End | Attendance page buttons | `POST /api/attendance/break-start` / `POST /api/attendance/break-end` |
| Clock Out | Dashboard -> **Clock Out** | `POST /api/attendance/clock-out` |
| Check Status/History | Attendance -> "Today's Status / History" | `GET /api/attendance/status`, `GET /api/attendance/me` |
| CSV Export | Attendance -> "CSV Export" card | `GET /api/attendance/export` |

### Request Flow (Leave/Overtime/Correction)

1. Select request type on `/requests`, enter period and reason, and submit.
2. Status (`pending` / `approved` / `rejected` / `cancelled`) can be checked on the same page.
3. While `pending`, editing and cancellation are possible.

## 4. Administrator (Approver)

Users with `role=admin` see **Admin** in the header and can access the `/admin` dashboard.

1. **Request List** - Filter by status or user, and approve/reject with comments from the detail modal.
   - Approve: `PUT /api/admin/requests/:id/approve`
   - Reject: `PUT /api/admin/requests/:id/reject`
2. **Detail Modal** - Review JSON payload before approval.
3. **Holiday Management and CSV Export** - Add/delete holidays, import from Google Calendar, and download attendance CSV from cards at the bottom of the page.

Non-system administrator admins can use the above functions, but user management, manual attendance editing, and MFA reset remain system administrator exclusive.

## 5. System Administrator Console

System administrators have **User Management** added to navigation and can use the following additional tools on `/admin` (holiday management and CSV export are also available to regular admins).

### 5.1 User Management (`/admin/users`)

- Create employees/admins with `POST /api/admin/users`. Initial password is set here.
- **System Administrator** checkbox grants highest tier privileges.
- View username/full name/role/system admin flag in the user table.

### 5.2 Manual Attendance Data Editing

- Use the "Manual Attendance Registration (Upsert)" form to call `PUT /api/admin/attendance` and directly register/overwrite user ID, date, clock in/out, and break intervals.
- "Force Break End" specifies a break ID and calls `PUT /api/admin/breaks/:id/force-end` to immediately end an unfinished break.

### 5.3 MFA Reset

- Select a target from the user list obtained via `/api/admin/users` and press the **Reset MFA** button to execute `POST /api/admin/mfa/reset`.
- After reset, all refresh tokens for that user are also revoked, requiring re-login.

### 5.4 Holiday Management

- (Common to admins/system admins) In addition to manual addition (`POST /api/admin/holidays`), holiday candidates can be imported from Google Calendar ICS via `GET /api/admin/holidays/google`.
- Existing holidays can be deleted with `DELETE /api/admin/holidays/:id`.

### 5.5 CSV Export

- (Common to admins/system admins) `/admin/export` allows downloading attendance CSV filtered by username and date range (`GET /api/admin/export`).
- The beginning of the downloaded file is also previewed on screen.

## 6. Logout and Password

- Call `POST /api/auth/logout` from the **Logout** button in the header. Sending `{"all": true}` invalidates refresh tokens on all devices at once.
- Password change is executed via `/api/auth/change-password` (minimum 8 characters, must differ from current password).

## 7. Troubleshooting

1. **401 Unauthorized** - Token expired/missing. Try re-login or refresh.
2. **403 Forbidden** - Possibly executing system admin-only functions as a regular admin.
3. **Migration Inconsistency** - Apply latest schema with `cargo run` or `cargo sqlx migrate run`.
4. **Frontend Not Displaying** - Re-run `wasm-pack build --dev` and restart Python server. Set `FRONTEND_BASE_URL` appropriately when running Playwright.

For API details, see `API_DOCS.md`. For environment setup details, see `documents/environment-setup.md`.
