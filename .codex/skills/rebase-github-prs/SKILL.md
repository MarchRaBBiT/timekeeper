---
name: rebase-github-prs
description: "Use this skill when the user asks to rebase one or more GitHub PRs identified by PR number. It is designed for CLI-driven batch processing: resolve PR metadata with gh, fetch branches with jj, rebase onto the latest base branch, and optionally push the rewritten branch back to the PR remote."
---

# Rebase GitHub PRs

Use this skill when the user wants a GitHub PR, or a batch of PR numbers, rebased from the current local repository.

## Trigger Patterns

Use this skill for requests such as:

- "PR #123 を rebase してください"
- "指定した PR 番号をまとめて rebase してください"
- "GitHub PR の batch rebase agent を使ってください"
- "複数 PR を CLI から順に rebase したい"

Do not use this skill for:

- Generic branch management with no GitHub PR number
- Cherry-pick only workflows
- Pure review with no rewrite request

## Preconditions

- `gh` is installed and authenticated for the target repository
- `jj` is installed and the current repo is a `jj` repo backed by Git
- The working copy should be clean before batch processing
- The user should be explicit if the rewritten PR branches should also be pushed

## Workflow

1. Read repo instructions first.
- Check root and relevant `AGENTS.md` files.
- Respect repo-specific validation, snapshot, and push rules.

2. Resolve PR metadata with `gh pr view <number>`.
- Capture `headRefName`, `baseRefName`, `isCrossRepository`, `headRepository.url`, `state`, and `url`.
- Skip or stop on closed PRs unless the user explicitly asks otherwise.

3. Fetch the base and head branches with `jj`.
- Same-repo PRs: fetch both branches from `origin`.
- Fork PRs: add a temporary remote pointing at the head repository URL, then fetch the head branch from that remote.

4. Materialize a local bookmark alias for the PR.
- Use a stable local alias such as `pr-<number>`.
- Point it at `<headRefName>@<remote>` with `jj bookmark set -B`.

5. Rebase the PR branch bookmark onto the latest base branch.
- Use `jj rebase -b pr-<number> -d <baseRefName>@origin`.
- If conflicts occur, do not stop at reporting only. Treat the helper's non-zero exit as the handoff into the agent resolution phase. Read [references/conflict-resolution.md](references/conflict-resolution.md), continue from the existing `pr-<number>` conflict state, and finish the resolution before deciding whether to push.

6. Push only when the user asked for the PR to be updated remotely.
- Move the fetched local head bookmark to the rebased alias with `jj bookmark set -B <headRefName> -r pr-<number>`.
- Then push it with `jj git push --remote <head-remote> -b <headRefName>`.
- Keep `pr-<number>` as the stable local alias for follow-up inspection and retries.

7. Summarize each PR.
- PR number
- Source remote
- Base branch
- Whether rebase succeeded
- Whether push was performed

## Conflict Resolution Workflow

When `jj rebase` produces conflicts, the agent should resolve them proactively instead of bouncing the problem back immediately.

The helper script's failure is not the end of the task. It is the boundary between deterministic setup and semantic merge work. After the helper stops on a conflict, the agent should continue automatically with the following flow instead of asking the user to take over.

1. Enter the resolution branch state.
- Treat `pr-<number>` as the source of truth for the conflicted rebase result.
- Create a working commit on top of it with `jj new pr-<number>` if the working copy is not already there.
- Do not restart the whole helper unless you intentionally want to throw away the current merge attempt.

2. Reconstruct intent before editing.
- Read the PR title, body, and labels with `gh pr view <number>`.
- Read the PR patch with `gh pr diff <number>` for the original change intent.
- Check whether there are review comments or linked issue clues that narrow the intended behavior.

3. Inspect the conflicting seam from both sides.
- Use `jj resolve --list` to enumerate conflicted files.
- Use `jj diff -r pr-<number>` and `jj diff -r <baseRefName>@origin` around the conflicted files.
- Read the actual conflicted file contents, not only the markers.
- Search the current base branch for the owning abstraction before editing. If the responsibility moved layers or modules, find the new entry point with `rg` and resolve the intent there instead of mechanically restoring the old location.

4. Apply intent-based merge rules.
- Keep the PR side when the base branch only reformats, renames symbols, or moves code without changing behavior, and the PR still represents the requested feature or fix.
- Keep the base side when the PR's old implementation is clearly superseded by a newer canonical abstraction and the original behavior can be re-expressed on top of it.
- Merge both when the PR introduces behavior that is still needed, but the base branch changed surrounding APIs, types, or structure.
- If the base branch relocated the business rule, port the PR behavior into that new owner and keep the conflict file aligned with the base branch's delegation structure.
- Re-check invariants after editing: API shape, validation rules, authorization behavior, tests, and user-visible copy must still match the PR's purpose unless the repo changed the requirement.

5. Validate the resolved intent.
- Run the smallest focused tests for the conflicted seam first.
- Prefer tests at the layer that now owns the behavior. If a handler now delegates to an application/service module, validate the new owner instead of only the old entry file.
- If the conflict touched routing/composition, run a smoke test too.
- Only escalate to the user when multiple plausible semantic resolutions remain and local context cannot disambiguate them safely.

6. Fold the resolution back into the rebased PR.
- Run `jj squash` so the fix becomes part of the conflicted rebased commit instead of a stray follow-up commit.
- Re-check `jj bookmark list pr-<number>` and ensure `(conflict)` is gone.

7. Then resume the batch.
- If `--push` was requested, move `<headRefName>` to `pr-<number>` and push it.
- Otherwise leave `pr-<number>` in place and summarize what remains for the user.

## Batch Helper

Use the bundled helper when processing multiple PR numbers:

```bash
bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh 123 124 130
```

Optional push:

```bash
bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --push 123 124 130
```

Automatic conflict handoff into Codex:

```bash
bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --push --agent-resolve 123 124 130
```

Dry-run planning:

```bash
bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh --dry-run 123 124 130
```

## Safety Notes

- The helper script refuses to run on a dirty working copy unless `--allow-dirty` is given.
- `--push` rewrites remote branch history. Treat it as an explicit user-facing action.
- `--agent-resolve` invokes `codex exec` locally to continue semantic conflict resolution from the existing `pr-<number>` conflict state.
- Temporary remotes are removed after successful processing unless `--keep-remote` is specified.
- The helper script detects when `jj rebase` leaves `pr-<number>` in a conflicted state and exits non-zero with follow-up inspection commands.
- The helper script does not resolve semantic conflicts by itself. Automatic semantic resolution requires `--agent-resolve`, which hands the task to Codex.

## Minimal Checklist

- Relevant `AGENTS.md` read
- PR metadata resolved from `gh`
- Base/head branches fetched with `jj`
- Local alias bookmark created
- Conflicts, if any, were resolved according to PR intent and validated
- Push only if explicitly requested
