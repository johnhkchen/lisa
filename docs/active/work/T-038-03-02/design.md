# Design: four bounded cleanup units

## Decision summary

Land all four predecessor-authorized candidates as three source units:

1. Add one private OS-filename parser and focused table-driven tests for C-01.
2. Give `AgentAdapter` shared defaults for C-02 and C-03, retaining all
   provider-specific methods and existing behavior assertions.
3. Add one script-local `event_count` primitive for C-04 and prove it through
   the real-Zellij integration harness.

Do not extract signal loops, provider assignment behavior, cross-harness
helpers, fixture builders, hook schemas, or historical evidence.

## Design criteria

- Every change must correspond directly to C-01, C-02, C-03, or C-04.
- Behavior must be identical before and after extraction.
- Each unit must have a focused passing proof.
- Parser extraction must remain pure and scheduler-independent.
- Adapter defaults must preserve future override points.
- Harness extraction must remain local to one executable fixture.
- No change may reorder polling, reads, deletes, state transitions, or commands.
- No public API needs to be introduced.
- Each source file must form an exact-path Lisa commit unit.

## C-01 option A: helper over UTF-8 `&str`

Shape:

`fn pane_id_from_signal_name(name: &str, suffix: &str) -> Option<u32>`

Advantages:

- Very small signature.
- Directly expresses prefix, suffix, and numeric parsing.
- Easy table-driven tests for ordinary strings.

Disadvantages:

- Every caller must continue its own `OsStr::to_str` chain.
- The repeated filename-conversion boundary remains at seven sites.
- Non-UTF-8 rejection is outside the helper and harder to prove directly.
- Call sites do not become a single expression from filesystem filename to id.

Decision: reject in favor of owning the complete repeated grammar boundary.

## C-01 option B: helper over `&Path`

Shape:

`fn pane_id_from_signal_path(path: &Path, suffix: &str) -> Option<u32>`

Advantages:

- Call sites can pass the directory-entry path directly.
- Filename extraction and UTF-8 rejection are centralized.
- Tests can naturally pass path fixtures.

Disadvantages:

- The grammar is about one filename, not a path.
- Passing paths makes directory components look semantically relevant.
- A path ending in a valid filename would be accepted even though only its final
  component participates, which is correct but less explicit.

Decision: viable, but reject because the narrower OS-filename input documents
the contract better.

## C-01 option C: helper over `&OsStr`

Shape:

`fn pane_id_from_signal_filename(filename: &OsStr, suffix: &str) -> Option<u32>`

Behavior:

- Convert the OS filename with `to_str`; reject non-UTF-8.
- Strip the exact `pane-` prefix.
- Strip the exact caller-provided suffix.
- Parse the remaining bytes as `u32`.

Advantages:

- Captures the complete repeated pure grammar.
- Does not imply any path or filesystem behavior.
- Directly supports a non-UTF-8 rejection test.
- Keeps suffix choice explicit at every consumer.
- Cannot absorb payload, deletion, or state-machine policy.

Disadvantages:

- Tests need `OsStr` values rather than only strings.
- Constructing invalid UTF-8 is platform-specific in the test.

Decision: choose Option C. Use `std::ffi::OsStr` in the private helper signature
and gate only the non-UTF-8 test with `#[cfg(unix)]`.

## C-01 transition-consumer preservation

Most consumers currently parse a valid pane id before deleting the signal.
Their call sites can directly replace the repeated chain with:

`path.file_name().and_then(|name| pane_id_from_signal_filename(name, suffix))`.

The transition consumer differs: it removes any UTF-8 filename with the
recognized `.stopped` or `.cleared` suffix before numeric parsing succeeds.
Changing malformed-file cleanup is outside this ticket. Therefore it will:

- retain explicit suffix recognition in its existing order;
- retain removal immediately after recognizing each suffix;
- call the shared helper only for pane-id grammar;
- continue on parse failure;
- keep stopped and cleared effect ordering unchanged.

This leaves a small amount of suffix branching by design. Extracting the whole
transition scanner would violate the C-05 boundary.

## C-01 parser test design

Use a table of filename, suffix, and expected `Option<u32>` covering:

- the minimum id `0`;
- a normal decimal id;
- `u32::MAX`;
- leading zeroes;
- wrong prefix;
- wrong suffix;
- suffix text not anchored at the end;
- empty id;
- non-numeric id;
- negative id;
- whitespace in the id;
- overflow past `u32::MAX`;
- an empty suffix that cannot admit an otherwise suffixed signal incorrectly.

