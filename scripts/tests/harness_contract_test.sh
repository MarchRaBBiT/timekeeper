#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

fail() {
  printf 'harness contract test failed: %s\n' "$*" >&2
  exit 1
}

reset_fixture() {
  rm -rf "$fixture_root/repo"
  mkdir -p \
    "$fixture_root/repo/scripts/tests" \
    "$fixture_root/repo/.agent" \
    "$fixture_root/repo/backend/tests" \
    "$fixture_root/repo/frontend"
  cp "$repo_root/scripts/harness.sh" "$fixture_root/repo/scripts/harness.sh"
  cp "$repo_root/scripts/tests/harness_contract.sh" \
    "$fixture_root/repo/scripts/tests/harness_contract.sh"
  cp "$repo_root/AGENTS.md" "$fixture_root/repo/AGENTS.md"
  cp -R "$repo_root/docs" "$fixture_root/repo/docs"
  cp "$repo_root/backend/AGENTS.md" "$fixture_root/repo/backend/AGENTS.md"
  cp "$repo_root/backend/tests/AGENTS.md" "$fixture_root/repo/backend/tests/AGENTS.md"
  cp "$repo_root/frontend/AGENTS.md" "$fixture_root/repo/frontend/AGENTS.md"
  cp "$repo_root/.agent/PLANS.md" "$fixture_root/repo/.agent/PLANS.md"
}

expect_failure() {
  local expected="$1"
  local output

  if output="$(bash "$fixture_root/repo/scripts/tests/harness_contract.sh" 2>&1)"; then
    fail "expected failure containing: $expected"
  fi
  grep -Fq "$expected" <<<"$output" \
    || fail "failure did not contain '$expected': $output"
}

reset_fixture
bash "$fixture_root/repo/scripts/tests/harness_contract.sh" >/dev/null \
  || fail "valid fixture must pass"

reset_fixture
sed -i \
  's/harness-contract run_harness_contract/harness-contract missing_function/' \
  "$fixture_root/repo/scripts/harness.sh"
expect_failure "harness lists a stage that is not dispatchable: harness-contract"

reset_fixture
sed -i '/^### `harness-contract`$/d' "$fixture_root/repo/docs/manual/HARNESS.md"
expect_failure "docs/manual/HARNESS.md documents 14 stages; harness exposes 15"

reset_fixture
sed -i '/^[0-9][0-9]*\. `harness-contract`$/d' "$fixture_root/repo/AGENTS.md"
expect_failure "AGENTS.md Validation Ladder documents 14 stages; harness exposes 15"

reset_fixture
sed -i \
  '0,/EP-20260730-lan-manual-testbed.md/{s//missing.md/g;}' \
  "$fixture_root/repo/.agent/PLANS.md"
expect_failure ".agent/PLANS.md: missing link target: ../docs/exec-plans/active/missing.md"

reset_fixture
mkdir -p "$fixture_root/repo/docs/with spaces"
touch "$fixture_root/repo/docs/with spaces/reference.md"
printf '\n[spaced path](<../docs/with spaces/reference.md>)\n' \
  >>"$fixture_root/repo/.agent/PLANS.md"
bash "$fixture_root/repo/scripts/tests/harness_contract.sh" >/dev/null \
  || fail "angle-bracket Markdown link containing spaces must pass"

printf 'harness contract tests: pass\n'
