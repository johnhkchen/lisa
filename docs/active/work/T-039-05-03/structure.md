# Structure: publication boundary regression

## Source inventory

Modify two existing files:

- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-core/src/provenance.rs`.

No file is created or deleted in maintained source. No manifest, schema,
fixture, CLI, documentation, or public API file changes.

## `publication.rs` production organization

Keep the existing module and types in place:

- `TemporaryName` remains the finite naming policy;
- `PublicationPath` remains destination plus temporary policy;
- `ResolvedPublicationPath` remains internal;
- `RustPublication` remains the Rust execution request;
- `ShellPublication` remains the shell rendering request;
- `shell_quote` and nonce generation remain unchanged.

Change `TemporaryName::resolve` to return a validated filename result. Add one
small private validator or perform the check directly after formatting. The
validation boundary is the complete resolved filename, not individual prefix
fields, so every policy receives the same rule.

## Filename invariant

Accepted resolved temporary names have:

- exactly one path component;
- that component is `std::path::Component::Normal`;
- no second component.

Rejected inputs include:

- empty string;
- current-directory component;
- parent-directory component;
- multi-component relative paths;
- rooted or absolute paths;
- platform prefixes where applicable.

Accepted inputs continue to include spaces, quotes, shell metacharacters,
backticks, dollar signs, and Unicode when they are ordinary filename bytes.

## Resolution interface

Change:

- `TemporaryName::resolve(self) -> String`

to:

- `TemporaryName::resolve(self) -> Result<String, String>`.

Change:

- `PublicationPath::resolve(self) -> ResolvedPublicationPath`

to:

- `PublicationPath::resolve(self) -> Result<ResolvedPublicationPath, String>`.

The returned resolved paths keep the destination unchanged and join the valid
temporary component to its parent.

## Publication interfaces

`RustPublication::publish` already returns `Result<PathBuf, String>` and will
propagate resolution failure before calling `fs::write`.

Change `ShellPublication::command` from `String` to `Result<String, String>`.
It will propagate resolution failure before rendering command text. The valid
command format and quoting remain exactly the same.

## Caller adaptation

`State::shell_readiness_probe` in `lib.rs` already returns `Result<String,
String>`. If the compiler requires a source edit, remove the wrapping `Ok` and
return `ShellPublication::command()` directly. No other caller signature changes.

If this adaptation modifies `lib.rs`, it becomes a third exact ticket-owned
commit path. Prefer the smallest necessary edit only.

## Direct test module

Add `#[cfg(test)] mod tests` at the bottom of `publication.rs`.

Test helpers may construct a standard `PublicationErrors` instance and invoke
private types directly. Avoid source-shape assertions and avoid sleeps or nonce
prediction.

Planned tests:

- repeated exact publication replaces once without duplicate/residue;
- failed rename preserves destination and removes complete temp;
- hostile temp traversal is rejected before cross-ticket mutation;
- all naming policies reject non-sibling components;
- hostile ordinary filename characters remain accepted;
- shell rendering rejects escape policies without executing a command.

The cross-ticket fixture uses adjacent directories with sentinels, making any
mixing or deletion directly observable.

## Provenance test organization

Production `append_record` remains unchanged.

Add one test to the existing `provenance.rs` inline test module. It will create
two independently minted ticket records, append them to one hostile-but-valid
ledger path, parse both lines, and assert exact outer/nested attribution.

No helper becomes public. The existing `sample()` test fixture can be cloned
and adjusted locally.

## Error surface

The new invalid-policy diagnostic belongs to the publication boundary rather
than a caller-specific write or publish label, because no I/O operation has
occurred. It contains the rejected filename but not the body.

Existing write and publish error strings remain unchanged after validation.
Existing call-site characterization continues to pin those messages.

## Ordering

1. Add validation and propagate the new result types.
2. Adapt shell readiness caller if required.
3. Add direct publication tests.
4. Run focused plugin tests.
5. Commit the plugin unit with exact paths.
6. Add the provenance attribution test.
7. Run core provenance tests.
8. Commit the core test unit with its exact path.
9. Run broad gates and cleanliness checks.

## Ownership boundary

Attempt artifacts are not passed to `lisa commit-ticket`; Lisa publishes those.
The modified ticket frontmatter and `.lisa/provenance.jsonl` are workflow-owned
and excluded. Only maintained source paths changed by this ticket enter isolated
source commits.

## Completion shape

At handoff:

- all direct and characterization regressions pass;
- workspace and Clippy gates pass;
- ticket-owned source has no staged, modified, or untracked residue;
- `progress.md` records implementation and command evidence;
- `review.md` summarizes changes, coverage, and open concerns.
