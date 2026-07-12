# Progress — T-037-01-01

## Step 1 — Adapter capability (`adapter.rs`)
- [x] `ReadinessMode` enum
- [x] trait method `readiness_mode`
- [x] Claude impl → SessionStart
- [x] Codex impl → Grace
- [x] adapter tests (both green)
- [x] commit unit 1 — 98a5abec

## Step 2 — Scheduler read at dispatch (`lib.rs`)
- [x] import ReadinessMode
- [x] `seat_readiness` field
- [x] `seat_readiness_mode` accessor (`#[cfg_attr(not(test), allow(dead_code))]`)
- [x] record at primary dispatch (gated on `fresh_launch`)
- [x] record at recovery relaunch
- [x] scheduler test (green)
- [x] commit unit 2 — 9de83016

## Verification
- `cargo test -p lisa-plugin readiness` → 5 passed
- `cargo test --workspace` → 286 passed, 0 failed
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` → clean
- `cargo clippy -p lisa-plugin` → no warnings
- `git status` for both source files → clean (committed via lisa commit-ticket)

## Deviations
(none)
