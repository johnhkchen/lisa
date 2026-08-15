//! Check the `lisa` verbs a Review disposition tells someone to run.
//!
//! A blocking disposition is the one Lisa artifact written for a person who
//! cannot read the source. Every other remedy on a board is addressed to an
//! agent with the code beside it; `remedy_owner: operator` leaves the
//! repository with no compiler behind it, and the failure it produces —
//! `unrecognized subcommand` — reads to its reader as *my install is wrong*
//! rather than *this command was never real*. That is the expensive direction
//! to be wrong in, so the vocabulary is checked while the reviewer is still
//! there to fix it.
//!
//! The vocabulary comes from the running binary's own clap definition, so the
//! answer is not a list anyone maintains by hand: it is what `lisa --help`
//! would print from this very executable. That also makes the check honest
//! about version skew — an agent whose `lisa` predates a subcommand refuses
//! the step, which is exactly what its operator's `lisa` will do.

use std::collections::{BTreeMap, BTreeSet};

/// Words that introduce a command in the middle of a sentence.
///
/// Deliberately short. The scanner's job is to find invocations, not every
/// sentence that says the word "lisa", and a guard that objects to prose is a
/// guard reviewers learn to route around.
const LEAD_IN_WORDS: &[&str] = &[
    "run", "runs", "ran", "running", "rerun", "re-run", "try", "exec", "execute", "then",
];

/// Words that mean the sentence is *about* lisa rather than typing it.
///
/// Only ever consulted for a word this binary does not have as a subcommand, so
/// the list can never hide a real verb — at worst it declines to complain about
/// an invented one that reads like English.
const PROSE_WORDS: &[&str] = &[
    "is", "was", "are", "were", "will", "would", "can", "could", "should", "must", "may", "might",
    "does", "did", "do", "has", "have", "had", "needs", "need", "says", "said", "refuses",
    "refused", "reads", "read", "writes", "wrote", "knows", "gets", "got", "and", "or", "but",
    "that", "this", "it", "its", "itself", "there", "here", "then", "when", "only", "also",
    "still", "never", "always", "already", "on", "in", "at", "to", "from", "by", "with", "for",
    "of", "as", "so", "if", "was", "just", "even", "not",
];

/// Punctuation a command picks up from the prose around it.
const EDGES: &[char] = &[
    '`', '\'', '"', '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '*', '_', '\\',
    '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}',
];

/// Characters that mean the next word starts a fresh command.
const SHELL_LEAD_INS: &[char] = &['`', '\'', '"', '$', '('];

/// The subcommands this binary actually has, and their own subcommands.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    verbs: BTreeMap<String, BTreeSet<String>>,
}

impl Vocabulary {
    /// Read the vocabulary out of a clap command tree.
    ///
    /// Hidden subcommands count: `lisa claim` is not in `--help` and runs
    /// perfectly well, so a step naming it is not an invented verb.
    pub fn from_command(command: &clap::Command) -> Self {
        let mut verbs = BTreeMap::new();
        for subcommand in command.get_subcommands() {
            let mut nested: BTreeSet<String> = subcommand
                .get_subcommands()
                .flat_map(names_of)
                .collect::<BTreeSet<_>>();
            if !nested.is_empty() {
                nested.insert("help".to_string());
            }
            for name in names_of(subcommand) {
                verbs.insert(name, nested.clone());
            }
        }
        // clap builds `help` on demand, so it never appears as a subcommand of
        // the command tree, and `lisa help status` is still a real thing to type.
        verbs.entry("help".to_string()).or_default();
        Self { verbs }
    }

    /// Build a vocabulary from names, for tests and callers without a clap tree.
    #[cfg(test)]
    fn from_names<'a>(names: impl IntoIterator<Item = (&'a str, &'a [&'a str])>) -> Self {
        Self {
            verbs: names
                .into_iter()
                .map(|(verb, nested)| {
                    (
                        verb.to_string(),
                        nested.iter().map(|name| (*name).to_string()).collect(),
                    )
                })
                .collect(),
        }
    }

    fn nested(&self, verb: &str) -> Option<&BTreeSet<String>> {
        self.verbs.get(verb)
    }

    fn closest_verb(&self, candidate: &str) -> Option<String> {
        closest(candidate, self.verbs.keys().map(String::as_str))
    }
}

