# Frontend Technical Specification

**Version:** 1.0.0
**Last Updated:** 2026-01-23
**Status:** As-Built Specification

## 1. Introduction
This document defines the technical specification for the Timekeeper frontend application. The system is a Single Page Application (SPA) built to manage employee attendance, requests, and administrative functions.

## 2. System Architecture

### 2.1 Technology Stack
*   **Framework:** Leptos (Rust-based WebAssembly framework)
*   **Styling:** TailwindCSS
*   **Runtime:** WASM (Client-side rendering)
*   **Build Tool:** wasm-pack (Tailwind build via npm scripts)

### 2.2 Design Pattern: MVVM
The application strictly adheres to the **Model-View-ViewModel (MVVM)** architecture to ensure separation of concerns:
*   **View (`panel.rs`)**: Defines the UI structure and layout using Leptos `view!` macros. Responsible for rendering and user events.
*   **ViewModel (`view_model.rs`)**: Manages local state (`Signal`), business logic, and mediates between the View and the Repository.
*   **Repository (`repository.rs`)**: Handles data fetching and persistence via the API Client.
*   **Layout (`layout.rs`)**: Provides structural wrappers (Frames) for consistent page composition.

## 3. Routing Specification

The application uses `leptos_router` with a centralized configuration in `router.rs`.

### 3.1 Route Table

| URL Path | Component | Access Level | Description |
|----------|-----------|--------------|-------------|
| **Public Routes** | | | |
| `/` | `HomePage` | Public | Landing page. |
| `/login` | `LoginPage` | Public | User authentication entry point. |
| `/mfa/register` | `MfaRegisterPage` | Public | Initial MFA setup flow. |
| `/forgot-password` | `ForgotPasswordPage` | Public | Password recovery initiation. |
| `/reset-password` | `ResetPasswordPage` | Public | Password reset completion. |
| **Employee Routes** | | | |
| `/dashboard` | `ProtectedDashboard` | Auth Required | Main employee hub. |
| `/attendance` | `ProtectedAttendance` | Auth Required | Detailed attendance history and management. |
| `/requests` | `ProtectedRequests` | Auth Required | Leave and overtime application interface. |
| `/settings` | `ProtectedSettings` | Auth Required | User profile, security, and preferences. |
| **Admin Routes** | | | |
| `/admin` | `ProtectedAdmin` | Admin Role | System overview and global configurations. |
| `/admin/users` | `ProtectedAdminUsers` | Admin Role | Employee account management. |
| `/admin/audit-logs` | `ProtectedAdminAuditLogs`| System Admin | System security and activity logs. |
| `/admin/export` | `ProtectedAdminExport` | Admin Role | CSV data export tools. |

### 3.2 Security
*   **Route Guards**: All protected routes are wrapped in `<RequireAuth>`, which verifies the global authentication state.
*   **Redirection**: Unauthenticated access attempts to protected routes must redirect to `/login`.
*   **Role Validation**: Admin pages perform additional checks (e.g., `is_system_admin`) and render unauthorized messages if permissions are insufficient.

## 4. Component Library Specification

Shared components are defined in `src/components/` and provide the building blocks for the UI.

### 4.1 Structural Components
*   **`Layout`**: The global application shell including the `Header` navigation and main content area.
*   **`Header`**: Responsive navigation bar containing page links and user menu.
*   **`EmptyState`**: Standardized placeholder for lists with zero records.

### 4.2 Form & Interaction
*   **`Button`**: Primary interactive element.
    *   **Variants**: Primary, Secondary, Danger, Ghost.
    *   **States**: Idle, Loading (shows spinner), Disabled.
*   **`AttendanceActionButtons`**: A specialized 2x2 grid component for core actions:
    *   Clock In / Clock Out
    *   Break Start / Break End
*   **`DatePicker`**: Custom input control for selecting dates, wrapping the native browser picker.

### 4.3 Feedback & Indicators
*   **`LoadingSpinner`**: Indeterminate loading state indicator.
*   **`InlineErrorMessage`**: Form field validation error text.
*   **`ErrorMessage`**: Global/Toast style error notification.
*   **`SuccessMessage`**: Global/Toast style success notification.

### 4.4 Data Display
*   **`AttendanceCard`**: Dashboard widget showing daily work/break stats.
*   **`SummaryCard`**: Dashboard widget showing monthly aggregates.
*   **`RequestCard`**: Dashboard activity/status summaries.
*   **`UserCard`**: User profile summary card.

## 5. Page Functional Specifications

### 5.0 Home (`/`)
*   **Layout**: Full-screen hero layout
*   **Functional Requirements**:
    1.  **Branding**: Display product name and short description.
    2.  **Primary CTA**: Login button linking to `/login`.

