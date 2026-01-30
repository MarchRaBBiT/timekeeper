---
name: Visual Review Helper
description: Automates the process of capturing and reviewing E2E screenshots for UI/UX validation.
---

# Visual Review Helper

This skill automates the capture of UI screenshots across different viewports (desktop, mobile) and color schemes (light, dark), and helps generate a review report.

## Prerequisites

1.  **Backend & Frontend Running**: The application must be accessible (default: `http://localhost:8080`).
2.  **Test Users Configured**:
    -   Admin user credentials in `.env` or environment variables (`E2E_ADMIN_USER`, `E2E_ADMIN_PASS`).
    -   (Optional) Non-admin user for unauthorized page checks.
3.  **Docker Environment**: Ensure API rate limits are relaxed if running against a local Docker backend (set `RATE_LIMIT_*` high).

## Resources

-   `resources/screenshots.mjs`: A reference Playwright script to capture screenshots.
    -   *Note*: The active script is usually located at `e2e/screenshots.mjs` in the project root.

## Usage

### 1. Capture Screenshots

Run the screenshot capture script. This will generate images in `e2e/screenshots/`.

```bash
cd e2e
node screenshots.mjs
```

### 2. Generate Review Report

Use the helper script to generate a Markdown report containing all captured screenshots, organized by page and variant.

```bash
node .agent/skills/visual_review/scripts/generate_report.mjs
```

This will create (or overwrite) `VISUAL_REVIEW_REPORT.md` in the project root.

### 3. Analyze

Open `VISUAL_REVIEW_REPORT.md` in a Markdown viewer (or VS Code Preview).
-   Check for layout consistency between Light and Dark modes.
-   Verify mobile responsiveness.
-   Look for visual regressions or alignment issues.

## Customization

To add new pages to the capture list, edit the `pages` array in `e2e/screenshots.mjs`.
