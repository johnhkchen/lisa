# T-021-01 spike harness

Throwaway probes that settle the five empirical unknowns behind the
`codex exec --json` wrapper decision (S-021 / doc 05). **Spike-only — never merged
into `crates/`.** Delete once verdicts are transcribed into `../design.md` and the
wrapper (T-023-01) is built.

Pinned target: **codex `rust-v0.142.5`**.

## Run

```bash
# on a host with codex rust-v0.142.5 installed:
bash run-all.sh          # runs q1..q5, writes evidence under ./out/
# or one probe at a time:
bash q2-json-fidelity.sh
```

Without codex on `PATH`, `run-all.sh` prints a notice and exits 1 (every probe is a
no-op); the verdicts in `../design.md` stay **PROVISIONAL** until a real run.

## Probes → unknowns

| Script | Unknown | Reads pass/fail from |
|---|---|---|
| `q1-env-inheritance.sh` | Does `codex exec`'s child see `LISA_PANE_ID`? | `out/q1/child-saw.txt` |
| `q2-json-fidelity.sh`  | `--json` events complete under tools? exit-code agrees? | `out/q2/anchor-check.txt` |
| `q3-inpane-rendering.sh` | stderr rich vs spinner; partial vs completed text | `out/q3/{stderr-analysis,granularity}.txt` |
| `q4-directory-trust.sh` | Does fresh `CODEX_HOME` block `-a never`? | `out/q4/{A,B,C}/exit-code` |
| `q5-resume-followup.sh` | Does `exec resume` carry context? | `out/q5/recall.txt` |

Each probe writes `codex-version.txt` beside its evidence — a verdict is only valid
for the version recorded there.