### 5.1 Dashboard (`/dashboard`)
*   **Layout**: `DashboardFrame` (Grid System)
*   **Functional Requirements**:
    1.  **Clock**: Display current server time.
    2.  **Actions**: Provide immediate access to `AttendanceActionButtons`.
    3.  **Summary**: Show current month's work hours and days.
    4.  **Activities**: List recent user actions (logs).
    5.  **Filters**: Activity list is filterable via `GlobalFilters`.
    6.  **Alerts**: Notify of missing inputs or anomalies.

### 5.2 Attendance (`/attendance`)
*   **Layout**: `AttendanceFrame`
*   **Functional Requirements**:
    1.  **History Table**: Editable table of daily attendance records.
    2.  **Monthly Navigation**: Ability to switch between months.
    3.  **Date Range Filter**: Custom range selection with validation errors.
    4.  **CSV Export**: Export attendance for selected range.
    5.  **Holiday Alerts**: Warning banner for upcoming holidays, refreshable.
    6.  **Correction**: Inline editing of Clock In/Out times (if permitted).

### 5.3 Requests (`/requests`)
*   **Layout**: `RequestsLayout`
*   **Functional Requirements**:
    1.  **Request Submission**: Dedicated forms for "Leave" and "Overtime".
    2.  **Responsive Design**: Mobile uses toggle view; Desktop uses split view.
    3.  **Edit/Cancel**: Existing requests can be edited or cancelled.
    4.  **Status Tracking**: List showing Pending/Approved/Rejected status.
    5.  **Details**: Modal view for full request context.

### 5.4 Administration (`/admin`)
*   **Layout**: `AdminDashboardScaffold`
*   **Functional Requirements**:
    1.  **Holiday Management**: Manage standard and Google Calendar holidays.
    2.  **Approval Queue**: Centralized list of requests requiring action.
    3.  **System Tools**: Utilities for data correction and MFA resets.

### 5.5 User Management (`/admin/users`)
*   **Functional Requirements**:
    1.  **Tabbed Interface**: Separate views for "Active Users" and "Archived (Retired) Users".
    2.  **User List**: Filterable list of active employees.
    3.  **Onboarding**: `InviteForm` to create new accounts (Active tab).
    4.  **Archiving**: Mechanism to archive users (move to Archived tab) or restore them.
    5.  **Editing**: Slide-over drawer for updating user roles and details.

### 5.6 Audit Logs (`/admin/audit-logs`)
*   **Layout**: `Layout` (Full width)
*   **Functional Requirements**:
    1.  **Filtering**: Multi-faceted search by Date Range, User ID, Event Type, and Result (Success/Failure).
    2.  **Visualization**: Colored badges for result status.
    3.  **Details**: Modal viewer for complex JSON metadata associated with log events.
    4.  **Pagination**: Server-side pagination controls.
    5.  **Export**: Ability to export the current filtered log set to JSON.

### 5.7 Data Export (`/admin/export`)
*   **Layout**: `Layout`
*   **Functional Requirements**:
    1.  **Scope Selection**: Export data for "All Users" or a "Specific User".
    2.  **Date Range**: `DatePicker` controls for defining the export period.
    3.  **Preview**: Real-time text preview of the CSV output (truncated to first 2KB).
    4.  **Download**: Generates and triggers download of the full CSV file.

### 5.8 Authentication & Security
*   **Login (`/login`)**: Standard email/password form with error feedback.
*   **Password Recovery**:
    *   `/forgot-password`: Email entry to request reset link.
    *   `/reset-password`: Form to define new password (token based).
*   **MFA Setup (`/mfa/register`)**: QR Code display for authenticator apps and TOTP verification step.

### 5.9 Settings (`/settings`)
*   **Layout**: `Layout`
*   **Functional Requirements**:
    1.  **Password Change**: Current/new/confirm flow with validation and success/error feedback.
    2.  **MFA Management**: Reuses MFA setup and verification components with status refresh.
    3.  **Subject Requests**: Create data subject requests (Access/Rectify/Delete/Stop).
    4.  **History**: List subject request history with cancel action for pending items.

## 6. Data & State Management
*   **Global State**: Authentication user profile is held in a global context.
*   **Local State**: Page-specific data (forms, list filters) is managed via `RwSignal` in ViewModels.
*   **API Integration**: All backend communication occurs via the centralized `ApiClient` in `src/api/client.rs`.

## 7. Theme Feature Plan

This section records the initial plan for implementing the screen theme feature. It will be refined later.

### 7.1 色指定部品の意味 (現状)

