# Structure — T-051-01-03 defang-the-checksum-flake

## Scope of file changes

| File | Disposition | Extent |
| --- | --- | --- |
| `crates/lisa-cli/src/runtime.rs` | **modified** | test module only (`#[cfg(test)] mod tests`) |

No files created. No files deleted. No production code path modified. No
`Cargo.toml`, manifest, or fixture-data change. One file, one ticket, no overlap
with any sibling ticket's ownership.

`libc` is already a dependency of `lisa-cli` (`Cargo.toml:36`) but the chosen fix
needs no new imports at all — `TcpStream::set_nonblocking` is `std`.

---

## Change 1 — `FixtureServer::start`, clear the inherited `O_NONBLOCK`

Location: `crates/lisa-cli/src/runtime.rs`, inside the accept loop of
`FixtureServer::start` (currently lines 640–681), in the `Ok((mut stream, _))`
arm at line 651.

Current shape:

```rust
match listener.accept() {
    Ok((mut stream, _)) => {
        thread_requests.fetch_add(1, AtomicOrdering::SeqCst);
        read_request_headers(&mut stream);
        ...
```

Target shape — one statement inserted before the request is read, carrying the
rationale comment (AC4):

```rust
Ok((mut stream, _)) => {
    // BSD/macOS `accept()` inherits O_NONBLOCK from the listening socket,
    // which is non-blocking here only so the accept loop can poll its
    // deadline. Left inherited, the accepted stream makes the SO_RCVTIMEO in
    // read_request_headers meaningless: the first read returns EWOULDBLOCK,
    // the request is abandoned unread, and closing a socket with unconsumed
    // inbound data makes the kernel send RST instead of FIN — killing the
    // client's in-flight body read. That is load-sensitive (the client
    // sometimes drains first), which is what made the checksum test flake
    // ~4/32 under parallel load while the guard under test was correct.
    // Restoring blocking mode deletes the race; it adds no retry and no delay.
    stream.set_nonblocking(false).unwrap();
    thread_requests.fetch_add(1, AtomicOrdering::SeqCst);
    read_request_headers(&mut stream);
    ...
```

Ordering constraint: the `set_nonblocking(false)` call must precede
`read_request_headers`. Placing it after would leave the header read racing.

Boundary constraint: the **listener** must remain non-blocking. The 5-second
accept-deadline poll (lines 648, 665–670) depends on `accept()` returning
`WouldBlock`; making the listener blocking would hang the fixture thread when a
test never connects.

Invariants preserved:

- `request_count()` semantics unchanged — the counter still increments once per
  accepted connection, still before the response is written.
- The 5-second accept deadline unchanged.
- `FixtureResponse::Interrupted` behaviour unchanged — it still advertises
  `body.len() + 1024` and still truncates, so
  `interrupted_download_leaves_no_torn_runtime_directory` keeps failing on
  purpose. Blocking mode affects *whether the request is read*, not *what the
  response contains*.
- `Drop for FixtureServer` / `join()` unchanged.

Blast radius: all four `FixtureServer` consumers
(`successful_fetch_verify_and_atomic_store`,
`checksum_mismatch_is_named_and_leaves_no_partial_install`,
`interrupted_download_leaves_no_torn_runtime_directory`,
`second_managed_resolution_performs_zero_network_calls`). Two of the four were
observed flaking from this cause; the repair is expected to retire both.

### Why not `read_request_headers` instead

The alternative site is `read_request_headers` (lines 700–712), which could
defend itself by treating `WouldBlock` as retryable. Rejected in Structure for
the same reason Design rejected retries: it would convert a deterministic
one-line correction into a polling loop, and it would leave `write_response` —
which also assumes blocking semantics and `unwrap()`s its writes — still exposed
on a non-blocking socket. Fixing the socket's mode fixes both call sites at
once. `read_request_headers` and `write_response` are left byte-for-byte
unchanged.

---

## Change 2 — `checksum_mismatch_is_named_and_leaves_no_partial_install`, self-diagnosing assertions

Location: `crates/lisa-cli/src/runtime.rs`, lines 1042–1066, assertion block at
lines 1059–1065.

Current shape — six bare assertions that print nothing:

```rust
assert!(error.contains("Managed Zellij checksum mismatch"));
assert!(error.contains(&server.url));
assert!(error.contains(expected_sha256));
assert!(error.contains(&actual_sha256));
assert_eq!(server.request_count(), 1);
assert!(!executable.parent().unwrap().exists());
assert_no_install_temporary_directories(&executable);
```

Target shape — same seven checks, none weakened, each carrying enough context to
identify itself:

- A short comment above the block stating the contract being asserted: the guard
  must *reject* the archive and *name* the mismatch with both hashes and the
  URL, and must leave nothing behind.
- The category assertion carries `{error}` and `request_count` in its message,
  so a future failure immediately distinguishes:
  - guard misfired (request served, wrong category), from
  - upstream transport failure (`download failed` category), from
  - fixture never served (`request_count == 0`).
- The URL / expected-sha / actual-sha assertions each carry `{error}`.
- The two no-partial-install assertions carry the offending path.

No assertion is removed, relaxed, or converted to a warning. The set of
conditions that turn this test red is **identical** before and after; only the
failure output changes. This matters for AC3: the negative fixture's red-ness is
carried by the unchanged assertions, not by the new messages.

Explicitly out of scope for this change: introducing an error enum on
`ensure_managed_zellij` so the test could match a variant. That is a production
change and Design rejected it under the ticket's no-production-change constraint.

---

## Ordering of changes

1. **Change 1 first** (harness). It is the behavioural fix; it must be in place
   before any tally run is meaningful.
2. **Change 2 second** (assertions). Diagnostic only; independent of Change 1's
   correctness.

Both land in a single file. They are separable in principle but are one logical
unit — "make this test tell the truth" — and will be committed as one unit
through `lisa commit-ticket` with an exact `--include` path.

---

## Verification structure

Executed in Implement, in this order:

1. `cargo test -p lisa-cli --bins runtime::tests` — the four fixture tests green
   in isolation. Judged by exit code.
2. **Mutation M1** — `runtime.rs:471`, `if actual_sha256 != release.sha256` →
   `if false`. Expect red at `.unwrap_err()`. Revert.
3. **Mutation M2** — disarm `TempInstall`'s drop-cleanup. Expect red at
   `assert_no_install_temporary_directories` (or the parent-exists assertion).
   Revert.
4. `just check` — fmt + clippy + workspace tests, by exit code.
5. **32× concurrent** compiled-test-binary run — the exact harness that
   reproduced 4/32. Expect 0 checksum failures.
6. **20 consecutive** `cargo test -p lisa-cli` runs (AC2), tallied by exit code,
   with confounders disclosed.

Mutations touch production code and must **never** be staged or passed to
`lisa commit-ticket`. After each, `git diff crates/lisa-cli/src/runtime.rs` must
show only the two intended test-module changes.

## Ownership and concurrency

`crates/lisa-cli/src/runtime.rs` is the sole file this ticket writes. Sibling
tickets active on this branch were observed editing `crates/lisa-plugin/src/*`
(T-051-02-01) — disjoint. `cargo test -p lisa-cli` does not compile
`lisa-plugin`, so this ticket's tally is insulated from that in-flight work;
`just check` builds the workspace and is not, which will be accounted for when
reading its result.
