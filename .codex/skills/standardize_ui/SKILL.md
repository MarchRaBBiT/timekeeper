---
name: Standardize UI Components
description: Identifies legacy UI patterns and provides instructions/scripts to migrate them to the new design system.
---

# Standardize UI Components

This skill helps maintain visual consistency by identifying "legacy" utility classes and patterns that should be replaced with modern Design System components or updated utility sets.

## Utility Patterns to Replace

Legacy patterns often found in this project include:
-   **Old Inputs**: `border border-form-control-border bg-form-control-bg rounded px-2 py-1`
    -   *Replace with*: `rounded-xl border-2 border-form-control-border bg-form-control-bg py-2.5 px-4 shadow-sm ...` (or use `DatePicker` / `Input` components)
-   **Old Buttons**: `px-3 py-1 rounded border ...`
    -   *Replace with*: `Button` component or taller, rounded-xl classes.

## Usage

### 1. Scan for Legacy Usages

Run the provided script to find files containing suspect legacy classes.

```bash
node .agent/skills/standardize_ui/scripts/scan_legacy.mjs
```

This will output a list of files and line numbers where legacy patterns are detected.

### 2. Manual Migration

For each detected instance:
1.  Open the file.
2.  Assess if it's a form input, button, or container.
3.  Replace the class strings with the new Design System equivalents (refer to `frontend/src/components/forms.rs` or `common.rs`).

## Scripts

-   `scripts/scan_legacy.mjs`: Scans specifically for `px-2 py-1`, `rounded px-2`, and other old combinations.