| Component / Location | Meaning (Semantic Role) | Notes |
|---|---|---|
| `frontend/src/components/common.rs` | Button variants for primary/secondary/danger/ghost actions | Uses `brand`, `gray`, `red` with `dark:` overrides |
| `frontend/src/components/layout.rs` | App surface, header, nav links, warning banner, success/error messages, loading indicator | Global UI tone and status feedback |
| `frontend/src/components/error.rs` | Inline form error feedback | Form validation error emphasis |
| `frontend/src/components/empty_state.rs` | Empty state placeholder | Neutral and subdued presentation |
| `frontend/src/components/forms.rs` | Attendance status, action buttons, holiday alert, processing, success/error messages | Status-specific colors (brand/amber/red/gray) |
| `frontend/src/pages/admin_audit_logs/panel.rs` | Result badges and link emphasis | Success vs failure badge colors |
| `frontend/src/pages/admin/components/requests.rs` | Approve/reject buttons and modal overlay | Success/danger actions and overlay tone |
| `frontend/src/pages/admin/components/holidays.rs` | Primary/confirm actions for holiday ops | Primary and success-like actions |
| `frontend/src/pages/settings/panel.rs` | Form labels, primary submit, danger actions | Text hierarchy and action emphasis |

### 7.2 意味と色のマッチング (現状)

| Meaning | Light Colors | Dark Colors | Examples |
|---|---|---|---|
| Primary action / brand | `brand-600/700` | `brand-500/400` | Primary button, key status |
| Secondary action / neutral | `gray-600/700` | `gray-700/600` | Secondary button |
| Danger / error | `red-600/700`, `red-50/200` | `red-500/400`, `red-900/700` | Error banners, reject actions |
| Success | `green-600/700`, `green-50/200` | `green-500/400`, `green-900/700` | Success banners |
| Warning / attention | `yellow-50/200/900`, `amber-50/100/800` | `yellow-900/700`, `amber-900/500` | Warning banners, holiday alerts |
| Informational / link | `blue-600` | `blue-400` | Links, some CTA |
| Surface / background | `bg-gray-50`, `bg-white` | `bg-gray-900`, `bg-gray-800` | App shell, cards |
| Overlay | `bg-black/30` | `bg-black/80` | Modals |
| Attendance status | brand/amber/red/gray | brand/amber/red/gray | Clock in/out/break |

### 7.3 実装計画

1. Define semantic color tokens (surface, text, border, action, status, overlay) and document the mapping.
2. Implement a CSS variable-based palette for Light/Dark (and optional System) themes.
3. Update Tailwind configuration to map semantic tokens to utilities (e.g., `bg-surface`, `text-primary`).
4. Introduce a theme switcher in global state (System/Light/Dark) and apply it via `class` or `data-theme`.
5. Replace hard-coded colors in shared components first, then page-level components.
6. Normalize status colors (success/warn/error) and attendance state colors across screens.
7. Validate contrast and readability on key pages (Dashboard/Attendance/Requests/Admin).

### 7.4 ダークモードの現状

- `frontend/tailwind.config.js` uses `darkMode: 'media'` and many components rely on `dark:` classes.
- Existing hard-coded dark styles should be replaced with semantic tokens while preserving current behavior.

### 7.5 トークン一覧 (案)

