# Review — T-046-06-03 closing acceptance run

## Disposition

Pass.

The recorded closing matrix satisfies the ticket and story after the operator's
2026-07-17 amendments aligned the acceptance contract with the calibrated
runbook and the platform-aware managed-runtime behavior.

Both primary low-end-agent legs are doctor-green, dry-run-green, hands-off,
under ten minutes, under the calibrated 300 MiB disk bound, and graded free of
the prohibited Rust/compiler path.

The seeded Zellij 0.40.1 variant demonstrates that the below-floor PATH binary
is unreachable under the default managed-runtime configuration.

The tour rematch names Claude Code and Codex, states the no-per-step-approval
benefit before mechanism, and identifies a durable audit trail.

No product source changed in this Review.

No ticket phase or status field was edited.

## Review scope

The ticket entered this attempt at `phase: review`.

Earlier Research, Design, Structure, Plan, and Implement artifacts were already
admitted under `docs/active/work/T-046-06-03/`.

This attempt therefore performed the remaining Review phase only.

The review covered:

- the current ticket and story acceptance text;
- the operator amendment and its recorded rationale;
- the current Chromebook test runbook and grader;
- both primary run records and metadata files;
- all captured install sections and exact instructions;
- Docker changed-path summaries;
- the seeded-old-Zellij run and operator interpretation;
- the tour-rematch HTML and landing-probe series comparison; and
- ordinary Git index and worktree ownership state.

## Governing contract

The ticket was amended by commit `8bf4d52` after the prior Review identified
two mismatches between the recorded evidence and the old acceptance wording.

The primary disk limit is now the runbook's calibrated 300 MiB bound with
recorded composition.

The seeded-old-Zellij criterion now accepts either diagnostic-driven recovery
or a recorded default path in which managed runtime resolution makes the old
binary unreachable.

These changes are explicit in both the ticket and parent story.

The run evidence itself was not altered by the amendment.

The previous blocking rationale remains preserved in the shared Review and in
`operator-note-2026-07-17.md`.

## Runbook and grader verification

The current runbook requires:

- a Haiku-class Claude leg and a mini-class Codex leg;
- the live README install section and exact instruction A;
- independent PATH, doctor, init, validate, and dry-run checks;
- no more than 600 seconds;
- no more than 300 MiB, with composition recorded;
- no `rustc`, `cargo`, `rustup`, `xz`, `gcc`, `cc`, `g++`, or `make`;
- no `~/.rustup` or Cargo registry;
- no Lisa or Zellij source-build checkout; and
- recorded sudo/apt actions.

`docker/chromebook-test/bin/grade` sets the outcome to PASS only after the
positive, timing, disk, and prohibited-tool checks hold.

Each collected primary record reports `outcome: PASS` from that grader.

The four collected install-section files share SHA-256
`54024af2a9d4b3e373ebe5728907d22612c24199010d5d5a3c3e26fa2aec0a4b`.

Re-extracting the current README install section produces the same hash.

The four exact instruction files also match one another byte-for-byte.

## Primary Claude leg

Evidence directory:
`cbt-0716-205306-closing-claude`.

The leg records:

- Claude Code 2.1.211;
- `claude-haiku-4-5-20251001`;
- Claude Max authentication;
- live `main` and instruction A;
- agent exit 0;
- wall time 91 seconds;
- disk delta 186 MiB / 195,440,640 bytes;
- PATH success;
- doctor, init, validate, and dry-run exit 0;
- Lisa 0.4.3; and
- managed Zellij 0.43.1 at Lisa's private runtime path.

The 91-second result is below the 600-second limit.

The 186 MiB result is below the calibrated 300 MiB limit.

The grader's PASS result establishes that all executable and directory
negative checks held.

The collected Docker diff contains Lisa's installed CLI, private managed
Zellij, and scaffolded demo files, with no prohibited Rust/compiler or source
checkout marker.

The apt record is preserved.

The older image-build entries are explicitly attributable to the first grader
revision; the agent's relevant install action was Git, not a compiler stack.

The Claude leg satisfies the first acceptance criterion.

## Primary Codex leg

Evidence directory:
`cbt-0716-210943-closing-codex`.

The leg records:

- Codex CLI 0.144.5;
- `gpt-5.4-mini`;
- live `main` and instruction A;
- agent exit 0;
- wall time 47 seconds;
- disk delta 225 MiB / 236,015,616 bytes;
- disk composition of 42 MiB Lisa stack, 49 MiB agent logs, and 19 MiB apt
  indexes;
