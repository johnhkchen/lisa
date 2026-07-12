# Structure: atomic publication boundary

## File inventory

Create `crates/lisa-plugin/src/publication.rs` and modify
`crates/lisa-plugin/src/lib.rs`. Do not change core provenance, CLI commit
transactions, manifests, ticket frontmatter, predecessor tests, or shared work.

## Module responsibility

`publication.rs` owns:

- wall-clock nonce generation;
- typed temporary-name construction;
- same-parent temp resolution;
- Rust write/rename/failure cleanup;
- shell publication command rendering;
- POSIX shell quoting.

It excludes:

- directory creation and site-specific directory errors;
- scheduler authority and attempt validation;
- serialization schemas and staged reads;
- `/host` path translation;
- provenance append and Git transactions;
- activity logging.

## `TemporaryName`

Define crate-private variants equivalent to:

```text
Nonce { prefix }
AttemptNonce { prefix, attempt_id }
Exact { file_name }
```

Resolution maps them to `{prefix}{nonce}`, `{prefix}{attempt_id}-{nonce}`, or
the exact filename. Fields represent sibling filenames, not arbitrary paths.

## `PublicationPath`

Contain a `PathBuf` destination and `TemporaryName`. Resolve the destination
parent once and join the typed sibling name. Keep the resolved pair private so
callers cannot separate the temp from the destination directory.

## `PublicationErrors`

Contain named `write` and `publish` static labels. The four mappings are:

| Site | Write label | Publish label |
|---|---|---|
| launch | cannot write launch payload | cannot publish launch payload |
| assignment | cannot write assignment payload | cannot publish assignment payload |
| lease | cannot write pane lease marker | cannot publish pane lease marker |
| artifact | cannot write canonical artifact temporary | cannot publish canonical artifact |

## Rust publication

Expose a crate-private function or options method accepting path policy, `&[u8]`,
and labels. It resolves once, writes exact bytes, renames, cleans temp
best-effort after rename failure, and returns the destination. Error formatting
remains `{label} {displayed_path}: {io_error}`.

## Shell publication

Expose a crate-private command renderer accepting path policy and serialized
body text. It resolves one nonce and renders exactly the existing quoted
`command printf '%s' ... > ... && command mv ... ...`. It performs no Rust I/O,
cleanup, or collision normalization.

## `lib.rs` wiring

Add `mod publication` and import the typed options. Re-export the module's
`shell_quote` crate-visibly so adapter and inline tests retain the same surface.

## Call-site edits

Fresh launch retains directory creation, destination, script serialization,
host stripping, and bounded return command. It delegates nonce/temp/write/rename.

Assignment retains directory creation, destination, raw payload, and return
semantics. It delegates through `.assignment.md.tmp.`.

Lease marker retains configuration checks, directory creation, destination, and
compact JSON serialization. It delegates through attempt-bearing naming.

Artifact admission retains lease validation, source existence/read, directory
creation, and boolean outcomes. It delegates using the exact deterministic temp.

Shell readiness retains JSON serialization and host stripping. It delegates
nonce generation, path resolution, quoting, and exact command rendering.

## Dependency direction

```text
State call sites
  -> publication options
     -> std::fs or shell command text

State call sites
  -> authority, serialization, directories

emit_provenance -> lisa-core append (unchanged)
completion command -> lisa-cli Git transaction (unchanged)
```

The module does not import `State`, `AttemptLease`, ticket, or provenance types.
Attempt identity reaches it only as the numeric temporary-name option.

## Test structure

Predecessor characterization stays in the `lib.rs` inline test module. Tests
continue calling the five `State` helpers and therefore exercise the new module
without visibility changes. Direct boundary tests remain for the successor.

## Commit unit

The module and routing edit are one compiling source unit. Commit exactly:

```text
crates/lisa-plugin/src/publication.rs
crates/lisa-plugin/src/lib.rs
```

through `lisa commit-ticket`. Workflow artifacts are not source includes.