| Token | Meaning / Usage | Light Value | Dark Value | References (Current) |
|---|---|---|---|---|
| `surface.base` | App background | `gray-50` | `gray-900` | `frontend/src/components/layout.rs` |
| `surface.elevated` | Card / panel background | `white` | `gray-800` | `frontend/src/pages/**` |
| `surface.muted` | Subtle section background | `gray-50` | `gray-700` | tables, filters |
| `text.primary` | Primary text | `gray-900` | `gray-100` | headings, main text |
| `text.secondary` | Secondary text | `gray-700` | `gray-300` | labels, body |
| `text.muted` | Muted text | `gray-500` | `gray-400` | helper, placeholders |
| `text.inverse` | Text on strong backgrounds | `white` | `white` | primary/danger buttons |
| `border.subtle` | Default border | `gray-200` | `gray-700` | cards, tables |
| `border.strong` | Emphasized border | `gray-300` | `gray-600` | inputs, separators |
| `form.control.bg` | Input background | `white` | `gray-700` | inputs/selects |
| `form.control.text` | Input text | `gray-900` | `gray-100` | inputs/selects |
| `form.control.border` | Input border | `gray-300` | `gray-600` | inputs/selects |
| `form.control.placeholder` | Input placeholder | `gray-500` | `gray-400` | inputs/selects |
| `action.primary.bg` | Primary action background | `brand-600` | `brand-500` | primary button |
| `action.primary.bg-hover` | Primary action hover | `brand-700` | `brand-400` | primary button hover |
| `action.primary.text` | Primary action text | `white` | `white` | primary button |
| `action.primary.border` | Primary action border | `brand-600` | `brand-500` | primary button border |
| `action.primary.border-hover` | Primary action border hover | `brand-700` | `brand-400` | primary button border hover |
| `action.primary.focus` | Primary action focus | `brand-600` | `brand-500` | focus outline |
| `action.secondary.bg` | Secondary action background | `gray-600` | `gray-700` | secondary button |
| `action.secondary.bg-hover` | Secondary action hover | `gray-700` | `gray-600` | secondary button hover |
| `action.secondary.text` | Secondary action text | `white` | `white` | secondary button |
| `action.secondary.border` | Secondary action border | `gray-600` | `gray-700` | secondary button border |
| `action.secondary.border-hover` | Secondary action border hover | `gray-700` | `gray-600` | secondary button border hover |
| `action.secondary.focus` | Secondary action focus | `gray-600` | `gray-500` | focus outline |
| `action.danger.bg` | Danger action background | `red-600` | `red-500` | danger button |
| `action.danger.bg-hover` | Danger action hover | `red-700` | `red-400` | danger button hover |
| `action.danger.text` | Danger action text | `white` | `white` | danger button |
| `action.danger.border` | Danger action border | `red-600` | `red-500` | danger button border |
| `action.danger.border-hover` | Danger action border hover | `red-700` | `red-400` | danger button border hover |
| `action.danger.focus` | Danger action focus | `red-600` | `red-500` | focus outline |
| `action.ghost.bg-hover` | Ghost hover background | `gray-100` | `gray-700` | ghost button |
| `action.ghost.text` | Ghost text | `gray-900` | `gray-100` | ghost button |
| `state.disabled.bg` | Disabled background | `gray-100` | `slate-800` | disabled buttons |
| `state.disabled.text` | Disabled text | `gray-400` | `slate-500` | disabled buttons |
| `state.disabled.border` | Disabled border | `gray-200` | `slate-700` | disabled buttons |
| `status.success.bg` | Success background | `green-50` | `green-900/30` | success banners |
| `status.success.border` | Success border | `green-200` | `green-700` | success banners |
| `status.success.text` | Success text | `green-700` | `green-200` | success banners |
| `status.error.bg` | Error background | `red-50` | `red-900/30` | error banners |
| `status.error.border` | Error border | `red-200` | `red-700` | error banners |
| `status.error.text` | Error text | `red-700` | `red-200` | error banners |
| `status.warning.bg` | Warning background | `amber-50` | `amber-900/20` | warning banners |
| `status.warning.border` | Warning border | `amber-100` | `amber-900/30` | warning banners |
| `status.warning.text` | Warning text | `amber-800` | `amber-200` | warning banners |
| `status.info.bg` | Info background | `blue-50` | `blue-900/30` | info banners |
| `status.info.border` | Info border | `blue-200` | `blue-700` | info banners |
| `status.info.text` | Info text | `blue-700` | `blue-200` | info banners |
| `status.neutral.bg` | Neutral badge background | `gray-100` | `gray-700` | status badges |
| `status.neutral.border` | Neutral badge border | `gray-200` | `gray-600` | status badges |
| `status.neutral.text` | Neutral badge text | `gray-800` | `gray-100` | status badges |
| `overlay.backdrop` | Modal overlay | `black/30` | `black/80` | modals |
| `link.default` | Link / emphasis | `blue-600` | `blue-400` | links |
| `link.hover` | Link hover | `blue-800` | `blue-300` | links |
| `status.attendance.clock_in` | Attendance: clock in | `brand-600` | `brand-500` | `frontend/src/components/forms.rs` |
| `status.attendance.break` | Attendance: break | `amber-600` | `amber-400` | `frontend/src/components/forms.rs` |
| `status.attendance.clock_out` | Attendance: clock out | `slate-400` | `slate-600` | `frontend/src/components/forms.rs` |
| `status.attendance.not_started` | Attendance: not started | `slate-300` | `slate-600` | `frontend/src/components/forms.rs` |
| `status.attendance.text-active` | Attendance: active text | `slate-900` | `white` | `frontend/src/components/forms.rs` |
| `status.attendance.text-inactive` | Attendance: inactive text | `slate-500` | `slate-400` | `frontend/src/components/forms.rs` |

### 7.6 調整メモ

- Added missing action tokens for danger/secondary/primary borders and focus outlines.
- Added disabled state tokens to cover repeated gray/slate disabled styles.
- Unified warning palette to amber for `status.warning.*` (current yellow usage should be migrated).
- Added neutral badge tokens for gray status labels.
- Added form control tokens for inputs and selects.
- Moved attendance colors under `status.attendance.*` to keep all special colors under `status.*`.
