# Structure: publication-site characterization

## Change inventory

Modified source files:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-core/src/provenance.rs`

Created or deleted source files:

- none.

Attempt-private workflow artifacts:

- `.lisa/attempts/T-039-05-01/1/work/research.md`
- `.lisa/attempts/T-039-05-01/1/work/design.md`
- `.lisa/attempts/T-039-05-01/1/work/structure.md`
- `.lisa/attempts/T-039-05-01/1/work/plan.md`
- `.lisa/attempts/T-039-05-01/1/work/progress.md`
- `.lisa/attempts/T-039-05-01/1/work/review.md`

No artifact is written directly to `docs/active/work/T-039-05-01`.

## Production architecture retained

```text
scheduler callers in State
  -> site-specific preparation/publication helper
  -> same-directory temporary
  -> site-specific serialization
  -> rename to site-specific destination
```

The helpers remain separate:

```text
prepare_fresh_launch
prepare_assignment
write_pane_lease_marker
admit_artifact
shell_readiness_probe
```

Provenance remains:

```text
State::emit_provenance
  -> complete ProvenanceRecord
  -> provenance::append_record
  -> compact JSON + newline
  -> append-only ledger open/write
```

No shared publication module, option type, trait, or helper is introduced.

## Plugin test placement

Add tests inside the existing `#[cfg(test)] mod tests` in
`crates/lisa-plugin/src/lib.rs` near the current launch and shell-quote tests.

This location provides direct access to:

- private `State` publication methods;
- `ASSIGNMENT_FILE_NAME`;
- `ActivityEvent`;
- lease-installation test helpers;
- `State` internal activity storage;
- existing `tempfile` support.

No production visibility changes are required.

## Plugin helper for assertions

If repetition warrants it, add a test-local function that lists directory entry
names as strings. It must remain inside the test module and have no production
callers.

Possible shape:

```text
fn entry_names(path: &Path) -> Vec<String>
```

The result is sorted for deterministic diagnostics. Tests can filter by `.tmp`
or assert one residual shell temporary.

A second test-local assertion may validate a nonce-bearing name:

```text
fn assert_numeric_suffix(value: &str, prefix: &str)
```

It strips the fixed prefix and checks a nonempty ASCII-digit suffix. It does not
parse to a fixed-width integer.

## Plugin test 1 component layout

Name:

```text
publication_sites_preserve_serialization_and_collision_contracts
```

Use one `TempDir` and distinct child directories so state from one site cannot
mask another.

### Fresh launch component

```text
hostile root/launch path ' ; $(...)
  .lisa-launch-7.sh = old bytes
  prepare_fresh_launch(payload)
  .lisa-launch-7.sh = #!/bin/sh + payload + newline
```

Assert the returned string references only the quoted destination and contains
no raw payload.

### Assignment component

```text
hostile root/assignment path ' ; $(...)
  assignment.md = old bytes
  prepare_assignment(hostile raw bytes)
  assignment.md = exact hostile raw bytes
```

Assert the returned path equals the destination.

### Lease-marker component

Create a minimal `State` with the hostile signal directory. Seed
`pane-19.lease`, call `write_pane_lease_marker`, and compare both raw compact JSON
and deserialized `AttemptLease`.

### Admitted-artifact component

Create a `State` with hostile `work_dir` and `attempt_dir`. Install an exact
current lease in `current_leases`. Seed:

- staged `research.md` with new bytes;
- canonical `research.md` with old bytes;
- deterministic `.research.md.attempt-1.tmp` with collision bytes.

Call `admit_artifact` and assert:

- result is `Ok(true)`;
- canonical bytes equal staged bytes;
- staged source remains unchanged;
- deterministic temporary is gone.

### Shell-readiness component

Build a hostile signal directory and exact lease. Derive the expected temp-name
prefix from pane and attempt IDs. Seed an old destination. Execute the command
with `sh -c`. Assert destination replacement, exact JSON, no sentinel file, and
no temporary residue.

## Plugin test 2 component layout

Name:

```text
publication_sites_preserve_temp_names_cleanup_and_operator_errors
```

### Overlong temporary-name fixtures

Create an existing child directory whose leaf length is legal by itself but
leaves insufficient room for each publication filename. Invoke:

- `prepare_fresh_launch`;
- `prepare_assignment`;
- `write_pane_lease_marker`.

Each operation reaches its temp write and returns an error containing:

- the site-specific write prefix;
- the hostile directory path;
- the exact fixed temp prefix;
- an opaque numeric nonce suffix.

Because OS error wording varies, do not assert the trailing system message.

### Rust rename-failure fixtures

For launch, assignment, and lease marker, create the final destination as a
directory. Invoke the site and assert:

- site-specific `cannot publish ...` prefix;
- rendered destination path;
- destination directory still exists;
- no temp entries remain.

### Admitted-artifact failure fixture

Create the staged file and occupy the deterministic temporary path with a
directory. Assert the error begins with
`cannot write canonical artifact temporary`, contains the exact temp path, does
not change an old canonical file, and preserves the blocking directory.

### Shell-readiness failure fixture

Occupy `pane-{id}.shell-ready` with a directory and execute the returned probe.
Assert nonzero exit, unchanged destination directory, one residual temporary,
the correct pane/attempt prefix and numeric nonce, and exact JSON bytes in the
temporary.

## Plugin provenance failure test

Name:

```text
provenance_append_failure_is_logged_without_mutating_target
```

Place it beside the existing provenance ledger tests.

Fixture:

- create a directory at the configured ledger path;
- give the path hostile characters;
- install a thread and current attempt lease;
- call `emit_provenance` for Done.

Assertions:

- return value is false;
- ledger path is still a directory;
- directory remains empty;
- activity contains an Error message with ticket identity and the stable
  `provenance write failed` prefix;
- thread and current lease remain present.

## Core provenance test placement

Add one test to `crates/lisa-core/src/provenance.rs` inside its existing inline
test module.

Name:

```text
append_serialization_failure_preserves_existing_ledger
```

Fixture structure:

```text
TempDir
  hostile path ' ; $()/
    provenance.jsonl = one valid preexisting line
```

Build an otherwise valid sample with `cost_usd = Some(f64::NAN)`. Call
`append_record`, assert `InvalidData`, then compare the complete pre/post ledger
byte vectors for equality. This demonstrates that serialization completes
before the append handle is opened.

## Interface and dependency impact

- no new public items;
- no changed function signatures;
- no changed serialized fields;
- no new crate dependencies;
- no Cargo manifest changes;
- no configuration changes;
- no ticket frontmatter edits;
- no production code movement.

## Commit boundary

The two modified files form one meaningful characterization unit. Commit them
together with:

```text
lisa commit-ticket --ticket-id T-039-05-01 \
  --message "test: characterize atomic publication sites" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-core/src/provenance.rs
```

No other source or workflow path enters that isolated transaction.
