# Plan: release-readiness report

## Step 1: capture preflight identity and ownership

From the repository root, record:

- full Git HEAD and one-line description;
- UTC timestamp and timezone;
- Lisa package version;
- Rust and Cargo versions;
- OS, architecture, and Zellij version;
- ordinary worktree status;
- ordinary index paths;
- source diff paths.

Classify `.lisa/provenance.jsonl` and the active ticket frontmatter as
Lisa-managed pre-existing workflow state.

Verification criteria:

- no ticket-owned product/test source delta exists before measurement;
- no ordinary-index entry exists;
- the active ticket is not edited manually;
- required tools are available.

Expected commit boundary: none.

## Step 2: build final release artifacts

Run the canonical repository-root command:

```bash
just build-cli
```

This must build the plugin first, touch the WASM input, and then build the CLI.

After success, record:

- CLI version;
- canonical paths;
- logical byte lengths;
- SHA-256 identities;
- file types.

Verification criteria:

- Just exits zero;
- both outputs exist and are nonempty;
- CLI is executable and reports `0.4.0-rc.6`;
- release WASM is recognized as WebAssembly;
- no size is recorded after a failed build.

Expected commit boundary: none; outputs are ignored generated artifacts.

## Step 3: calculate final size comparison

Run:

```bash
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

Use predecessor before values:

- CLI: 3,013,904 bytes;
- WASM: 1,414,183 bytes.

Calculate for each artifact:

- `delta_bytes = after - before`;
- `delta_percent = delta_bytes / before * 100`.

Compare after fingerprints to T-038-04-01's freshly dogfooded identities.

Verification criteria:

- command output matches direct file metadata;
- CLI/WASM hashes match the dogfood artifact identities or any mismatch is
  investigated before proceeding;
- arithmetic is independently checked;
- size is not treated as runtime memory.

Expected commit boundary: none.

## Step 4: run after planning-startup batch 1

Use the exact predecessor benchmark command:

```bash
ruby -e 'cmd=["target/release/lisa","loop","--dry-run","--path","."]; 3.times { abort "warmup failed" unless system(*cmd, out: File::NULL, err: File::NULL) }; xs=30.times.map { t=Process.clock_gettime(Process::CLOCK_MONOTONIC); abort "sample failed" unless system(*cmd, out: File::NULL, err: File::NULL); (Process.clock_gettime(Process::CLOCK_MONOTONIC)-t)*1000 }; s=xs.sort; median=(s[14]+s[15])/2.0; puts "raw_ms=#{xs.map { |x| format("%.3f",x) }.join(",")}"; puts format("min_ms=%.3f\nmedian_ms=%.3f\nmean_ms=%.3f\nmax_ms=%.3f",s.first,median,xs.sum/xs.length,s.last)'
```

Record raw values plus min, median, mean, and max.

Verification criteria:

- all three warmups succeed;
- all 30 samples succeed;
- primary after median is calculated from sorted positions 15 and 16;
- child stdout/stderr is excluded consistently with the baseline;
- no provider or Zellij launch is implied.

Expected commit boundary: none.

## Step 5: run independent after planning-startup batch 2

Invoke the identical command a second time without changing tracked inputs.

Calculate:

- absolute median difference between after batches;
- relative difference from after batch 1;
- pass/fail against the predeclared ±20% same-host tolerance.

Then compare primary before and after medians:

- before: 2.707 ms;
- after: batch 1 median;
- absolute and percentage delta.

Verification criteria:

- second batch has 30 successful samples after three warmups;
- tolerance result is explicit;
- if tolerance fails, report failure without selecting a more favorable batch;
- active ticket/DAG input drift is called out.

Expected commit boundary: none.

## Step 6: run final-tree host-RSS observation

First syntax-check the retained helper:

```bash
bash -n .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
```

Then run exactly:

```bash
bash .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
```

Capture complete output.

The helper should:

- create an external disposable fixture;
- initialize it with the current release CLI;
- use one uniquely named Zellij session;
- resolve exactly one Zellij server PID;
- hold idle for ten one-second RSS samples;
- activate one deterministic stub-backed ticket;
- keep the same server PID;
- hold active for ten one-second RSS samples;
- emit `measurement_complete=PASS`;
- tear down the named session and fixture.

Verification criteria:

- helper syntax and execution exit zero;
- both states contain exactly ten numeric KiB values;
- idle state precedes any stub launch;
- active launch receipt precedes active samples;
- final server identity uses the same PID;
- output identifies current final artifacts;
- no authenticated provider or model work runs;
- unique session is absent afterward.

Expected commit boundary: none.

## Step 7: recompute footprint summaries

Independently sort the recorded idle and active values and calculate:

- count;
- minimum;
- median;
- maximum;
- active median minus idle median.

Compare with predecessor before observations:

- idle median 81,416 KiB, range 81,408–81,424;
- active median 81,568 KiB, range 81,552–81,568;
- paired host-state median difference +152 KiB.

Calculate before/after median deltas only as host-process observation deltas.

Verification criteria:

- all arithmetic recomputes from raw samples;
- no threshold is imposed;
- every result is labeled “Zellij host-process RSS — not Lisa plugin-heap
  attribution”;
- OS residency variability is stated.

Expected commit boundary: none.

## Step 8: reconcile final gate and dogfood evidence

Read back the post-cleanup and dogfood reviews.

Record these final outcomes without rerunning them absent a source delta:

- fmt pass;
- native Clippy pass with warnings denied;
- WASM Clippy pass with warnings denied;
- `just check` pass;
- workspace tests: 725 passed, zero failed, one ignored;
- ignored real-Zellij test: explicitly passed;
- atomic provider contract fixture: PASS, 1.31 seconds wall;
- real-Zellij delivery fixture: PASS, 125.50 seconds wall;
- all four named delivery scenarios passed.

Verification criteria:

- each result cites the exact predecessor command;
- fixture duration is not presented as startup latency;
- source commit ordering confirms no product change after dogfood;
- deterministic local scope is explicit.

Expected commit boundary: none.

## Step 9: reconcile cleanups and named retained repetition

Record the four landed candidates C-01 through C-04 and their three source
paths.

Record C-05 through C-14 individually with the predecessor-approved rationale.

Verification criteria:

- all identifiers C-05 through C-14 appear exactly once in the retained list;
- no retained family is represented as completed;
- C-10 and C-14 are described as intentional evidence retention;
- no new cleanup is performed.

Expected commit boundary: none.

## Step 10: write `progress.md` as the single report

Construct the report using the Structure section order.

Required content checks:

- one scoped readiness verdict;
- before/after table for all requested measurement types;
- exact units and deltas;
- exact reproduction command for every measurement;
- raw after timing and RSS evidence;
- timing and footprint caveats adjacent to results;
- clean-gate status;
- deterministic dogfood status;
- all retained repetition C-05 through C-14;
- residual risks and blocking assessment;
- source/transaction integrity.

Search for ambiguous claims such as:

- “plugin memory”;
- “launch time” applied to dry-run planning startup;
- “end-to-end” without the deterministic fixture qualifier;
- “all tests” without identifying the ignored test;
- “reproducible” without host/toolchain/input context.

Correct any ambiguity before Review.

Expected commit boundary: none.

## Step 11: final repository audit

Run:

```bash
git status --porcelain=v1 --untracked-files=all
git diff --cached --name-only
git diff --name-only
```

Confirm:

- no ticket-owned source file is staged;
- no ticket-owned source file is modified;
- no ticket-owned source file is untracked;
- only Lisa-managed workflow paths differ;
- generated target outputs remain ignored;
- no ordinary Git staging or commit was used.

If there is no source unit, record zero `lisa commit-ticket` transactions.

If an unexpected owned source unit exists, stop report finalization until it is
committed with an exact include path and the residue is clear.

## Step 12: write Review

Read back:

- the ticket criterion;
- all five earlier phase artifacts;
- the completed `progress.md` report;
- final repository status.

Write `review.md` summarizing:

- acceptance outcome;
- principal before/after values;
- change/artifact scope;
- measurement validation;
- gate and fixture coverage;
- source commit status;
- open concerns, limitations, and critical issues;
- Lisa-owned completion boundary.

Verification criteria:

- all six private artifacts exist;
- Review does not become a conflicting second report;
- every acceptance item maps to the authoritative report;
- ticket-owned source residue is absent;
- phase/status were not manually edited.

## Step 13: stop on this ticket

After `review.md` exists:

- remain on `T-038-04-02`;
- do not update frontmatter;
- do not publish shared work directly;
- do not run Lisa's completion command;
- do not start another ticket;
- wait for Lisa to verify the lease, admit Review, create the completion commit,
  publish Done, and release the seat.
