# Repository Guidelines

Timekeeper combines a Rust Axum backend, a Leptos/WASM frontend, and Playwright smoke checks. Use this guide to stay aligned with the structure, tooling, and delivery expectations.

## Project Structure & Module Organization
- `backend/` runs the API; `src/handlers`, `models`, `middleware`, and `db` mirror the service layers, while `migrations/*.sql` seed SQLx with schema changes.
- `frontend/` is a Rust crate compiled to WASM; `src/components` covers reusable widgets, `src/pages` hosts routed screens, `src/api` centralizes HTTP calls, and `config.json` pairs with `src/config.rs` for runtime settings.
- `e2e/` holds Playwright journeys (`run.mjs`, `guard.mjs`, `logout.mjs`) that assume the frontend at `FRONTEND_BASE_URL`.
- `scripts/` bundles PowerShell helpers for dockerized backend control, frontend builds, and API smoke automation; keep new automation here.

## Build, Test, and Development Commands
```powershell
# backend (native)
cd backend; cargo run
# backend (docker compose via helper)
pwsh -File .\scripts\backend.ps1 start
# frontend (rebuild + static server on :8000)
pwsh -File .\scripts\frontend.ps1 start
```
Prefer the scripts’ `stop/status/logs` subcommands to avoid stale PID files.

## Coding Style & Naming Conventions
- Rust code uses `cargo fmt --all` (4-space indent) and `cargo clippy --all-targets -- -D warnings` before review; keep modules snake_case, types and components in PascalCase.
- Frontend signals reactive state with `*_signal` or `use_*` helpers; keep Leptos components in `PascalCase`.
- Keep configuration values in `.env`; never commit generated artifacts (`frontend/pkg/`, `.backend.pid`).

## Testing Guidelines
- Run `cd backend; cargo test` plus `cargo clippy` for backend changes; `pwsh -File .\scripts\test_backend.ps1` exercises key API flows against a running server.
- Frontend modules require `wasm-pack test --headless --firefox` from `frontend/` once you install Playwright’s browsers.
- UI regressions: `cd e2e; node run.mjs` (set `FRONTEND_BASE_URL` when not on `http://localhost:8080`).

## Commit & Pull Request Guidelines
- This archive omits Git history; align with Conventional Commits (`feat:`, `fix:`, `chore:`) so changelog tooling stays predictable and scope stays clear.
- Each PR needs: concise summary, linked issue/Linear ticket, backend/frontend impact notes, env variable diffs, and test evidence (`cargo test`, `wasm-pack test`, Playwright smoke output).
- Include screenshots or terminal transcripts for UI-facing changes and note follow-up migration steps for reviewers.

## Environment & Configuration Tips
- Copy `env.example` to `.env`, set `DATABASE_URL` (SQLite locally or your managed Postgres DSN), and keep `JWT_SECRET` unique because docker compose loads it straight from `.env`.
