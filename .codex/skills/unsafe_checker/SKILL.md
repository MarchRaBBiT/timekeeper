---
name: Unsafe Checker
description: Validates `unsafe` code usage and ensures proper SAFETY comments.
---

# Unsafe Code Checker

## Rules for Unsafe Code

1. **Avoid if possible**: Always prefer safe abstractions.
2. **SAFETY Comments**: Every `unsafe` block MUST be preceded by a `// SAFETY:` comment explaining why it is safe.

```rust
// Good
// SAFETY: We checked that index < len above, so this is in bounds.
unsafe { slice.get_unchecked(index) }

// Bad
unsafe { slice.get_unchecked(index) }
```

## Verification Steps
When reviewing or writing `unsafe` code:
1. **Preconditions**: Identify what must be true for the code to be safe (e.g., valid pointers, bounds checks).
2. **Invariants**: Ensure the operation maintains the type's memory safety invariants.
3. **Comment**: Document the specific check that justifies the `unsafe` block.

## Checklist
- [ ] Is `unsafe` strictly necessary?
- [ ] Is there a `// SAFETY:` comment?
- [ ] Does the comment explain *why* it is safe, not just *what* it does?
- [ ] Are raw pointers checked for null/alignment?
