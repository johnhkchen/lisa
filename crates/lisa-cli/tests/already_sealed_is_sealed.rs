//! Replaying a seal that already landed converges on it.
//!
//! Built on the concurrency fixture S-055-01 is named for: four completions
//! dispatched at once, then replayed. Before T-055-01-02 every replay staged
//! nothing and was refused, which is the condition the loop's own earlier
//! success created.

mod support;

use support::{assert_no_guard_collision, dispatch_together, SealFixture};

use lisa_cli::commit_transaction::complete_ticket;

const TICKETS: [&str; 4] = ["T-SEAL-01", "T-SEAL-02", "T-SEAL-03", "T-SEAL-04"];

#[test]
fn replaying_any_of_four_concurrent_seals_converges_on_its_commit() {
    let fixture = SealFixture::new(&TICKETS);

    let sealed = dispatch_together(TICKETS.len(), |index| {
        complete_ticket(fixture.complete_request(TICKETS[index], 1))
    });
    assert_no_guard_collision(&sealed);
    let sealed: Vec<String> = sealed
        .into_iter()
        .enumerate()
        .map(|(index, result)| {
            result
                .unwrap_or_else(|error| panic!("{} did not seal: {error}", TICKETS[index]))
                .commit_id
        })
        .collect();
    let commits_after_seal = fixture.head_commit_count();
    let head_after_seal = fixture.git_string(["rev-parse", "HEAD"]);

    // Identical arguments, dispatched together again: two successes per ticket,
    // one commit per ticket, the second reporting the first's id.
    let replays = dispatch_together(TICKETS.len(), |index| {
        complete_ticket(fixture.complete_request(TICKETS[index], 1))
    });
    assert_no_guard_collision(&replays);

    for (index, replay) in replays.into_iter().enumerate() {
        let replay =
            replay.unwrap_or_else(|error| panic!("{} did not converge: {error}", TICKETS[index]));
        assert_eq!(
            replay.commit_id, sealed[index],
            "{} converged on a different commit",
            TICKETS[index]
        );
        assert!(
            replay.committed_paths.is_empty(),
            "{} committed paths on a convergent replay",
            TICKETS[index]
        );
    }

    // And again under a later generation — the key the loop retries with is
    // not always the key that landed, which is how the field board got stuck.
    let later = dispatch_together(TICKETS.len(), |index| {
        complete_ticket(fixture.complete_request(TICKETS[index], 2))
    });
    assert_no_guard_collision(&later);
    for (index, replay) in later.into_iter().enumerate() {
        let replay =
            replay.unwrap_or_else(|error| panic!("{} did not converge: {error}", TICKETS[index]));
        assert_eq!(
            replay.commit_id, sealed[index],
            "{} converged on a different commit under a later generation",
            TICKETS[index]
        );
        assert!(replay.committed_paths.is_empty());
    }

    assert_eq!(
        fixture.head_commit_count(),
        commits_after_seal,
        "replays must add no commits"
    );
    assert_eq!(fixture.git_string(["rev-parse", "HEAD"]), head_after_seal);

    let subjects = fixture.commit_subjects();
    for ticket in TICKETS {
        let message = fixture.complete_message(ticket);
        assert_eq!(
            subjects.iter().filter(|s| **s == message).count(),
            1,
            "{ticket} is not in HEAD exactly once: {subjects:?}"
        );
    }

    fixture.assert_no_commit_lock();
}

/// The field shape: the seal landed under one key and the loop keeps retrying
/// under another. The retry has nothing to commit and the ticket is sealed.
#[test]
fn a_later_generations_key_converges_on_the_sealed_commit() {
    let ticket = "T-SEAL-FIELD";
    let fixture = SealFixture::new(&[ticket]);

    let sealed = complete_ticket(fixture.complete_request(ticket, 1)).unwrap();
    let commits_after_seal = fixture.head_commit_count();

    let replay = complete_ticket(fixture.complete_request(ticket, 2)).unwrap();

    assert_eq!(replay.commit_id, sealed.commit_id);
    assert!(replay.committed_paths.is_empty());
    assert_eq!(fixture.head_commit_count(), commits_after_seal);
    fixture.assert_no_commit_lock();
}

/// Emptiness is not the evidence. A ticket that was never sealed still fails,
/// and the refusal names the paths the transaction staged from.
#[test]
fn an_unsealed_ticket_with_nothing_to_commit_still_fails() {
    let ticket = "T-SEAL-EMPTY";
    let fixture = SealFixture::new(&[ticket]);

    // Commit the fixture's dirty review artifact by hand and mark the ticket
    // done, so both include paths match HEAD without any completion commit
    // existing for this ticket.
    fixture.write(
        &fixture.ticket_path(ticket),
        &format!(
            "---\nid: {ticket}\ntitle: concurrent seal\ntype: task\nstatus: done\npriority: high\nphase: done\n---\n\nBody\n"
        ),
    );
    fixture.git(["add", "-A"]);
    fixture.git(["commit", "--quiet", "-m", "hand-committed work"]);
    let head_before = fixture.git_string(["rev-parse", "HEAD"]);

    let error = complete_ticket(fixture.complete_request(ticket, 1))
        .unwrap_err()
        .to_string();

    assert!(error.contains("has no changes"), "{error}");
    assert!(error.contains(&fixture.ticket_path(ticket)), "{error}");
    assert!(error.contains(&fixture.work_dir(ticket)), "{error}");
    assert_eq!(fixture.git_string(["rev-parse", "HEAD"]), head_before);
    fixture.assert_no_commit_lock();
}
