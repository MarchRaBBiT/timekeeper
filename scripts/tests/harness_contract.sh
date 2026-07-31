#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
harness="$repo_root/scripts/harness.sh"
agents="$repo_root/AGENTS.md"
harness_manual="$repo_root/docs/manual/HARNESS.md"
plans="$repo_root/.agent/PLANS.md"

fail() {
  printf 'harness contract failed: %s\n' "$*" >&2
  exit 1
}

for path in "$harness" "$agents" "$harness_manual" "$plans"; do
  [[ -f "$path" ]] || fail "missing ${path#"$repo_root/"}"
done

executable_stages=()
while IFS= read -r stage; do
  executable_stages+=("$stage")
done < <(bash "$harness" --list)
[[ ${#executable_stages[@]} -gt 0 ]] || fail "harness exposes no stages"

for stage in "${executable_stages[@]}"; do
  bash "$harness" --check-stage "$stage" >/dev/null \
    || fail "harness lists a stage that is not dispatchable: $stage"
done

manual_stages=()
while IFS= read -r stage; do
  manual_stages+=("$stage")
done < <(sed -n 's/^### `\([^`]*\)`$/\1/p' "$harness_manual")

ladder_stages=()
while IFS= read -r stage; do
  ladder_stages+=("$stage")
done < <(sed -n 's/^[0-9][0-9]*\. `\([^`]*\)`$/\1/p' "$agents")

compare_stage_sets() {
  local source_name="$1"
  shift
  local -a documented_stages=("$@")

  if [[ ${#documented_stages[@]} -ne ${#executable_stages[@]} ]]; then
    fail "$source_name documents ${#documented_stages[@]} stages; harness exposes ${#executable_stages[@]}"
  fi

  local executable
  local documented
  executable="$(printf '%s\n' "${executable_stages[@]}" | sort)"
  documented="$(printf '%s\n' "${documented_stages[@]}" | sort)"
  [[ "$documented" == "$executable" ]] \
    || fail "$source_name stage set does not match scripts/harness.sh --list"
}

compare_stage_order() {
  local source_name="$1"
  shift
  local -a documented_stages=("$@")

  compare_stage_sets "$source_name" "${documented_stages[@]}"

  local index
  for index in "${!executable_stages[@]}"; do
    if [[ "${documented_stages[$index]}" != "${executable_stages[$index]}" ]]; then
      fail "$source_name stage $((index + 1)) is ${documented_stages[$index]}; expected ${executable_stages[$index]}"
    fi
  done
}

compare_stage_sets "docs/manual/HARNESS.md" "${manual_stages[@]}"
compare_stage_order "AGENTS.md Validation Ladder" "${ladder_stages[@]}"

command -v python3 >/dev/null 2>&1 \
  || fail "python3 >= 3.8 is required to validate Markdown links"
python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 8) else 1)' \
  || fail "python3 >= 3.8 is required to validate Markdown links"

python3 - "$repo_root" "$agents" "$plans" <<'PY'
import re
import sys
from pathlib import Path
from urllib.parse import unquote

repo_root = Path(sys.argv[1]).resolve()
documents = [Path(value).resolve() for value in sys.argv[2:]]
link_pattern = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
failures = []

for document in documents:
    text = document.read_text(encoding="utf-8")
    for raw_target in link_pattern.findall(text):
        target = raw_target.strip()
        if target.startswith("<"):
            closing_bracket = target.find(">")
            if closing_bracket == -1:
                failures.append(
                    f"{document.relative_to(repo_root)}: malformed link target: {target}"
                )
                continue
            target = target[1:closing_bracket]
        else:
            target = target.split(maxsplit=1)[0]
        if (
            not target
            or target.startswith(("#", "http://", "https://", "mailto:"))
        ):
            continue

        path_text = unquote(target.split("#", maxsplit=1)[0])
        resolved = (document.parent / path_text).resolve()
        try:
            resolved.relative_to(repo_root)
        except ValueError:
            failures.append(
                f"{document.relative_to(repo_root)}: link escapes repository: {target}"
            )
            continue

        if not resolved.exists():
            failures.append(
                f"{document.relative_to(repo_root)}: missing link target: {target}"
            )

if failures:
    print("harness contract failed: broken local Markdown links", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    raise SystemExit(1)
PY

printf 'harness contract: pass\n'
