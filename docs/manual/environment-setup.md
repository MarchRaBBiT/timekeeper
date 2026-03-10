# Timekeeper Environment Setup Guide

This document summarizes the procedures from `SETUP_GUIDE.md`, `README.md`, and `scripts/` to quickly set up a local Timekeeper environment. The same flow applies to macOS / Linux / Windows (PowerShell).

## 1. Requirements

| Purpose | Tool |
| --- | --- |
| Backend | [Rust 1.70+](https://www.rust-lang.org/tools/install) (with `cargo`) |
| Frontend (WASM) | `wasm-pack` (`cargo install wasm-pack`) and Python 3 (for static server) |
| Database | SQLite (bundled) for local execution, PostgreSQL also available via `DATABASE_URL` |
| Automation (optional) | Podman Desktop + Podman Compose |
| Testing | Firefox (`wasm-pack test --headless --firefox`), Node.js 18+ (Playwright/E2E) |

> Note: On Windows, use `scripts/backend.ps1` / `scripts/frontend.ps1` to batch control start/stop/status/logs.

## 2. Clone the Repository

```bash
git clone <repository-url>
cd timekeeper
```

## 3. Environment Variables

1. Copy the template
   ```bash
   cp env.example .env
   ```
2. Edit `.env` and configure the database and secrets
   ```env
   DATABASE_URL=postgres://timekeeper:timekeeper@localhost:5432/timekeeper
   JWT_SECRET=change-me-for-local
   JWT_EXPIRATION_HOURS=1
   REFRESH_TOKEN_EXPIRATION_DAYS=7
   AUDIT_LOG_RETENTION_DAYS=365
   AUDIT_LOG_RETENTION_FOREVER=false
   ```
3. For local PostgreSQL / staging, replace `DATABASE_URL` with the DSN and update `JWT_SECRET` to a sufficiently random value (Podman Compose also reads `.env` directly).

> For SQLite, rewrite as `DATABASE_URL=sqlite:./timekeeper.db`. The default and `env.example` assume PostgreSQL.
> Audit log retention: `AUDIT_LOG_RETENTION_DAYS=0` disables recording, `AUDIT_LOG_RETENTION_FOREVER=true` disables deletion. When both are specified, FOREVER takes precedence.

## 4. Backend Setup

```bash
cd backend
cargo fetch          # Optional: fetch dependencies only
cargo sqlx prepare   # If using offline mode
cargo run            # Apply migrations + start Axum API (port 3000)
```

Useful commands:

- `pwsh -File ..\scripts\backend.ps1 start|stop|status|logs`
- `cargo test`, `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`

On first startup, success is indicated by `Server listening on 0.0.0.0:3000`.

## 5. Frontend Setup

```bash
cd frontend
wasm-pack build --target web --out-dir pkg --dev
python -m http.server 8000
```

Access `http://localhost:8000` to view the UI. The SPA expects `http://localhost:3000/api` as the backend.

Helper tools:

- `pwsh -File ..\scripts\frontend.ps1 start` (runs build + static server together)
- `wasm-pack test --headless --firefox` (frontend unit tests)

## 6. Default Account

After running migrations, the following system administrator account is automatically created:

```
username: admin
password: admin123
```

Log in with this user and add employees/admins via the UI or `POST /api/admin/users`.

## 7. Podman (Optional)

The repository includes the following container definitions:

- `backend/Dockerfile` (Rust build → Debian slim)
- `frontend/Dockerfile` (Rust build + nginx)
- Compose definitions (`docker-compose.yml` / `.example`)

Quick start:

```bash
podman compose up --build
```

Override secrets via `.env` or the Compose `environment` section. Mount volumes if you want to persist SQLite/PostgreSQL.

## 8. E2E Smoke Test

1. Start backend and frontend (e.g., `scripts/backend.ps1 start`, `scripts/frontend.ps1 start`).
2. Run `npm install` (first time only) in the `e2e/` directory, then start `node run.mjs`. Override `FRONTEND_BASE_URL` via environment variable if not `http://localhost:8080`.

The Playwright script logs in as admin, navigates through main pages, and logs out as a smoke scenario.

---

For more detailed OS-specific procedures, see `SETUP_GUIDE.md`. For API request/response details, see `API_DOCS.md`.
