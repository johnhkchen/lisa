# T-017-05 Design: Tag and cut alpha release

## Decision 1: Version Number

### Option A: Keep `0.1.6`
- **Pro**: Zero additional work — no files to edit, no extra commit, no extra CI cycle
- **Pro**: Simpler — ship what's already configured
- **Con**: Doesn't communicate the scope of changes (doctor, cargo-dist, docs, distribution infra, 5+ stories of work)
- **Con**: `0.1.x` implies a patch bump from a previous `0.1.5`, but no `0.1.5` release exists publicly

### Option B: Bump to `0.2.0`
- **Pro**: Signals a meaningful milestone — this is the first public release with working install pipeline
- **Pro**: Semantically correct: new features (doctor, init improvements), new infrastructure (cargo-dist, homebrew)
- **Con**: Requires editing 2 files, cargo check, commit, push, wait for CI green — adds ~10 min
- **Con**: Introduces risk of typo or missed file

### Decision: **Bump to `0.2.0`**

Rationale: This is the first public alpha release. Starting at `0.2.0` signals "first usable milestone" better than `0.1.6` which looks like a patch on something that was never publicly released. The extra work is trivial (2 file edits + cargo check).

Files to edit:
1. `Cargo.toml` line 6: `version = "0.1.6"` → `version = "0.2.0"`
2. `crates/lisa-cli/Cargo.toml` line 19: `lisa-core = { version = "0.1.6", ...}` → `lisa-core = { version = "0.2.0", ...}`
3. Run `cargo check` to regenerate `Cargo.lock`

## Decision 2: Tag Strategy

### Approach
1. Create a lightweight tag (not annotated) — cargo-dist doesn't require annotated tags
2. Tag format: `v0.2.0` — matches the regex in release.yml and cargo-dist convention
3. Push only the tag — the code is already on main from T-017-04

### Sequence
```
1. Verify T-017-04 is done (CI green on main)
2. Edit version in Cargo.toml and crates/lisa-cli/Cargo.toml
3. cargo check (regenerates Cargo.lock)
4. git add + commit "Bump version to 0.2.0"
5. git push origin main (wait for CI green on the bump commit)
6. git tag v0.2.0
7. git push origin v0.2.0
8. Monitor release workflow
9. Verify GitHub Release page has expected artifacts
```

## Decision 3: Handling Homebrew Failure

The `publish-homebrew-formula` job will fail because `HOMEBREW_TAP_TOKEN` is likely not configured. Per the ticket, this is expected and non-blocking.

**Action**: Note the failure when monitoring. The `host` job creates the GitHub Release independently of homebrew. The `announce` job uses `needs.publish-homebrew-formula.result == 'skipped' || needs.publish-homebrew-formula.result == 'success'` which means if homebrew *fails* (rather than skips), announce won't run. However, the release will already be created by the `host` step.

**Mitigation**: If homebrew is not configured, it will still run and fail. The release artifacts will exist regardless. This is acceptable for alpha.

## Rejected Alternatives

- **Annotated tag**: Not needed. cargo-dist handles lightweight tags fine. Annotated tags add no value here.
- **Pre-release tag (v0.2.0-alpha.1)**: Adds complexity. cargo-dist would mark the release as prerelease, which is fine, but `v0.2.0` is cleaner for a first release that's already "alpha" by nature.
- **Keeping 0.1.6**: Rejected per Decision 1 rationale.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| WASM build fails in release CI | Medium | High (no binaries) | build-setup.yml tested in T-014-01; we can fix and re-tag |
| Version mismatch between tag and Cargo.toml | Low | High (dist errors) | Explicit verification step before tagging |
| Homebrew publish fails | High | Low | Expected, documented, non-blocking |
| CI fails on version bump commit | Low | Low | Trivial fix, re-push |
