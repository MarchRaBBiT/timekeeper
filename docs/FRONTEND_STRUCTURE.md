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
