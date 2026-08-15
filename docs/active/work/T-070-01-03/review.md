# T-070-01-03 — a review asks the operator for a command that does not exist

`lisa check-disposition` now reads every `lisa` verb a blocking disposition
names — in `reason`, `ask`, `steps`, and `check` — against the running binary's
own subcommand list, and refuses one that binary does not have. The reviewer is
still there when it objects; the operator is not.

## What changed

| File | Change |
| --- | --- |
| `crates/lisa-cli/src/disposition_verbs.rs` | New. The vocabulary, the scanner, the "did you mean", the message, and the board sweep. |
| `crates/lisa-cli/src/check_disposition.rs` | Blocks are scanned before the recorded `check` is run. |
| `crates/lisa-cli/src/main.rs` | Module declaration. |
| `crates/lisa-cli/tests/check_disposition_cli.rs` | Two black-box cases: a step naming a verb this lisa lacks is refused; a step naming real verbs (hidden and nested included) passes untouched. |
| `crates/lisa-cli/data/lisa-workflow.md`, `docs/knowledge/lisa-workflow.md` | The rule, stated where the agent writing a block reads it. |

Two commits through `lisa commit-ticket`: `c8ef7ea` (guard) and `f39aecb`
(contract wording). Working tree clean of ticket-owned files.

### Where the vocabulary comes from

`Vocabulary::from_command(&Cli::command())` — clap's own tree, so the answer is
what `lisa --help` would print *from this very executable*. Nobody maintains a
list; adding a subcommand teaches the guard about it in the same commit. Hidden
subcommands count (`lisa claim` is not in `--help` and runs fine), aliases
count, and a subcommand's own actions are checked one level down, so
`lisa nightly frobnicate` is caught as well as `lisa frobnicate`.

This also decides the version-skew question the right way round. The binary
running `check-disposition` is the same *class* of binary the operator has — an
installed release, not the working tree — so a verb that only exists on `main`
is refused rather than shipped to someone whose lisa cannot run it.

### Finding invocations rather than the word

A `lisa` token counts as something to type when it starts a line, is quoted or
backticked, follows a shell lead-in (`:`, `;`, `&&`, `|`, `$`, `(`), or follows
a run-verb (`run`, `then`, `try`…). Prose is left alone: "the lisa binary",
"two lisas shadowing each other", `.lisa/attempts/…`, `crates/lisa-cli`,
`docs/knowledge/lisa-workflow.md`, and any sentence that capitalises Lisa. A
word this binary lacks that reads like English (`lisa is stale`) is not
complained about either. A guard that objects to sentences is one reviewers
learn to write around, and then it protects nobody.

### The message

```
Error: Fix review-disposition.json: step 1 names `lisa uprade`, and lisa 0.5.0 has no uprade subcommand.
  step 1: Prove the way back, on the mini: lisa uprade --tag v0.4.4
  closest: lisa upgrade
  step 2: Then check the schedule: lisa nightly statuss --json
  closest: lisa nightly status
Fix: name a subcommand this lisa has — `lisa --help` lists them — or, if the verb is real
and newer than this binary, say in the step which lisa version it needs and how to get
there. If `lisa` here is prose rather than something to type, write it as Lisa.
```

The version is in the lead line on purpose. The two ways to be wrong here need
opposite fixes — a verb nobody built has to go, a verb newer than the reader's
binary has to say which release it needs — and the operator hit the second
while reading it as the first.

## The two questions the ticket asks

**Is `T-068-01-03` the only one?** Yes. All 126 published
`docs/active/work/*/review-disposition.json` were swept for `lisa <word>`
mentions. Exactly one file names any `lisa` verb at all:

| Ticket | Where | Verb |
| --- | --- | --- |
| `T-068-01-03` | `reason`, `step 3` | `upgrade` |
| `T-068-01-03` | `step 4` | `status` |
| `T-068-01-03` | `step 5`, `step 7` | `doctor` |

(Two further matches, `lisa in ~/.local/bin` and `lisa at /opt/homebrew/bin`,
are prose about where the binary lives — the same false positives the scanner's
prose rule declines, which is a small real-world check on that rule.) Nothing
else on the board tells anyone to run lisa at all. The sweep is not a one-time
answer: `no_published_disposition_asks_for_a_verb_this_binary_lacks` re-reads
every published disposition on each `cargo test`, which is the half that catches
what was written before the guard existed.

**Built, or stopped being asked for?** *Built* — and the ticket's premise needs
one correction. `lisa upgrade` was not invented: `T-068-01-01` built it
(`c86f294`, 2026-08-14) and it shipped in **`v0.5.0-rc.3`**, tagged the same
day. `lisa nightly` likewise, from `T-068-01-03` itself (`f809898`). The dev
desk that measured `unrecognized subcommand` was running **`0.5.0-rc.2`** — one
release behind the tag that contains the verb.

So the fleet does have a supported way back; what it did not have was a way for
the writer of a step to notice that the reader's binary predates it. That is
exactly what this guard is, and it is why it asks the running binary rather than
the source tree: on that desk, `lisa check-disposition` would have refused
step 3 — for the right reason, one turn after it was written.

`T-068-01-03` is a block owned by the operator and its step 3 is now correct for
any lisa at or past `v0.5.0-rc.3`. It is not edited here; that ticket's remedy
is still theirs to perform.

## Tests

`just check` — fmt, clippy, `cargo check` for wasm, and `cargo test --workspace`
— passes, exit 0. New coverage:

- `disposition_verbs`, 10 unit tests: the field step verbatim; invocations in
  every position they occur in real steps; real verbs including hidden and
  nested ones; nested actions checked against their own subcommands; prose left
  alone (six real sentences from this repo); the suggestion; every block field;
  the message; the vocabulary read off `Cli::command()`; the board sweep.
- `check_disposition_cli`, 2 black-box tests through the built binary:
  `a_step_naming_a_verb_this_lisa_lacks_is_refused_at_record_time` and
  `a_step_naming_real_verbs_passes_untouched`.

Reproduced by hand as the ticket asks, against `target/debug/lisa` on a
throwaway board — a step reading
`Prove the way back, on the mini: lisa rollback --tag v0.4.4` is refused with
exit 1 and the message above, before any operator sees it.

## What still concerns me

1. **A newer agent than its operator still slips through.** The guard answers
   for the binary running it. On this desk that binary was *older* than the
   tree, which is the safe direction. The reverse — an agent on a fresh lisa
   writing a step for an operator on an old one — passes the check and still
   fails in their terminal. The message asks for a version when the verb is new,
   which is guidance, not enforcement. Fixing it properly means dispositions
   carrying a minimum version, which is a bigger contract change than this bug.
2. **Lisa's own internal-command blocks are not scanned.** The asks generated in
   `crates/lisa-plugin` name `lisa unblock` and `lisa already-done`; both are
   real and both are compiled and reviewed, but the plugin is wasm and cannot
   see clap, so a hand-maintained list would be the only way to guard them —
   the exact thing this design avoids. Left alone deliberately.
3. **The prose word list is a list.** `PROSE_WORDS` is consulted only for a word
   this binary does not have as a subcommand, so it can never hide a real verb;
   the worst it does is decline to complain about an invented verb that reads
   like English (`lisa needs`). Verified against the only real board data there
   is — the two `lisa in` / `lisa at` mentions above.
4. **Only `lisa` verbs are checked, by design.** A step naming `brew switch`,
   a flag that no longer exists, or a path on another machine is still nobody's
   to verify. The ticket asks for exactly this scope, and widening it would put
   the guard in the business of judging other people's tools.
