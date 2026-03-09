# Conflict Resolution

Use this reference when `rebase-github-prs` hits a conflict and you need to decide what the merged result should be.

## Primary Principle

Resolve conflicts by preserving the PR's intended behavior, not by mechanically preferring "ours" or "theirs".

## Read First

For the target PR, gather:

- `gh pr view <number> --json title,body,labels,author,baseRefName,headRefName`
- `gh pr diff <number>`
- any linked issue or plan mentioned in the PR body

Then gather local conflict context:

- `jj resolve --list`
- `jj diff -r pr-<number> -- <file>`
- `jj diff -r <base>@origin -- <file>`
- `rg -n "<function|type|route name>" <repo paths>` to find whether the same responsibility moved to another module or layer on base

If the helper already produced `pr-<number>` with conflicts, continue from that state instead of starting over:

- `jj bookmark list pr-<number>`
- `jj new pr-<number>` if you need a working commit above the conflicted rebased commit
- edit/resolve in place
- `jj squash` after validation to fold the resolution back into the rebased PR commit

## Heuristics

### Prefer PR intent

Use the PR side as the source of truth when:

- the base branch only renamed helpers, moved modules, or reformatted logic
- the PR fixes a bug that still exists after rebase
- the PR adds validation, security checks, or user-visible behavior that the base branch does not replace

### Prefer base abstractions

Use the base branch structure when:

- the same responsibility was moved into a new module or abstraction on main
- the PR's old code path no longer matches current types or architecture
- keeping the old structure would reintroduce deleted duplication

In these cases, port the PR behavior onto the new abstraction instead of resurrecting the old layout.

### Re-home the behavior when ownership moved

When the conflict happens in an old entry file, do not assume that file should still contain the logic.

- Find the current owner of the behavior on base: handler vs application service vs repository vs model validation
- Keep thin wrappers thin if base moved business logic out of them
- Apply the PR's rule at the layer now enforcing adjacent rules
- Preserve base delegation paths and only reintroduce code into the old file if the base branch still expects that file to own the behavior

### Merge both

Do a manual merge when:

- both sides made behaviorally meaningful edits
- the PR changed validation or branching logic and the base branch changed data flow or types
- both sides touched tests and production code around the same seam

## Red Flags

Escalate only when local evidence cannot disambiguate the intended result:

- the PR body and code disagree about the intended behavior
- review comments request one behavior, but later commits on base appear to reject it
- the conflict changes business rules and there is no local spec, test, or issue context to settle it

## After Resolving

- ensure conflict markers are fully removed
- run focused tests for the touched seam
- also run focused tests for the module that now owns the behavior if ownership changed during the merge
- fold the resolution back with `jj squash` so `pr-<number>` becomes the resolved rebased result
- if the caller requested push, move the head bookmark to `pr-<number>` and push that bookmark
- check whether any snapshots or fixtures also need updates
- summarize the chosen resolution in terms of intent, not only file mechanics
