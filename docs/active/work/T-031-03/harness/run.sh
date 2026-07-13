#!/usr/bin/env bash
# Deterministic T-031-03 atomic provider-contract regression.
set -euo pipefail

KEEP=0
ROOT=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --keep)
      KEEP=1
      shift
      ;;
    --root)
      [ "$#" -ge 2 ] || { echo "--root requires a path" >&2; exit 2; }
      ROOT="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [ -z "$ROOT" ]; then
  ROOT="$(mktemp -d "${TMPDIR:-/tmp}/lisa-t03103.XXXXXX")"
else
  mkdir -p "$ROOT"
  ROOT="$(cd "$ROOT" && pwd)"
fi

REPO="$ROOT/repo"
EVIDENCE="$ROOT/evidence"
mkdir -p "$REPO" "$EVIDENCE/trees" "$EVIDENCE/hashes"

cleanup() {
  status=$?
  if [ "$status" -ne 0 ] || [ "$KEEP" -eq 1 ]; then
    echo "T-031-03 fixture retained at $ROOT" >&2
    echo "evidence: $EVIDENCE" >&2
  else
    rm -rf "$ROOT"
  fi
  exit "$status"
}
trap cleanup EXIT

if [ -n "${LISA_BIN:-}" ]; then
  LISA="$LISA_BIN"
else
  LISA="$(command -v lisa || true)"
fi
[ -n "$LISA" ] && [ -x "$LISA" ] || {
  echo "set LISA_BIN to an executable lisa binary" >&2
  exit 2
}

cd "$REPO"
git init -q
git config user.email lisa-atomic-harness@example.invalid
git config user.name "Lisa atomic harness"

printf '[package]\nname = "atomic-provider-fixture"\nversion = "0.1.0"\nedition = "2021"\n' > Cargo.toml
"$LISA" init > "$EVIDENCE/init.txt"
# The transaction lock is a reusable repository-local inode. Projects normally
# ignore it; make that contract explicit in this fixture baseline.
printf '.lisa-commit.lock\n' > .gitignore

mkdir -p docs/active/tickets src
printf 'baseline foreign content\n' > foreign.txt

write_ticket() {
  id="$1"
  title="$2"
  agent="$3"
  dependency="$4"
  path="docs/active/tickets/${id}-${title}.md"
  if [ -n "$dependency" ]; then
    depends="[$dependency]"
  else
    depends="[]"
  fi
  printf '%s\n' \
    '---' \
    "id: $id" \
    "title: $title" \
    'type: task' \
    'status: open' \
    'priority: high' \
    'phase: review' \
    "agent: $agent" \
    "depends_on: $depends" \
    '---' \
    '' \
    '## Context' \
    '' \
    'Atomic provider-contract fixture ticket.' \
    '' \
    '## Acceptance Criteria' \
    '' \
    '- Complete through the isolated Lisa transaction.' > "$path"
}

write_ticket T-CDX-01 codex-first codex ""
write_ticket T-CDX-02 codex-second codex ""
write_ticket T-CDX-03 codex-third codex ""
write_ticket T-CDX-04 codex-fourth codex ""
write_ticket T-CDX-05 codex-dependent codex T-CDX-01
write_ticket T-MIX-01 mixed-claude claude T-CDX-05

# A ready baseline sentinel lets `lisa validate` check the complete fixture even
# though transaction tickets begin at Review (phase advancement is plugin-tested).
printf '%s\n' \
  '---' \
  'id: T-SENTINEL' \
  'title: validation-sentinel' \
  'type: task' \
  'status: open' \
  'priority: low' \
  'phase: ready' \
  'depends_on: []' \
  '---' \
  '' \
  '## Acceptance Criteria' \
  '' \
  '- Keep the validation fixture eligible.' > docs/active/tickets/T-SENTINEL-validation-sentinel.md

"$LISA" validate > "$EVIDENCE/validate.txt"
git add -A
git commit -q -m "fixture baseline"

# This ordinary-index entry belongs to a foreign actor and must survive every
# implementation and completion commit byte-for-byte without entering a commit.
printf 'foreign staged change that Lisa must preserve\n' > foreign.txt
git add -- foreign.txt
git ls-files --stage -- foreign.txt > "$EVIDENCE/index.before"

: > "$EVIDENCE/activity.jsonl"
: > "$EVIDENCE/provenance.jsonl"
: > "$EVIDENCE/commits.txt"

assert_foreign_index_unchanged() {
  git ls-files --stage -- foreign.txt > "$EVIDENCE/index.current"
  cmp "$EVIDENCE/index.before" "$EVIDENCE/index.current"
}

assert_foreign_not_committed() {
  commit="$1"
  if git diff-tree --no-commit-id --name-only -r "$commit" | grep -Fxq foreign.txt; then
    echo "foreign.txt entered ticket commit $commit" >&2
    exit 1
  fi
}