Add a Unix-only test using `OsStringExt::from_vec` to prove non-UTF-8 rejection.
Existing consumer tests remain independent evidence for scheduler effects.

## C-02/C-03 option A: free helper functions

Add free functions for the shared reset and follow-up values, then call them
from both provider implementations.

Advantages:

- Removes repeated construction internals.
- Leaves required trait methods explicit in each implementation.

Disadvantages:

- Two provider methods remain duplicated forwarding shells.
- A future provider must discover and call the helpers manually.
- The shared native default policy is not expressed at the trait seam.

Decision: reject. It removes fewer divergence points than a trait default.

## C-02/C-03 option B: native-adapter base type

Introduce a base struct or secondary trait containing shared native behavior.

Advantages:

- Could centralize more provider behavior later.

Disadvantages:

- Adds inheritance-like structure for two one-expression methods.
- Encourages C-13 provider assignment behavior to migrate into the abstraction.
- Creates a broader architecture than the demonstrated cleanup requires.

Decision: reject as disproportionate and boundary-expanding.

## C-02/C-03 option C: `AgentAdapter` defaults

Give the existing crate-private trait these defaults:

- `reset_strategy` returns `ResetStrategy::ClearHandshake`.
- `follow_up` constructs the existing `finish_up_prompt` and returns
  `FollowUp::TypeIntoPane`.

Remove only the identical Claude and Codex implementations.

Advantages:

- Expresses current common native policy once.
- Existing trait-object calls prove dispatch through the default.
- Future adapters retain an explicit override mechanism.
- `FreshExec` and `SpawnCommand` remain available.
- Provider-specific behavior stays in each implementation.

Disadvantages:

- A future adapter might inherit a native default unintentionally.
- Defaults make the native policy less visually repeated at implementations.

Decision: choose Option C. The method documentation will say these are native
defaults and future integrations override them when their transport differs.
The current comprehensive per-provider assertions mitigate accidental drift.

## C-02/C-03 proof design

Retain rather than deduplicate the existing assertions because C-14 identifies
their independence as regression evidence. Specifically:

- Claude follow-up still compares the complete prompt wrapper.
- Codex follow-up still compares the complete prompt wrapper.
- Claude reset still equals `ClearHandshake`.
- Codex reset still equals `ClearHandshake`.
- Resolver tests still exercise trait objects under multiple routing cases.

No mock adapter is necessary: both concrete adapters inheriting the defaults
and passing their existing tests is direct evidence of the change.

## C-04 option A: duplicate `awk`, share log path

Create a variable or function only for the event-log filename.

Decision: reject. It leaves the repeated counting policy and missing-file
fallback intact.

## C-04 option B: parameterized comparison helper

Create a function accepting the event kind, operator, and expected value.

Advantages:

- Could collapse both predicates into one implementation.

Disadvantages:

- Dynamic shell operators require branching or evaluation.
- `eval` would be unjustified and unsafe.
- Named exact and lower-bound predicates are clearer at call sites.

Decision: reject. Comparison intent should remain explicit.

## C-04 option C: value-producing `event_count`

Create `event_count(kind)` which prints zero if the log does not exist and
otherwise prints the existing `awk` count. Each predicate captures the value
and applies its existing comparison.

Advantages:

- Centralizes precisely the duplicated policy.
- Preserves readable predicate names and comparisons.
- Preserves the missing-log result.
- Remains local to the deterministic harness.

Disadvantages:

- Adds command substitution at both predicate sites.

Decision: choose Option C. The helper output is a single base-10 integer, so
command substitution has no ambiguity.

## Commit and verification design

- Commit `lib.rs` alone for C-01 after focused plugin tests.
- Commit `adapter.rs` alone for C-02/C-03 after adapter tests.
- Commit the deterministic shell fixture alone for C-04 after its ignored
  integration test prints the PASS receipt.
- Use `lisa commit-ticket --ticket-id T-038-03-02` with one exact include path
  per transaction.
- Run formatting and syntax checks before their corresponding commits.
- Run full workspace tests and clippy after the integrated units are committed.
- If the real-Zellij harness cannot run for an environmental reason, do not
  claim C-04 complete; diagnose within the current ticket.
- Leave C-05 through C-14 named and unchanged in Review.
