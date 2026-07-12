# Build and deterministic test evidence

## Source revision

`0ffe40f67551774964cfaf3e229ba5052cee43ea`

This is the prerequisite T-034-03-01 commit and the parent repository `HEAD`
used for the proof.

## Toolchain

- Rust: `rustc 1.99.0-nightly (c4af71034 2026-07-06)`
- Cargo: `cargo 1.99.0-nightly (2f0e7011e 2026-07-05)`
- Zellij: `0.44.3`
- Codex CLI: `0.144.1`
- Claude Code: `2.1.207`
- Fresh Lisa: `0.4.0-rc.5`

## Fresh release build

Commands:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
cargo build -p lisa-cli --release
```

Both completed successfully.

The actual plugin output is:

`target/wasm32-wasip1/release/lisa.wasm`

The plan's initial `lisa_plugin.wasm` spelling was corrected after inspecting
the release directory.

The release CLI exposed both current isolated transaction subcommands:

- `commit-ticket`
- `complete-ticket`

It was copied to the temporary proof installation at:

`<fixture>/bin/lisa`

The Homebrew Lisa at `/opt/homebrew/bin/lisa` did not drive either loop.

## Hashes

Release WASM:

`cfac4d9390a0898682a4d262a1bf3a4b042608cf0db5a1f947643659f5f63ce8`

Loop-extracted WASM:

`cfac4d9390a0898682a4d262a1bf3a4b042608cf0db5a1f947643659f5f63ce8`

Fresh installed Lisa CLI:

`c01d0eda63b793725a2d3e6c81888b6cad388bb0f815bf0a7af4bf8677075094`

The target and loop-extracted WASM hashes match exactly.

The runtime path was content-hashed as:

`$TMPDIR/lisa-plugin-547be5a7957a5b25.wasm`

The generated layout's `lisa_bin` field named the temporary fresh CLI copy.

## Committed split-brain regression

Command:

```text
cargo test -p lisa-plugin \
  split_brain_timeline_fences_old_attempt_and_admits_one_winner \
  -- --nocapture
```

Result:

```text
running 1 test
test tests::split_brain_timeline_fences_old_attempt_and_admits_one_winner ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 272 filtered out
```

The named regression executes the production timeout, lease revocation,
fencing, redispatch, missed Codex acknowledgement, stale signal rejection,
private artifact admission, completion, and provenance methods.

Its assertions prove:

- `LeaseRevoked -> PaneFenced -> SlotReleased` ordering;
- the fenced predecessor pane is not redispatched;
- predecessor heartbeat, ack, idle, stopped, cleared, and error signals do not
  mutate successor state;
- predecessor private artifact bytes cannot reach the canonical work path;
- stale completion cannot enter the isolated transaction;
- exactly one authoritative Done record belongs to attempt 2.

## Broad verification

Commands:

```text
cargo test -p lisa-plugin
cargo fmt --all -- --check
cargo check -p lisa-plugin --target wasm32-wasip1
git diff --check -- docs/active/work/T-034-03-02
```

Results:

- plugin tests: 273 passed, 0 failed;
- formatting check: passed;
- WASM target check: passed;
- ticket work whitespace check: passed.

No parent source file was modified by T-034-03-02.