fn names_of(command: &clap::Command) -> Vec<String> {
    let mut names = vec![command.get_name().to_string()];
    names.extend(command.get_all_aliases().map(str::to_string));
    names
}

/// One `lisa …` invocation this binary cannot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVerb {
    /// Where it was written, as a reviewer would look for it: `step 3`, `check`.
    pub field: String,
    /// The whole string it was written in.
    pub text: String,
    /// The invocation as far as it was understood: `lisa upgrade`.
    pub named: String,
    /// The nearest real verb, when there is one near enough to mean it.
    pub closest: Option<String>,
}

/// Find every `lisa` invocation in one string that this binary cannot run.
pub fn unknown_verbs_in(field: &str, text: &str, vocabulary: &Vocabulary) -> Vec<UnknownVerb> {
    let mut found = Vec::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        for (index, token) in tokens.iter().enumerate() {
            if core_of(token) != "lisa" {
                continue;
            }
            if !starts_a_command(index, token, tokens.get(index.wrapping_sub(1)).copied()) {
                continue;
            }
            let Some(verb) = word_after(&tokens, index) else {
                continue;
            };
            match vocabulary.nested(&verb) {
                None if PROSE_WORDS.contains(&verb.as_str()) => {}
                None => found.push(UnknownVerb {
                    field: field.to_string(),
                    text: text.to_string(),
                    named: format!("lisa {verb}"),
                    closest: vocabulary.closest_verb(&verb),
                }),
                Some(nested) if !nested.is_empty() => {
                    if let Some(action) = word_after(&tokens, index + 1) {
                        if !nested.contains(&action) {
                            found.push(UnknownVerb {
                                field: field.to_string(),
                                text: text.to_string(),
                                named: format!("lisa {verb} {action}"),
                                closest: closest(&action, nested.iter().map(String::as_str))
                                    .map(|suggestion| format!("{verb} {suggestion}")),
                            });
                        }
                    }
                }
                Some(_) => {}
            }
        }
    }
    found
}

/// Check every part of a blocking disposition a person could be handed.
///
/// `steps` and `check` are commands by contract; `ask` and `reason` are prose
/// that routinely quotes one, and the field case put the wrong verb in both.
pub fn unknown_verbs_in_block(
    reason: &str,
    ask: &str,
    steps: Option<&[String]>,
    check: Option<&str>,
    vocabulary: &Vocabulary,
) -> Vec<UnknownVerb> {
    let mut found = unknown_verbs_in("reason", reason, vocabulary);
    found.extend(unknown_verbs_in("ask", ask, vocabulary));
    for (index, step) in steps.unwrap_or(&[]).iter().enumerate() {
        found.extend(unknown_verbs_in(
            &format!("step {}", index + 1),
            step,
            vocabulary,
        ));
    }
    if let Some(check) = check {
        found.extend(unknown_verbs_in("check", check, vocabulary));
    }
    found
}

/// What a reviewer needs to fix an invented verb, while they still can.
///
/// Names the version as well as the verb, because the two ways to be wrong here
/// need opposite fixes: a verb nobody ever built has to go, and a verb newer
/// than the reader's binary has to say which version it needs. The operator hit
/// the second and could not tell it from the first.
pub fn unknown_verb_message(found: &[UnknownVerb], version: &str) -> String {
    let mut message = match found.first() {
        Some(first) => format!(
            "{} names `{}`, and lisa {version} has no {} subcommand.\n",
            first.field,
            first.named,
            first.named.trim_start_matches("lisa ")
        ),
        None => return String::new(),
    };
    for unknown in found {
        message.push_str(&format!(
            "  {}: {}\n",
            unknown.field,
            one_line(&unknown.text)
        ));
        if let Some(closest) = &unknown.closest {
            message.push_str(&format!("  closest: lisa {closest}\n"));
        }
    }
    message.push_str(
        "Fix: name a subcommand this lisa has — `lisa --help` lists them — or, if the verb is real \
         and newer than this binary, say in the step which lisa version it needs and how to get \
         there. If `lisa` here is prose rather than something to type, write it as Lisa.",
    );
    message
}

fn one_line(text: &str) -> String {
    let flattened = text.replace(['\n', '\r'], " ");
    if flattened.chars().count() <= 160 {
        return flattened;
    }
    let head: String = flattened.chars().take(157).collect();
    format!("{head}...")
}

