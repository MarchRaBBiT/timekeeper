# Timekeeper Rebuild Architecture

**Status:** Proposed baseline for a from-scratch rebuild
**Updated:** 2026-06-10
**Scope:** package selection, module boundaries, and harness implications

## Decision Summary

If Timekeeper were rebuilt from zero, use a Rust modular monolith:

- Backend: Axum + SQLx + PostgreSQL
- Frontend: Leptos CSR/WASM, upgraded to the current Leptos line during rebuild
- Auth: opaque server-side session cookies for browser users; JWT only for external API/service tokens
- Background work: first-class worker binary using the same application crates
- API contract: code-first OpenAPI plus shared Rust DTOs in a contract crate
- Harness: one executable entrypoint in `scripts/harness.sh`, with docs/architecture checks treated as a first-class stage

The goal is not to replace the running product immediately. The goal is to give future agents a stable target architecture so they do not continue widening existing handler/client-centric structure.

## Non-Goals

- Do not split into microservices for the initial rebuild.
- Do not keep SQLite compatibility.
- Do not make Redis mandatory at bootstrap.
- Do not preserve the current route/module layout if it conflicts with clearer domain boundaries.
- Do not migrate all current functionality in one PR.

## Target Workspace

```text
timekeeper/
  apps/
    api/                 # Axum HTTP adapter, routing, middleware
    web/                 # Leptos frontend
    worker/              # cleanup, email, imports, audit export jobs
  crates/
    domain/              # pure business model and policies
    app/                 # use cases and transaction orchestration
    contract/            # request/response DTOs and OpenAPI schemas
    infra-postgres/      # SQLx repositories and migrations-facing code
    infra-security/      # password, session, CSRF, MFA, PII encryption
    infra-integrations/  # email, Google Calendar, AWS KMS/S3
    observability/       # tracing and OpenTelemetry setup
  e2e/
  docs/
  scripts/
```

## Package Choices

| Area | Package | Decision |
| --- | --- | --- |
| HTTP | `axum`, `tower`, `tower-http`, `tokio` | Keep. These fit a Rust API server and middleware-heavy app. |
| Database | `sqlx` with PostgreSQL features only | Keep SQL explicit and compile/test repository queries against PostgreSQL. |
| API docs | `utoipa` or `aide` | Prefer `utoipa` for continuity unless route-first generation becomes materially better with `aide`. |
| Validation | `validator` or `garde` | Validate DTOs at boundaries; represent invariants in domain value objects. |
| Session/auth | `tower-sessions`, `argon2`, `totp-rs`, `rand` | Browser auth should use opaque sessions in HttpOnly Secure cookies. |
| Secrets/PII | `aes-gcm`, `aws-sdk-kms`, `secrecy`, `zeroize` | Keep encryption explicit and prevent accidental secret logging. |
| Time | `chrono`, `chrono-tz` | Keep for SQLx compatibility; wrap in domain types such as `WorkDate`. |
| Observability | `tracing`, `tracing-subscriber`, `opentelemetry`, `tracing-opentelemetry`, `opentelemetry-otlp` | Make request/audit/DB latency correlation a baseline concern. |
| Frontend | `leptos`, `leptos_router`, `leptos_meta`, Tailwind CSS | Keep Rust-first frontend, but rebuild against the current Leptos line. |
| E2E | `@playwright/test` | Replace ad-hoc Node scripts with a runner, fixtures, and traces. |

## Architectural Boundaries

### `crates/domain`

Owns business concepts and invariants. It must not depend on Axum, SQLx, Leptos, or browser APIs.

Examples:

- `User`, `Role`, `Department`, `Permission`
- `WorkDate`, `WorkTime`, `AttendanceDay`, `BreakPeriod`
- `LeaveRequest`, `OvertimeRequest`, `AttendanceCorrection`
- Policies such as holiday clock-in denial, self-approval denial, and pending-only cancellation

### `crates/app`

Owns use cases and transaction orchestration. Each externally visible workflow should have one obvious entrypoint.

Examples:

- `ClockIn`
- `ClockOut`
- `StartBreak`
- `SubmitLeaveRequest`
- `ApproveRequest`
- `ChangePassword`
- `RotatePiiKey`

Use cases accept repositories, clock, identity, and integration traits. They return domain or contract-level results, not Axum responses.

### `crates/contract`

Owns API-facing request and response types. This is the shared boundary between backend and frontend.

Rules:

- No database row types.
- No frontend signal/view types.
- OpenAPI schema annotations live here when possible.
- Backward-compatible additions are preferred over route-specific ad-hoc JSON.

### `crates/infra-postgres`

Owns SQLx query implementation, migrations-facing structs, and repository implementations.

Rules:

- Query code belongs here, not in HTTP handlers.
- Migrations are append-only.
- PostgreSQL constraints should enforce durable business invariants where practical.
- Read-replica support is a later feature, not an initial architecture dependency.

### `apps/api`

Owns HTTP adaptation only.

Rules:

- Handlers parse extractors, call one use case, and translate the result.
- Handlers do not build CSV, perform complex SQL, or decide business policy.
- Route modules are grouped by capability: `auth`, `attendance`, `requests`, `admin`, `compliance`.
- CSRF, session extraction, request IDs, tracing, and rate limiting live as Tower/Axum middleware.

### `apps/web`

Owns user experience and browser state.

Rules:

- Feature folders own views, view models, and repositories.
- API calls go through feature clients generated or typed from `crates/contract`.
- Avoid one global API client with every endpoint.
- Shared components remain small and semantic: buttons, forms, tables, dialogs, empty states, layout.

### `apps/worker`

Owns asynchronous and scheduled work.

Examples:

- token/session cleanup
- retention cleanup
- password reset email delivery
- lockout notification delivery
- Google Calendar holiday import
- audit export/archive

Initial implementation can poll PostgreSQL. Redis or a queue can be added only when operational evidence justifies it.

## Data and Auth Model

PostgreSQL is the only supported primary database in the rebuild.

Use opaque sessions for browser users:

- Browser stores only HttpOnly Secure SameSite cookies.
- Server stores session records and rotation metadata.
- CSRF protection applies to cookie-authenticated mutations.
- JWT remains available only for non-browser API clients or service-to-service tokens.

This removes the current need to validate every browser request through active JWT tables plus optional Redis token caching.

## Migration Strategy

1. Build target crates and empty app shells.
2. Move API DTOs into `crates/contract` without changing wire format.
3. Extract domain value objects and policies from existing handlers/services.
4. Move one workflow at a time into `crates/app`, starting with attendance clock-in/out.
5. Move SQLx code into `crates/infra-postgres` behind repository traits.
6. Split backend routes into capability modules.
7. Rebuild frontend feature clients from `contract`.
8. Convert E2E scripts to `@playwright/test`.
9. Only then consider optional Redis/read-replica/queue support.

## Completion Gates For Rebuild PRs

Every rebuild PR must state:

- Which target crate/app boundary it advances.
- Which existing endpoint or workflow is behaviorally unchanged.
- Which harness stages were run.
- Whether `docs/design-docs/backend-api-catalog.md` and generated OpenAPI remain aligned.

Recommended minimum gates:

- `bash scripts/harness.sh docs-check`
- `bash scripts/harness.sh fmt-check`
- focused Rust tests for the moved workflow
- `bash scripts/harness.sh lint` before merge
- `bash scripts/harness.sh frontend-login` or relevant Playwright spec for browser-visible changes
