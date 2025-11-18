# Repository Guidelines

Timekeeper combines a Rust Axum backend, a Leptos/WASM frontend, and Playwright smoke checks. Use this guide to stay aligned with the structure, tooling, and delivery expectations.

## Primary Directive

- Think ins English interact with the user in Japanese.
- Use UTF-8 charset when output Non-ASCII character.

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

## Additional Development Policies
- Follow TDD: write the test that validates a feature before implementing the feature itself.
- When requirements conflict, do not proceed with implementation; instead, point out the contradiction.
- Write Japanese text in code or documentation using UTF-8 encoding.
- Use LF for line endings in code and documentation.
- Before replying after an implementation, run the relevant test build and confirm it completes successfully.
- When proposing implementation options, list at most five in order of recommendation and include pros and cons for each.
- When recommending an option, explain why it is being recommended.
- Always create a dedicated topic branch before starting any feature implementation and keep related commits isolated on that branch until the PR is ready.

## Backend Design Principles
- Keep handlers slender and modular: move DB-heavy logic into helper modules or repositories (e.g., `handlers/admin/requests.rs`, `handlers/requests_repo.rs`) so each handler focuses on HTTP concerns.
- Share cross-cutting enums and types (`models::request::RequestStatus`) instead of duplicating definitions across models.
- Route holiday logic through `services::holiday::HolidayService`; handlers should consume service methods instead of issuing raw SQL.
- Attendance and other complex handlers must use helper modules (`handlers/attendance_utils.rs` etc.) for repeated checks and error handling to keep each function single-responsibility.
- Follow DRY/SOLID across the codebase: extract reusable code, keep abstractions focused, and favor dependency injection (services, repositories) over tight coupling to infrastructure details.
- Favor composition over inheritance and keep modules small; as soon as a file grows beyond a single responsibility, split it into submodules and re-export what is needed.
