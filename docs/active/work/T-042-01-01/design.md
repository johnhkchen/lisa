# Design: completion effect adapter seam

## Goals

Introduce one typed dispatch boundary for artifact and stopped completion
inputs, use E-041's reducer unchanged, and centralize execution of
`LaunchCompletion` so the host command has one launch site. Preserve all
existing admission, lease, dependency, pending, and result gates.

## Options

Replacing every completion source and `PendingCompletion` with the full E-041
state machine would be architecturally clean, but overlaps successor tickets
for remaining sources, reconciliation, correlation rendering, and persistence.
It is rejected as a broad rewrite.

A free-standing generic module with an injected closure would be easy to unit
test, but scheduler admission and command construction would remain in State.
The closure boundary and State launch boundary would make effect ownership less
clear while adding indirection around private state. It is rejected.

The chosen option is a State-owned typed dispatcher and State-owned exhaustive
effect executor. A small `CompletionInput` enum represents Artifact and Stopped
inputs. `dispatch_completion` validates Review evidence, maps the input to a
typed request event, calls `reduce`, and sends only its returned effect to the
executor. The executor retains the old transaction gates and contains the only
completion `run_command` call.

## Staged migration

Factor disposition admission into a helper shared by the new dispatcher and
temporarily retained legacy Review callers. Artifact polling calls the new
dispatcher. Stopped Review completion calls the new dispatcher. Idle remains on
the legacy Review helper. Manual and ObservedDone remain legacy. Those sources
delegate to the same centralized effect executor but do not claim reducer
coverage until T-042-01-02.

## Identity mapping

Convert `AttemptLease::attempt_id` to `AttemptId` using its decimal string.
Use the ticket ID as the initial `CompletionId`, matching the current one
pending aggregate per ticket. The executor retains the original lease and
checks the effect identities match the lease and ticket before state mutation.
Later idempotency work may strengthen completion identity.

## Reducer state bridge

If `pending_completions` contains the ticket, dispatch supplies
`CompletionState::Requested`; otherwise it supplies `Eligible`. Duplicate
artifact/stop requests therefore receive E-041's `AlreadyPending` rejection.
Full CommandInFlight/Rejected/Confirmed storage is intentionally deferred.
Reducer rejections are logged here; detailed variant rendering is owned by
T-042-01-04.

## Effect execution

Refactor the boolean request body into an executor accepting
`EffectCommand`, ticket, diagnostic source, and authority. Exhaustively match
`LaunchCompletion`, validate its identity, then apply existing pending,
authority, dependency, path, command, and activity behavior. Legacy wrappers
may construct an effect during migration, but no other function performs the
host launch.

## Test strategy

Under `cfg(test)`, State stores executed effects in a vector. The single
executor records the effect before the existing native-test command-config
short circuit. A focused test creates a current leased attempt with a passing
private Review, drives artifact polling, and asserts exactly one typed launch
effect. It then drives stopped completion for the same ticket and asserts the
count remains one. This exercises the real reducer and a stubbed host executor
observable.

Existing tests remain the compatibility suite for disposition admission,
pending state, completion results, and stopped transitions. Production WASM
does not contain the recording vector. No manifest change is required.

## Rejected shortcuts

Calling `reduce` while ignoring its effect would not create an adapter seam.
Constructing effects directly in artifact/stopped callers would preserve two
gateways. Moving launch I/O into lisa-core would violate reducer purity.
Routing every source now would steal successor scope. Testing only the pending
map would not prove one returned effect was executed.