/// Whether this `lisa` is being typed rather than talked about.
fn starts_a_command(index: usize, token: &str, previous: Option<&str>) -> bool {
    if index == 0 {
        return true;
    }
    if token.starts_with(SHELL_LEAD_INS) {
        return true;
    }
    let Some(previous) = previous else {
        return true;
    };
    if previous.ends_with([':', ';', '|', '&'])
        || previous == "$"
        || previous == "("
        || previous.ends_with('(')
    {
        return true;
    }
    LEAD_IN_WORDS.contains(&core_of(previous).as_str())
}

/// The subcommand-shaped word after `tokens[index]`, if there is one.
///
/// A flag, a placeholder like `<board>`, or anything that is not a word you
/// could type as a subcommand means this invocation names no verb to check.
fn word_after(tokens: &[&str], index: usize) -> Option<String> {
    let next = tokens.get(index + 1)?;
    if next.starts_with('-') {
        return None;
    }
    let word = core_of(next);
    subcommand_shaped(&word).then_some(word)
}

fn core_of(token: &str) -> String {
    token.trim_matches(EDGES).to_string()
}

fn subcommand_shaped(word: &str) -> bool {
    !word.is_empty()
        && word.starts_with(|c: char| c.is_ascii_lowercase())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !word.ends_with('-')
}

fn closest<'a>(candidate: &str, known: impl Iterator<Item = &'a str>) -> Option<String> {
    known
        .map(|verb| (distance(candidate, verb), verb))
        .filter(|(distance, verb)| *distance <= 3 && *distance < verb.chars().count())
        .min_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))
        .map(|(_, verb)| verb.to_string())
}

