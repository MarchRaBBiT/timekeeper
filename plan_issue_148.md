# Plan: Self-service Password Reset (Issue #148)

## Overview
Implement a secure, self-service password reset flow to reduce administrative overhead. This involves database schema changes, adding email infrastructure, and new API/frontend flows.

## Current State Analysis
- **Missing Data**: `users` table lacks an `email` column.
- **Missing Infra**: No email sending capability (SMTP) in backend.
- **Missing Tables**: No storage for reset tokens.
- **Auth**: Existing `argon2` password hashing and `totp` MFA are robust and can be leveraged.

## Implementation Steps

### Phase 1: Database Schema & Models
- [ ] **Migration**: Create migration script `NNN_add_email_and_password_resets.sql`.
  - Add `email` column to `users` (TEXT UNIQUE NOT NULL - requires strategy for existing users).
  - Create `password_resets` table (id, user_id, token_hash, expires_at, created_at, used_at).
- [ ] **Models**: Update `User` struct in `backend/src/models/user.rs`.
- [ ] **Models**: Create `PasswordReset` struct in `backend/src/models/password_reset.rs`.

### Phase 2: Email Infrastructure
- [ ] **Dependencies**: Add `lettre` crate to `backend/Cargo.toml`.
- [ ] **Config**: Add SMTP settings to `backend/src/config.rs` (`SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`).
- [ ] **Service**: Implement `EmailService` in `backend/src/utils/email.rs`.
  - Support for plain text and HTML emails.
  - Mock implementation for testing.

### Phase 3: Backend API
- [ ] **Handlers**: Create `backend/src/handlers/auth/password_reset.rs`.
  - `POST /api/auth/forgot-password`:
    - Input: `email`
    - Action: Generate random token, hash it, store in DB, send email with raw token.
    - Security: Always return 200 OK even if email not found (prevent enumeration).
  - `POST /api/auth/reset-password`:
    - Input: `token`, `new_password`
    - Action: Verify token hash, check expiration/usage, update user password, mark token used.
- [ ] **Routing**: Register new routes in `backend/src/main.rs`.

### Phase 4: Frontend UI
- [ ] **Forgot Password Page**: `frontend/src/pages/forgot_password/`.
  - Simple form requesting email address.
- [ ] **Reset Password Page**: `frontend/src/pages/reset_password/`.
  - Form taking `new_password` (token from URL query param).
- [ ] **Login Integration**: Add "Forgot Password?" link to `frontend/src/pages/login/components/form.rs`.
- [ ] **API Client**: Update `frontend/src/api/client.rs`.

### Phase 5: Testing & Verification
- [ ] **Backend Tests**:
- Integration tests for the full flow using `testcontainers` and mocked email service.
  - Verify token expiration and invalidation logic.
- [ ] **Frontend Tests**: WASM tests for new pages.
- [ ] **Security Review**: Ensure raw tokens are never stored and rate limiting is applied.

## Note on Existing Users
Since `email` is being added as `NOT NULL`, existing users (like `admin`) need a default email or the migration must handle them. Strategy: Use a placeholder or update specific users in migration.