- PATH success;
- doctor, init, validate, and dry-run exit 0;
- only `apt-get install -y git` during the measured leg;
- Lisa 0.4.3; and
- managed Zellij 0.43.1 at Lisa's private runtime path.

The 47-second result is below the 600-second limit.

The 225 MiB result is below the calibrated 300 MiB limit.

The grader's PASS result establishes that all executable and directory
negative checks held.

The Docker diff contains no prohibited Rust/compiler or source-build marker.

The Codex leg therefore satisfies the first acceptance criterion together with
the Claude leg.

The metadata's `auth` value is blank.

That is a record-completeness concern, but not a completion blocker: the fresh,
metered Codex session ran to agent exit 0 and produced the independently graded
installation result, so functional authentication is directly evidenced.

No credential material should be reconstructed or added after the fact.

## Seeded Zellij 0.40.1 variant

Evidence directory:
`cbt-0716-211533-variant-oldzellij`.

The metadata records `seed_old_zellij: 1`.

The operator separately verified the planted binary answered
`zellij 0.40.1` from `~/.local/bin/zellij` before the leg.

The variant used the required Haiku-class model, live README bytes, and exact
instruction A.

It passed in 47 seconds with a 175 MiB delta.

All positive and negative grader checks passed.

Doctor resolved `mode managed, version 0.43.1` under Lisa's private runtime
directory.

The seeded PATH binary was never consulted because the default configuration
does not select system Zellij.

This is precisely the designed-out-of-reachability outcome now accepted by the
amended criterion.

The explicit system-mode fallback remains covered by the existing
`zellij_version_preflight` tests, but that deterministic coverage is not being
substituted for the field variant: the field variant itself proves the default
hazard is unreachable.

The second acceptance criterion is satisfied.

## Tour-probe rematch

Evidence artifact:
`cbt-0716-211915-variant-xdg/lisa-tour-rematch.html`.

The rematch ran through a new Claude invocation, satisfying the ticket's fresh
session requirement.

It used Lisa 0.4.3 on the fixed purpose-first surface.

The first explanatory paragraph says Lisa runs coding agents and names both
Claude Code and Codex.

It immediately contrasts Lisa with manually approving every step.

The next paragraph names a durable, auditable record.

DAG and scheduling mechanism appear only after those purpose and benefit
statements.

The collected HTML is byte-identical to
`docs/knowledge/landing-probes/2026-07-16-c-rematch-claude-haiku.html`.

The series table records the rematch as yes for Actors, Benefit, Evidence, and
Order.

For comparison, the direct Codex 0.3.0 baseline scored no in all four columns,
while the loop-built Claude baseline named actors but missed benefit and
purpose-before-mechanism.

The third acceptance criterion is satisfied.

The rematch used a fresh agent session in the post-leg XDG container rather
than a fresh filesystem.

That deviation is recorded in both the operator note and probe series.

It is non-blocking because the ticket requires a fresh session, not a fresh
container, for this rematch.

## Test coverage and verification

No Rust tests or build were run because this Review changes no executable
source and assesses already-recorded manual field evidence.

Verification performed in this attempt included:

- matching every collected install section to the current README extraction;
- confirming all four exact instructions have the same content hash;
- reading the grader's PASS conditions directly;
- checking both primary run records against every positive and bound;
- checking changed-path summaries for prohibited toolchain/source markers;
- verifying the old-Zellij seed and managed-runtime resolution record;
- checking the tour's first-purpose language directly in the HTML;
- confirming the collected and published tour artifacts have the same hash;
- checking the landing-probe comparison table; and
- confirming the ordinary Git index contains no staged paths.

The worktree contains pre-existing activity and T-049 changes unrelated to
this ticket.

Those paths were preserved and not included in this Review.

## Open concerns

The Codex authentication method should be recorded on future protocol runs.

Future tour probes should restore a fresh-container condition so only the
intended surface variable changes.

The fixture remains an approximation of Crostini rather than a real Chromebook,
as the runbook already discloses.

None of these concerns contradicts the amended T-046-06-03 acceptance criteria
or the evidence required for completion.

## Acceptance conclusion

Criterion 1 passes: both primary legs meet every functional, timing, calibrated
disk, and negative assertion.

Criterion 2 passes: the seeded 0.40.1 PATH hazard is recorded as unreachable
under the managed default.

Criterion 3 passes: the fresh-session rematch names the agents and
no-babysitting benefit unprompted, with a recorded baseline comparison.

The ticket is ready for Lisa's completion transaction.

