---
name: Configure E2E Environment
description: Automates the configuration of the local Docker/Podman environment for stable E2E testing (High Rate Limits, etc.).
---

# Configure E2E Environment

This skill optimizes the `docker-compose.yml` (or equivalent) configuration to ensure E2E tests run smoothly without hitting API rate limits or timeout issues.

## Problem
E2E tests often generate a high volume of requests in a short time, triggering the backend's rate limiter (`429 Too Many Requests`). This causes flaky tests.

## Solution
This skill provides a script to:
1.  Read `docker-compose.yml`.
2.  Locate the `backend` service definition.
3.  Inject or Update environment variables for rate limiting to high values (e.g., 10000).
    -   `RATE_LIMIT_IP_MAX_REQUESTS`
    -   `RATE_LIMIT_USER_MAX_REQUESTS`
    -   `RATE_LIMIT_IP_WINDOW_SECONDS`
    -   `RATE_LIMIT_USER_WINDOW_SECONDS`

## Usage

### 1. Optimize Configuration

Run the optimization script:

```bash
node .agent/skills/configure_e2e_env/scripts/optimize_env.mjs
```

This will modify `docker-compose.yml` in place.

### 2. Apply Changes

After running the script, restart your containers to apply the changes:

```bash
# If using Docker Compose
docker-compose up -d backend

# If using Podman Compose (WSL)
podman-compose up -d backend
```

## Reverting
Currently, this skill is "optimize-only". To revert, you can manually edit `docker-compose.yml` or discard changes via git if committed.