/// Levenshtein distance, for "did you mean" and nothing else.
fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut row: Vec<usize> = (0..=right.len()).collect();
    for (i, left_char) in left.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let next_diagonal = row[j + 1];
            row[j + 1] = if left_char == *right_char {
                diagonal
            } else {
                1 + diagonal.min(row[j]).min(row[j + 1])
            };
            diagonal = next_diagonal;
        }
    }
    row[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use lisa_core::disposition::{parse_review_disposition, ReviewDisposition};
    use std::path::{Path, PathBuf};

    fn vocabulary() -> Vocabulary {
        Vocabulary::from_names([
            ("status", &[][..]),
            ("unblock", &[][..]),
            ("doctor", &[][..]),
            ("nightly", &["install", "run", "status", "uninstall"][..]),
        ])
    }

    fn named(field: &str, text: &str) -> Vec<String> {
        unknown_verbs_in(field, text, &vocabulary())
            .into_iter()
            .map(|unknown| unknown.named)
            .collect()
    }

    /// The field case, exactly as it was written.
    #[test]
    fn the_step_an_operator_was_handed_is_objected_to() {
        let step = "Prove the way back, on the mini: lisa upgrade --tag v0.4.4";
        let found = unknown_verbs_in("step 3", step, &vocabulary());

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].named, "lisa upgrade");
        assert_eq!(found[0].field, "step 3");
        assert_eq!(found[0].text, step);
    }

    #[test]
    fn an_invented_verb_is_found_wherever_a_command_can_start() {
        for text in [
            "lisa upgrade --tag v0.4.4",
            "Then: lisa upgrade --tag v0.4.4",
            "run lisa upgrade",
            "Confirm it: lisa --version, then lisa upgrade",
            "the way back is `lisa upgrade --tag`",
            "cd ~/board && lisa upgrade",
            "(lisa upgrade)",
        ] {
            assert!(
                !named("step 1", text).is_empty(),
                "missed the invented verb in {text:?}"
            );
        }
    }

    #[test]
    fn a_real_verb_passes_including_hidden_and_nested_ones() {
        for text in [
            "lisa status --path ~/board",
            "Run lisa doctor on the mini",
            "lisa nightly status --json",
            "lisa --version",
            "lisa",
            "lisa <subcommand>",
        ] {
            assert_eq!(named("step 1", text), Vec::<String>::new(), "{text}");
        }
    }

    #[test]
    fn a_nested_action_is_checked_against_its_own_subcommands() {
        assert_eq!(
            named("step 1", "lisa nightly frobnicate"),
            ["lisa nightly frobnicate"]
        );
        assert_eq!(named("step 1", "lisa nightly run"), Vec::<String>::new());
    }

    /// Prose is not an invocation. A guard that objects to sentences is a guard
    /// reviewers write around, and then it protects nobody.
    #[test]
    fn prose_about_lisa_is_left_alone() {
        for text in [
            "The lisa binary on that box is stale.",
            "Lisa needs the release published; run: just release.",
            "Two lisas are shadowing each other on the dev desk.",
            "Work artifacts live under .lisa/attempts/T-070-01-03/1/work/.",
            "See docs/knowledge/lisa-workflow.md for the contract.",
            "Built by crates/lisa-cli, tested against lisa-core.",
        ] {
            assert_eq!(named("reason", text), Vec::<String>::new(), "{text}");
        }
    }

    #[test]
    fn the_nearest_real_verb_is_offered_when_there_is_one() {
        let found = unknown_verbs_in("step 1", "lisa statuss", &vocabulary());
        assert_eq!(found[0].closest.as_deref(), Some("status"));

        let found = unknown_verbs_in("step 1", "lisa frobnicate", &vocabulary());
        assert_eq!(found[0].closest, None);
    }

    #[test]
    fn every_part_of_a_block_a_person_reads_is_covered() {
        let steps = vec!["lisa upgrade --tag v0.4.4".to_string()];
        let found = unknown_verbs_in_block(
            "the way back is `lisa upgrade --tag`",
            "Put the mini back on the previous Lisa.",
            Some(&steps),
            Some("lisa nightly status --json"),
            &vocabulary(),
        );

        let fields: Vec<&str> = found.iter().map(|unknown| unknown.field.as_str()).collect();
        assert_eq!(fields, ["reason", "step 1"]);
    }

    #[test]
    fn the_message_names_the_verb_the_version_and_both_fixes() {
        let steps = vec!["Prove the way back, on the mini: lisa upgrade --tag v0.4.4".to_string()];
        let found = unknown_verbs_in_block("reason", "Do it.", Some(&steps), None, &vocabulary());
        let message = unknown_verb_message(&found, "0.5.0-rc.2");

        assert!(message.contains("step 1 names `lisa upgrade`"), "{message}");
        assert!(
            message.contains("lisa 0.5.0-rc.2 has no upgrade subcommand"),
            "{message}"
        );
        assert!(message.contains("lisa --help"), "{message}");
        assert!(message.contains("which lisa version it needs"), "{message}");
        assert!(message.contains("write it as Lisa"), "{message}");
    }

    /// The vocabulary is this binary's own, not a list to keep in step.
    #[test]
    fn the_vocabulary_is_read_off_this_binary() {
        let vocabulary = Vocabulary::from_command(&crate::Cli::command());

        for real in ["status", "loop", "claim", "check-disposition", "help"] {
            assert!(vocabulary.nested(real).is_some(), "{real} is missing");
        }
        assert!(vocabulary.nested("frobnicate").is_none());
        assert!(vocabulary
            .nested("nightly")
            .is_some_and(|nested| nested.contains("status")));
    }

    /// The board is swept by the suite, not once by hand.
    ///
    /// Dispositions already published cannot be fixed by a check that runs at
    /// authoring time, and a step only reaches a person after it is written. So
    /// the ones on the board are re-read every `cargo test`.
    #[test]
    fn no_published_disposition_asks_for_a_verb_this_binary_lacks() {
        let vocabulary = Vocabulary::from_command(&crate::Cli::command());
        let mut complaints = Vec::new();

        for path in published_dispositions() {
            let ReviewDisposition::Block {
                reason,
                ask,
                steps,
                check,
                ..
            } = parse_review_disposition(&path)
            else {
                continue;
            };
            for unknown in unknown_verbs_in_block(
                &reason,
                &ask,
                steps.as_deref(),
                check.as_deref(),
                &vocabulary,
            ) {
                complaints.push(format!(
                    "{}: {} names `{}`",
                    path.display(),
                    unknown.field,
                    unknown.named
                ));
            }
        }

        assert!(
            complaints.is_empty(),
            "published dispositions ask for verbs this lisa does not have:\n{}",
            complaints.join("\n")
        );
    }

    fn published_dispositions() -> Vec<PathBuf> {
        let work = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/active/work");
        let Ok(entries) = std::fs::read_dir(work) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("review-disposition.json"))
            .filter(|path| path.is_file())
            .collect();
        paths.sort();
        paths
    }
}
