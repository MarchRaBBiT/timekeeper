#!/usr/bin/env bash
set -euo pipefail

BASE_REMOTE="origin"
DO_PUSH=0
KEEP_REMOTE=0
ALLOW_DIRTY=0
DRY_RUN=0

declare -a PRS=()

usage() {
  cat <<'EOF'
Usage: bash .codex/skills/rebase-github-prs/scripts/rebase_prs.sh [options] <pr-number>...

Options:
  --base-remote NAME  Base branch remote to rebase onto (default: origin)
  --push              Push the rebased commit back to the PR head remote
  --dry-run           Print the rebase/push commands without mutating history
  --keep-remote       Keep temporary remotes for fork PRs
  --allow-dirty       Skip the clean working copy check
  --help              Show this help
EOF
}

log() { printf '[rebase-prs] %s\n' "$1"; }
fail() { printf '[rebase-prs][FAIL] %s\n' "$1" >&2; exit 1; }

run_cmd() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run] '
    printf '%q ' "$@"
    printf '\n'
    return 0
  fi
  "$@"
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

bookmark_has_conflict() {
  local bookmark_name="$1"
  local bookmark_output

  bookmark_output="$(jj bookmark list "$bookmark_name" 2>/dev/null || true)"
  [[ -n "$bookmark_output" ]] && grep -q '(conflict)' <<<"$bookmark_output"
}

report_conflict_resolution_hint() {
  local pr="$1"
  local bookmark_name="$2"
  local head_ref="$3"
  local remote_name="$4"

  cat >&2 <<EOF
[rebase-prs][FAIL] Rebase for PR #$pr produced conflicts on bookmark '$bookmark_name'.
[rebase-prs][FAIL] Continue with the agent resolution phase from the existing conflict state:
[rebase-prs][FAIL]   jj new $bookmark_name
[rebase-prs][FAIL]   gh pr view $pr
[rebase-prs][FAIL]   gh pr diff $pr
[rebase-prs][FAIL]   jj bookmark list $bookmark_name
[rebase-prs][FAIL]   jj resolve --list
[rebase-prs][FAIL]   jj diff -r $bookmark_name
[rebase-prs][FAIL]   <edit and validate>
[rebase-prs][FAIL]   jj squash
[rebase-prs][FAIL]   jj bookmark set -B $head_ref -r $bookmark_name
[rebase-prs][FAIL]   jj git push --remote $remote_name -b $head_ref
EOF
  exit 1
}

require_cmds() {
  have_cmd gh || fail "gh is required"
  have_cmd jj || fail "jj is required"
}

ensure_clean_working_copy() {
  if [[ "$ALLOW_DIRTY" -eq 1 ]]; then
    return 0
  fi

  local status_output
  status_output="$(jj status)"
  if ! grep -q "The working copy has no changes." <<<"$status_output"; then
    printf '%s\n' "$status_output" >&2
    fail "working copy is dirty; rerun with --allow-dirty only if you understand the risk"
  fi
}

gh_field() {
  local pr="$1"
  local field="$2"
  local jq_expr="$3"
  gh pr view "$pr" --json "$field" --jq "$jq_expr"
}

remote_url() {
  local remote_name="$1"
  jj git remote list | awk -v name="$remote_name" '$1 == name { print $2 }'
}

ensure_remote() {
  local remote_name="$1"
  local url="$2"

  if jj git remote list | awk '{print $1}' | grep -qx "$remote_name"; then
    run_cmd jj git remote set-url "$remote_name" "$url"
  else
    run_cmd jj git remote add "$remote_name" "$url"
  fi
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base-remote)
        BASE_REMOTE="$2"
        shift 2
        ;;
      --push)
        DO_PUSH=1
        shift
        ;;
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      --keep-remote)
        KEEP_REMOTE=1
        shift
        ;;
      --allow-dirty)
        ALLOW_DIRTY=1
        shift
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        PRS+=("$1")
        shift
        ;;
    esac
  done

  [[ "${#PRS[@]}" -gt 0 ]] || {
    usage
    exit 1
  }

  require_cmds
  ensure_clean_working_copy

  local base_remote_url
  base_remote_url="$(remote_url "$BASE_REMOTE")"
  [[ -n "$base_remote_url" ]] || fail "remote '$BASE_REMOTE' is not configured"

  local origin_url
  origin_url="$(remote_url origin)"

  local pr
  for pr in "${PRS[@]}"; do
    [[ "$pr" =~ ^[0-9]+$ ]] || fail "PR number must be numeric: $pr"

    log "Resolving PR #$pr"

    local state
    local title
    local pr_url
    local head_ref
    local base_ref
    local is_cross
    local head_repo_url
    local remote_name
    local bookmark_name
    local cleanup_remote=0

    state="$(gh_field "$pr" state '.state')"
    title="$(gh_field "$pr" title '.title')"
    pr_url="$(gh_field "$pr" url '.url')"
    head_ref="$(gh_field "$pr" headRefName '.headRefName')"
    base_ref="$(gh_field "$pr" baseRefName '.baseRefName')"
    is_cross="$(gh_field "$pr" isCrossRepository '.isCrossRepository')"
    head_repo_url="$(gh_field "$pr" headRepository '.headRepository.url')"

    [[ "$state" == "OPEN" ]] || fail "PR #$pr is not open (state=$state)"
    [[ -n "$head_ref" ]] || fail "Failed to resolve headRefName for PR #$pr"
    [[ -n "$base_ref" ]] || fail "Failed to resolve baseRefName for PR #$pr"

    remote_name="$BASE_REMOTE"
    if [[ "$is_cross" == "true" && -n "$head_repo_url" && "$head_repo_url" != "$origin_url" ]]; then
      remote_name="pr-${pr}"
      ensure_remote "$remote_name" "$head_repo_url"
      cleanup_remote=1
    fi

    bookmark_name="pr-${pr}"

    log "PR #$pr: ${title}"
    log "URL: ${pr_url}"
    log "Base: ${base_ref}@${BASE_REMOTE}"
    log "Head: ${head_ref}@${remote_name}"

    run_cmd jj git fetch --remote "$BASE_REMOTE" --branch "$base_ref"
    run_cmd jj git fetch --remote "$remote_name" --branch "$head_ref"
    run_cmd jj bookmark set -B "$bookmark_name" -r "${head_ref}@${remote_name}"

    if ! run_cmd jj rebase -b "$bookmark_name" -d "${base_ref}@${BASE_REMOTE}"; then
      fail "Rebase failed for PR #$pr. Resolve conflicts around bookmark '$bookmark_name' and rerun."
    fi

    if [[ "$DRY_RUN" -ne 1 ]] && bookmark_has_conflict "$bookmark_name"; then
      report_conflict_resolution_hint "$pr" "$bookmark_name" "$head_ref" "$remote_name"
    fi

    log "Rebased bookmark '$bookmark_name' for PR #$pr"

    if [[ "$DO_PUSH" -eq 1 ]]; then
      run_cmd jj bookmark set -B "$head_ref" -r "$bookmark_name"
      if [[ "$DRY_RUN" -eq 1 ]]; then
        run_cmd jj git push --dry-run --remote "$remote_name" -b "$head_ref"
      else
        run_cmd jj git push --remote "$remote_name" -b "$head_ref"
      fi
      log "Pushed rewritten branch '${head_ref}' to remote '${remote_name}'"
    fi

    if [[ "$cleanup_remote" -eq 1 && "$KEEP_REMOTE" -ne 1 ]]; then
      run_cmd jj git remote remove "$remote_name"
    fi
  done
}

main "$@"