run_ticket() {
  id="$1"
  title="$2"
  agent="$3"
  dependency="$4"
  ticket_path="docs/active/tickets/${id}-${title}.md"
  work_path="docs/active/work/$id"
  source_path="src/$id.txt"

  if [ -n "$dependency" ]; then
    prerequisite_file="$EVIDENCE/hashes/$dependency"
    [ -s "$prerequisite_file" ] || {
      echo "$id started before $dependency had a completion receipt" >&2
      exit 1
    }
    prerequisite="$(cat "$prerequisite_file")"
    git cat-file -e "$prerequisite^{commit}"
    git merge-base --is-ancestor "$prerequisite" HEAD
  fi

  printf '{"event":"ticket_started","ticket":"%s","agent":"%s","seat":"seat-1"}\n' \
    "$id" "$agent" >> "$EVIDENCE/activity.jsonl"

  printf 'ticket-owned implementation for %s via %s\n' "$id" "$agent" > "$source_path"
  implementation_commit="$("$LISA" commit-ticket \
    --path "$REPO" \
    --ticket-id "$id" \
    --message "Implement $id" \
    --include "$source_path")"
  git cat-file -e "$implementation_commit^{commit}"
  assert_foreign_index_unchanged
  assert_foreign_not_committed "$implementation_commit"
  printf '%s implementation %s\n' "$id" "$implementation_commit" >> "$EVIDENCE/commits.txt"
  printf '{"event":"implementation_committed","ticket":"%s","agent":"%s","seat":"seat-1","commit":"%s"}\n' \
    "$id" "$agent" "$implementation_commit" >> "$EVIDENCE/activity.jsonl"

  mkdir -p "$work_path"
  for artifact in research design structure plan progress review; do
    printf '# %s\n\nTicket %s %s evidence.\n' "$artifact" "$id" "$agent" > "$work_path/$artifact.md"
  done

  printf '{"event":"completion_pending","ticket":"%s","agent":"%s","seat":"seat-1"}\n' \
    "$id" "$agent" >> "$EVIDENCE/activity.jsonl"
  completion_commit="$("$LISA" complete-ticket \
    --path "$REPO" \
    --ticket-id "$id" \
    --attempt-id "fixture-$id" \
    --completion-generation 1 \
    --message "Complete $id" \
    --ticket-file "$ticket_path" \
    --work-dir "$work_path")"
  git cat-file -e "$completion_commit^{commit}"
  git merge-base --is-ancestor "$implementation_commit" "$completion_commit"
  assert_foreign_index_unchanged
  assert_foreign_not_committed "$completion_commit"

  git show "$completion_commit:$ticket_path" > "$EVIDENCE/$id.ticket.done"
  grep -Fxq 'phase: done' "$EVIDENCE/$id.ticket.done"
  grep -Fxq 'status: done' "$EVIDENCE/$id.ticket.done"
  if git show "$completion_commit^:$ticket_path" | grep -Fxq 'phase: done'; then
    echo "$id Done frontmatter existed before its completion commit" >&2
    exit 1
  fi

  git ls-tree -r --name-only "$completion_commit" > "$EVIDENCE/trees/$id.txt"
  grep -Fxq "$source_path" "$EVIDENCE/trees/$id.txt"
  for artifact in research design structure plan progress review; do
    git cat-file -e "$completion_commit:$work_path/$artifact.md"
  done

  if [ -n "$(git status --porcelain=v1 -- "$ticket_path" "$work_path" "$source_path")" ]; then
    echo "$id left loop-owned residue after completion" >&2
    git status --porcelain=v1 -- "$ticket_path" "$work_path" "$source_path" >&2
    exit 1
  fi

  printf '%s\n' "$completion_commit" > "$EVIDENCE/hashes/$id"
  printf '%s completion %s\n' "$id" "$completion_commit" >> "$EVIDENCE/commits.txt"
  printf '{"event":"completion_confirmed","ticket":"%s","agent":"%s","seat":"seat-1","commit":"%s"}\n' \
    "$id" "$agent" "$completion_commit" >> "$EVIDENCE/activity.jsonl"
  printf '{"ticket":"%s","agent":"%s","seat":"seat-1","outcome":"done","commit":"%s"}\n' \
    "$id" "$agent" "$completion_commit" >> "$EVIDENCE/provenance.jsonl"
}

run_ticket T-CDX-01 codex-first codex ""
run_ticket T-CDX-02 codex-second codex ""
run_ticket T-CDX-03 codex-third codex ""
run_ticket T-CDX-04 codex-fourth codex ""
run_ticket T-CDX-05 codex-dependent codex T-CDX-01
run_ticket T-MIX-01 mixed-claude claude T-CDX-05

git ls-files --stage -- foreign.txt > "$EVIDENCE/index.after"
cmp "$EVIDENCE/index.before" "$EVIDENCE/index.after"
git status --porcelain=v1 > "$EVIDENCE/status.final"
[ "$(cat "$EVIDENCE/status.final")" = 'M  foreign.txt' ] || {
  echo "unexpected final fixture status" >&2
  cat "$EVIDENCE/status.final" >&2
  exit 1
}

[ "$(grep -c '"event":"ticket_started".*"agent":"codex".*"seat":"seat-1"' "$EVIDENCE/activity.jsonl")" -eq 5 ]
[ "$(grep -c '"event":"ticket_started".*"agent":"claude".*"seat":"seat-1"' "$EVIDENCE/activity.jsonl")" -eq 1 ]
[ "$(grep -c '"outcome":"done"' "$EVIDENCE/provenance.jsonl")" -eq 6 ]

first_confirm_line="$(grep -n '"event":"completion_confirmed","ticket":"T-CDX-01"' "$EVIDENCE/activity.jsonl" | cut -d: -f1)"
dependent_start_line="$(grep -n '"event":"ticket_started","ticket":"T-CDX-05"' "$EVIDENCE/activity.jsonl" | cut -d: -f1)"
dependent_confirm_line="$(grep -n '"event":"completion_confirmed","ticket":"T-CDX-05"' "$EVIDENCE/activity.jsonl" | cut -d: -f1)"
mixed_start_line="$(grep -n '"event":"ticket_started","ticket":"T-MIX-01"' "$EVIDENCE/activity.jsonl" | cut -d: -f1)"
[ "$first_confirm_line" -lt "$dependent_start_line" ]
[ "$dependent_confirm_line" -lt "$mixed_start_line" ]

echo "PASS: six-ticket atomic provider contract; evidence at $EVIDENCE"
