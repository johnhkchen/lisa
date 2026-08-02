//! The concurrency fixture S-055-01 is named for.
//!
//! Four commit transactions dispatched at once against one repository must all
//! seal. Before T-055-01-01 the guard was taken with a single non-blocking
//! attempt, so three of these four died on `os error 35`.

mod support;

use support::{assert_no_guard_collision, dispatch_together, SealFixture};

const TICKETS: [&str; 4] = ["T-CONC-01", "T-CONC-02", "T-CONC-03", "T-CONC-04"];

#[test]
fn four_concurrent_seals_all_land_exactly_one_commit_each() {
    let fixture = SealFixture::new(&TICKETS);
    let commits_before = fixture.head_commit_count();

    let results = dispatch_together(TICKETS.len(), |index| {
        lisa_cli::commit_transaction::complete_ticket(fixture.complete_request(TICKETS[index], 1))
    });

    assert_no_guard_collision(&results);
    for (index, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "{} did not seal: {}",
            TICKETS[index],
            result.as_ref().err().unwrap()
        );
    }

    assert_eq!(
        fixture.head_commit_count(),
        commits_before + TICKETS.len(),
        "four transactions must produce exactly four commits"
    );

    let subjects = fixture.commit_subjects();
    for ticket in TICKETS {
        let message = fixture.complete_message(ticket);
        assert_eq!(
            subjects.iter().filter(|s| **s == message).count(),
            1,
            "{ticket} is not in HEAD exactly once: {subjects:?}"
        );

        let sealed = fixture
            .show_at_head(&fixture.ticket_path(ticket))
            .unwrap_or_else(|| panic!("{ticket} is not at HEAD"));
        assert!(sealed.contains("phase: done"), "{ticket}: {sealed}");
        assert!(sealed.contains("status: done"), "{ticket}: {sealed}");

        assert!(
            fixture
                .show_at_head(&format!("{}/review.md", fixture.work_dir(ticket)))
                .is_some(),
            "{ticket} work artifact is not at HEAD"
        );
    }

    fixture.assert_no_commit_lock();
}
