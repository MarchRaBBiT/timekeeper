# Continuity Ledger

- Goal: Fix env.js loading error causing HTML to be executed as JS.
- Constraints/Assumptions: Follow project coding standards, LF endings, UTF-8 for non-ASCII, stay within writable roots.
- Key decisions: Existing data compatibility concerns are not an issue because service not live yet.
- State: FIXING_ENV_JS
- Done:
    - Verified current frontend assets; browser cache likely cause of prior WASM error.
    - CSP updated for Google Fonts.
- Now:
    - Added default `frontend/env.js` and Dockerfile copy step so env.js is real JS, not index.html fallback.
- Next:
    - Rebuild frontend image or static assets; confirm env.js loads without syntax error.
- Open questions:
    - None.
- Working set:
    - `frontend` build artifacts/load path
