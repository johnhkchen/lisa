//! `lisa spend` — what the desk has spent, read straight off its own captures.
//!
//! **The bare command is the reading half of `S-072-01`.** It reads
//! `.lisa/<client>/captures.jsonl` on this machine and on every other machine
//! `rail desk --hosts --json` names, sums what it finds by model and by
//! machine, over the last day and the last week, and says so. It decides
//! nothing and writes nothing.
//!
//! **`--guard` is `T-072-01-02`: what to do about the number, on this board
//! alone.** It reuses the same reading, compares it against a
//! `[scheduling].weekly_token_allowance` the operator configured, and —
//! only when this board is also marked `[scheduling].priority = "low"` —
//! stops this board's own loop and tells the operator through
//! `rail tell --kind loop-degraded`. See
//! `docs/knowledge/spend-autonomy.md` for the chosen position and why.
//!
//! ## Why raw totals, not a fraction of an allowance
//!
//! A published API for "how much of your week is left" does not exist — that
//! number came from an operator reading a screen. Inventing a price per model
//! or a weekly ceiling here would be a guess dressed as a fact, and the whole
//! point of this command is to let a person calibrate a real number against
//! what they see, not hand them a second number to distrust. So this reports
//! token counts, broken down by model (because the models are not the same
//! price) and by machine (because the desk is not one machine), and says
//! plainly that a transcript's tokens are not a provider's own accounting.
//!
//! ## Why a machine that cannot be asked is never zero
//!
//! An unreachable machine has almost certainly been spending; folding it into
//! the total as if it were silent would be the more dangerous kind of wrong,
//! because it looks like an answer. So [`aggregate`] keeps unreachable hosts
//! out of every sum and [`render`] names each one and why, right next to the
//! numbers it could gather.
//!
//! ## Why one `echo … ; cat …`, not a remote `lisa`
//!
//! A host's `reach` is a plain argv prefix — `rail desk --hosts` documents it
//! as "split it on whitespace and put it in front of the command you wanted to
//! run," never a shell line of its own. What follows it here is still passed
//! through the far side's shell once ssh joins the whole argv with spaces, so
//! `echo MARKER; cat f1 f2 …` runs as one line without this process building a
//! shell string by hand. The marker is what tells a host that answered
//! anything at all — even a project with no captures files yet — apart from a
//! host nothing came back from, which is the whole of what "unreachable" means
//! here. This spends exactly one process per remote machine, whatever its
//! project count, per the ticket's own budget: "a file read plus one reach per
//! remote machine."

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use lisa_core::capture::CaptureRecord;
use lisa_core::provenance::system_time_to_epoch;

/// The two client directories a capturing hook writes under `.lisa/`.
const CLIENT_DIRS: [&str; 2] = ["claude", "codex"];

const DAY_SECS: u64 = 24 * 60 * 60;
const WEEK_SECS: u64 = 7 * DAY_SECS;

/// The label a record with no recorded model is shown under — never a guess
/// at which model actually spent those tokens.
const UNKNOWN_MODEL: &str = "unknown (model not recorded)";

/// Printed as the first line of a remote reach's stdout, ahead of whatever
/// `cat` found. Its presence is what "reached" means; its absence — a
/// connection refused, a name that does not resolve, a host that never
/// answers — is what "unreachable" means. Deliberately not the exit code:
/// `cat` exits non-zero on a project with no captures yet, and that is not
/// the same fact as the host being unreachable.
const REACHED_MARKER: &str = "LISA-SPEND-REACHED";

/// How long a remote reach is given before it is abandoned. Matches `rail
/// desk`'s own default bound for asking a host something, so an operator
/// reading both does not learn two different meanings of "a while."
const DEFAULT_REACH_TIMEOUT_SECS: u64 = 6;

/// One machine on the desk, as `rail desk --hosts --json` named it, or the
/// single-project fallback used when rail cannot say.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeskHost {
    name: String,
    /// Argv prefix that reaches this host; empty means local.
    reach: String,
    /// Project roots on that host, already usable as a path on it (rail's
    /// `used`, or the one path this process was asked about).
    projects: Vec<String>,
}

impl DeskHost {
    fn is_local(&self) -> bool {
        self.reach.is_empty()
    }

    /// A short label for this host suitable for a row of output: `here` or
    /// `mini (ssh mini)`.
    fn display_label(&self) -> String {
        if self.is_local() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.reach)
        }
    }
}

/// Running totals for one bucket (a model, a machine, or the grand total).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub turns: u64,
}

impl Totals {
    fn add(&mut self, record: &CaptureRecord) {
        self.input_tokens += record.input_tokens;
        self.output_tokens += record.output_tokens;
        self.turns += 1;
    }
}

/// One window's worth of spend (a day, or a week), broken down two ways.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WindowReport {
    pub by_model: BTreeMap<String, Totals>,
    pub by_machine: BTreeMap<String, Totals>,
    pub total: Totals,
}

impl WindowReport {
    fn add(&mut self, machine: &str, record: &CaptureRecord) {
        let model = record
            .model
            .clone()
            .unwrap_or_else(|| UNKNOWN_MODEL.to_string());
        self.by_model.entry(model).or_default().add(record);
        self.by_machine
            .entry(machine.to_string())
            .or_default()
            .add(record);
        self.total.add(record);
    }
}

/// The complete answer: two windows, and which machines could and could not
/// be read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpendReport {
    pub day: WindowReport,
    pub week: WindowReport,
    /// Machines whose captures were read, in the order they were asked.
    pub reached: Vec<String>,
    /// Machines that could not be asked, with why, in the order they were
    /// asked. Never folded into `day`/`week` as zero.
    pub unreachable: Vec<(String, String)>,
}

/// Sum a capture record into every window it falls inside, keyed by machine.
///
/// `now` and each record's `captured_at` are both Unix seconds; a record
/// newer than `now` (clock skew) is treated as age zero rather than
/// discarded, since it is certainly within both windows.
pub fn aggregate(
    now: u64,
    per_host: &[(String, Result<Vec<CaptureRecord>, String>)],
) -> SpendReport {
    let mut report = SpendReport::default();
    for (host, result) in per_host {
        match result {
            Ok(records) => {
                report.reached.push(host.clone());
                for record in records {
                    let age = now.saturating_sub(record.captured_at);
                    if age <= WEEK_SECS {
                        report.week.add(host, record);
                        if age <= DAY_SECS {
                            report.day.add(host, record);
                        }
                    }
                }
            }
            Err(reason) => report.unreachable.push((host.clone(), reason.clone())),
        }
    }
    report
}

fn render_totals(t: &Totals) -> String {
    format!(
        "{} in / {} out ({} turn{})",
        t.input_tokens,
        t.output_tokens,
        t.turns,
        if t.turns == 1 { "" } else { "s" }
    )
}

fn render_window(label: &str, window: &WindowReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("{label}\n"));
    if window.total.turns == 0 {
        out.push_str("  nothing captured in this window\n");
        return out;
    }
    out.push_str("  by model\n");
    for (model, totals) in &window.by_model {
        out.push_str(&format!("    {model}: {}\n", render_totals(totals)));
    }
    out.push_str("  by machine\n");
    for (machine, totals) in &window.by_machine {
        out.push_str(&format!("    {machine}: {}\n", render_totals(totals)));
    }
    out.push_str(&format!("  total: {}\n", render_totals(&window.total)));
    out
}

/// Turn a computed [`SpendReport`] into the text `lisa spend` prints.
///
/// `discovery_note` carries a caveat about how the desk's machine list was
/// found — set when `rail` could not be asked, so the report never implies a
/// desk-wide answer it did not actually gather.
pub fn render(report: &SpendReport, discovery_note: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("Desk spend — read from session transcripts, not from a provider balance.\n\n");

    if let Some(note) = discovery_note {
        out.push_str(&format!("Note: {note}\n\n"));
    }

    out.push_str("Machines\n");
    for host in &report.reached {
        out.push_str(&format!("  {host}: ok\n"));
    }
    for (host, reason) in &report.unreachable {
        out.push_str(&format!("  {host}: unreachable — {reason}\n"));
    }
    out.push('\n');

    out.push_str(&render_window("Last 24h (day)", &report.day));
    out.push('\n');
    out.push_str(&render_window("Last 7d (week)", &report.week));

    if !report.unreachable.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "{} machine{} could not be reached and {} not counted above. \
             An unreachable machine has almost certainly been spending — treat it as unknown, not zero.\n",
            report.unreachable.len(),
            if report.unreachable.len() == 1 { "" } else { "s" },
            if report.unreachable.len() == 1 { "is" } else { "are" },
        ));
    }

    let unknown_model_note = report
        .week
        .by_model
        .get(UNKNOWN_MODEL)
        .filter(|t| t.turns > 0)
        .map(|t| {
            format!(
                "{} of the week's turns ({}) have no recorded model — from before per-turn \
                 model attribution landed, or a transcript that did not say. They are counted \
                 in the totals above under \"{UNKNOWN_MODEL}\", not silently added to any real \
                 model's total.\n",
                t.turns,
                render_totals(t)
            )
        });
    if let Some(note) = unknown_model_note {
        out.push('\n');
        out.push_str(&note);
    }

    out.push('\n');
    out.push_str(
        "These are token counts read from session transcripts on disk, not your provider's \
         own accounting of a weekly allowance. They will not match exactly — use this to \
         calibrate against what you see on screen, not as an authoritative balance.\n",
    );

    out
}

/// Parse whatever complete `CaptureRecord` lines are in `text`, skipping a
/// blank or unparseable line rather than failing the whole read: drift in one
/// row must not hide every other row on the same machine.
fn parse_capture_lines(text: &str) -> Vec<CaptureRecord> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<CaptureRecord>(line).ok())
        .collect()
}

fn read_local_captures(host: &DeskHost) -> Vec<CaptureRecord> {
    let mut records = Vec::new();
    for project in &host.projects {
        for client in CLIENT_DIRS {
            let path = Path::new(project)
                .join(".lisa")
                .join(client)
                .join("captures.jsonl");
            if let Ok(text) = std::fs::read_to_string(&path) {
                records.extend(parse_capture_lines(&text));
            }
        }
    }
    records
}

/// What one reach process said, before it is read for a marker.
struct Ran {
    stdout: String,
    stderr: String,
    timed_out: bool,
}

/// Run one argv, draining its pipes on their own threads so a document larger
/// than the pipe buffer cannot deadlock the wait, and give up after `timeout`
/// rather than hang on a host that never answers.
///
/// On timeout the reader threads are abandoned, not joined. `kill` reaches
/// only the process this function started — not a descendant it spawned, an
/// `ssh` `ControlMaster`, a `sleep` a fixture forked — and such a descendant
/// can hold the pipe's write end open long after the process we started is
/// gone. Joining would mean waiting on *that*, for as long as it runs, which
/// defeats the timeout it is standing in for. Whatever those threads
/// eventually read is simply discarded.
fn run_reach(argv: &[String], timeout: Duration) -> Result<Ran, String> {
    let (program, rest) = argv
        .split_first()
        .ok_or_else(|| "empty reach prefix".to_string())?;
    let mut child = Command::new(program)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start '{program}': {e}"))?;

    let mut stdout_pipe = child.stdout.take().expect("stdout was piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr was piped");
    let reading = std::thread::spawn(move || {
        let mut body = String::new();
        let _ = stdout_pipe.read_to_string(&mut body);
        body
    });
    let complaining = std::thread::spawn(move || {
        let mut body = String::new();
        let _ = stderr_pipe.read_to_string(&mut body);
        body
    });

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return Ok(Ran {
                    stdout: reading.join().unwrap_or_default(),
                    stderr: complaining.join().unwrap_or_default(),
                    timed_out: false,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(Ran {
                        stdout: String::new(),
                        stderr: String::new(),
                        timed_out: true,
                    });
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                return Ok(Ran {
                    stdout: reading.join().unwrap_or_default(),
                    stderr: complaining.join().unwrap_or_default(),
                    timed_out: false,
                });
            }
        }
    }
}

/// Read one remote host's captures over its `reach`, spending exactly one
/// process no matter how many projects it names.
fn read_remote_captures(host: &DeskHost, timeout: Duration) -> Result<Vec<CaptureRecord>, String> {
    let mut argv: Vec<String> = host.reach.split_whitespace().map(str::to_string).collect();
    argv.push("echo".to_string());
    argv.push(format!("{REACHED_MARKER};"));
    argv.push("cat".to_string());
    for project in &host.projects {
        let project = project.trim_end_matches('/');
        for client in CLIENT_DIRS {
            argv.push(format!("{project}/.lisa/{client}/captures.jsonl"));
        }
    }

    let ran = run_reach(&argv, timeout)?;
    let mut lines = ran.stdout.splitn(2, '\n');
    let first = lines.next().unwrap_or("");
    if first.trim() == REACHED_MARKER {
        return Ok(parse_capture_lines(lines.next().unwrap_or("")));
    }

    if ran.timed_out {
        return Err(format!("timed out after {}s", timeout.as_secs()));
    }
    let reason = ran
        .stderr
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("no response");
    let reason: String = reason.chars().take(200).collect();
    Err(reason)
}

/// Parse the subset of `rail desk --hosts --json` this command needs. Any
/// unexpected shape — a field missing, `ok: false`, unparseable JSON — is
/// treated the same as `rail` not being available at all: the caller falls
/// back to the one project it was asked about, rather than print a partial or
/// guessed desk.
fn parse_rail_hosts(stdout: &[u8]) -> Option<Vec<DeskHost>> {
    let value: serde_json::Value = serde_json::from_slice(stdout).ok()?;
    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }
    let hosts = value.pointer("/data/hosts")?.as_array()?;
    let mut out = Vec::with_capacity(hosts.len());
    for host in hosts {
        let name = host.get("name")?.as_str()?.to_string();
        let reach = host
            .get("reach")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let projects = host
            .get("used")
            .or_else(|| host.get("projects"))
            .and_then(serde_json::Value::as_array)?
            .iter()
            .filter_map(|p| p.as_str().map(str::to_string))
            .collect();
        out.push(DeskHost {
            name,
            reach,
            projects,
        });
    }
    Some(out)
}

fn run_rail_desk_hosts() -> Option<Vec<DeskHost>> {
    let output = Command::new("rail")
        .args(["desk", "--hosts", "--json"])
        .output()
        .ok()?;
    parse_rail_hosts(&output.stdout)
}

/// Which machines to read, and the caveat to show if that list is narrower
/// than the whole desk.
fn discover_hosts(fallback_path: &Path) -> (Vec<DeskHost>, Option<String>) {
    if let Some(hosts) = run_rail_desk_hosts().filter(|hosts| !hosts.is_empty()) {
        return (hosts, None);
    }
    (
        vec![DeskHost {
            name: "here".to_string(),
            reach: String::new(),
            projects: vec![fallback_path.display().to_string()],
        }],
        Some(
            "`rail` was not available, or `rail desk --hosts --json` did not answer — showing \
             only this project's local captures, not the whole desk."
                .to_string(),
        ),
    )
}

/// Read every reachable machine's captures, once, for both `run_spend` and
/// `run_guard`. `path` is the project this process was asked about; it is
/// only used when the desk-wide machine list could not be learned, as the one
/// project to fall back to.
fn compute_report(path: &Path, reach_timeout_secs: u64) -> (SpendReport, Option<String>) {
    let (hosts, discovery_note) = discover_hosts(path);
    let now = system_time_to_epoch(SystemTime::now());
    let timeout = Duration::from_secs(reach_timeout_secs);

    let per_host: Vec<(String, Result<Vec<CaptureRecord>, String>)> = hosts
        .iter()
        .map(|host| {
            let result = if host.is_local() {
                Ok(read_local_captures(host))
            } else {
                read_remote_captures(host, timeout)
            };
            (host.display_label(), result)
        })
        .collect();

    (aggregate(now, &per_host), discovery_note)
}

/// Read every reachable machine's captures and render the answer.
pub fn run_spend(path: &Path, reach_timeout_secs: u64) -> String {
    let (report, discovery_note) = compute_report(path, reach_timeout_secs);
    render(&report, discovery_note.as_deref())
}

/// The default reach timeout, exposed for the CLI's `--reach-timeout-secs`.
pub const DEFAULT_REACH_TIMEOUT: u64 = DEFAULT_REACH_TIMEOUT_SECS;

/// `lisa spend --guard` — running low is a decision, not a crash (T-072-01-02).
///
/// **The position, chosen in [`docs/knowledge/spend-autonomy.md`]: it stops,
/// but never starts or changes.** Running low ends this board's own loop when
/// it opted itself in; it never swaps a model, never changes effort, and
/// never touches a board it did not ask to stop. Stopping is the reversible
/// direction — a stopped loop restarts with `lisa loop` — and it cannot
/// produce the kind of surprising result a silent model swap can.
///
/// Fraction of the configured allowance a board must have spent, this week,
/// before the guard will consider stopping its own loop. Not a rate limiter —
/// spending 80% on Monday is legal (`S-072-01`) — this is the point past
/// which "keep going and hope" stops being a decision and starts being an
/// accident.
const LOW_SPEND_STOP_PCT: u64 = 90;

/// What `lisa spend --guard` decided, and why. Every variant renders to a
/// sentence an operator reads without opening the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardAction {
    /// No `[scheduling].weekly_token_allowance` is configured. There is
    /// nothing to compare the spend against, so nothing acts — an
    /// unconfigured number is not a low reading.
    NoAllowanceConfigured,
    /// One or more hosts could not be reached this pass. A desk-wide total
    /// with a hole in it is not a low reading either, so the guard refuses to
    /// act on it (T-072-01-02 AC: "nothing acts on a number it could not
    /// read").
    UnreadableSpend { unreachable: Vec<String> },
    /// Spend is under the threshold. Frontloading is legal; this is
    /// knowledge, not a rule to pace against.
    BelowThreshold { spent: u64, allowance: u64 },
    /// Spend is over the threshold, but this board never marked itself low.
    /// The default priority (medium) protects a board that never configured
    /// one — only a board explicitly marked low is ever a stop target
    /// (T-072-01-02 AC).
    NotEligible {
        spent: u64,
        allowance: u64,
        priority: lisa_core::types::Priority,
    },
    /// Spend is over the threshold and this board is low priority: stop it.
    Stop { spent: u64, allowance: u64 },
}

fn pct_spent(spent: u64, allowance: u64) -> u64 {
    spent.saturating_mul(100) / allowance.max(1)
}

fn priority_word(priority: lisa_core::types::Priority) -> &'static str {
    match priority {
        lisa_core::types::Priority::Critical => "critical",
        lisa_core::types::Priority::High => "high",
        lisa_core::types::Priority::Medium => "medium",
        lisa_core::types::Priority::Low => "low",
    }
}

/// The guard's whole decision, as a pure function of what it read: the
/// desk-wide spend, the configured allowance, and this board's own priority.
/// Nothing here touches a scheduler or shells to `rail` — that half is
/// [`stop_for_guard`], reached only when this returns [`GuardAction::Stop`].
pub fn decide_guard_action(
    report: &SpendReport,
    allowance: Option<u64>,
    priority: lisa_core::types::Priority,
) -> GuardAction {
    let Some(allowance) = allowance else {
        return GuardAction::NoAllowanceConfigured;
    };
    if !report.unreachable.is_empty() {
        return GuardAction::UnreadableSpend {
            unreachable: report.unreachable.iter().map(|(h, _)| h.clone()).collect(),
        };
    }
    let spent = report.week.total.input_tokens + report.week.total.output_tokens;
    if spent * 100 < allowance * LOW_SPEND_STOP_PCT {
        return GuardAction::BelowThreshold { spent, allowance };
    }
    if priority != lisa_core::types::Priority::Low {
        return GuardAction::NotEligible {
            spent,
            allowance,
            priority,
        };
    }
    GuardAction::Stop { spent, allowance }
}

/// Render a [`GuardAction`] as the text `lisa spend --guard` prints under the
/// ordinary spend report.
pub fn render_guard_action(action: &GuardAction) -> String {
    match action {
        GuardAction::NoAllowanceConfigured => {
            "Guard: [scheduling].weekly_token_allowance is not set, so there is nothing to \
             compare this week's spend against. Not acting.\n"
                .to_string()
        }
        GuardAction::UnreadableSpend { unreachable } => format!(
            "Guard: {} machine{} could not be reached this pass, so this week's total has a \
             hole in it. Not acting on a reading it could not fully take.\n",
            unreachable.len(),
            if unreachable.len() == 1 { "" } else { "s" },
        ),
        GuardAction::BelowThreshold { spent, allowance } => format!(
            "Guard: {spent} of {allowance} tokens spent this week ({}%), under the \
             {LOW_SPEND_STOP_PCT}% mark. Nothing to do — spending early is not an error.\n",
            pct_spent(*spent, *allowance),
        ),
        GuardAction::NotEligible {
            spent,
            allowance,
            priority,
        } => format!(
            "Guard: {spent} of {allowance} tokens spent this week ({}%), over the \
             {LOW_SPEND_STOP_PCT}% mark, but this board's [scheduling].priority is {} — only a \
             board marked low stops itself. Nothing to do.\n",
            pct_spent(*spent, *allowance),
            priority_word(*priority),
        ),
        GuardAction::Stop { spent, allowance } => format!(
            "Guard: {spent} of {allowance} tokens spent this week ({}%), over the \
             {LOW_SPEND_STOP_PCT}% mark, and this board is low priority. Stopping its loop.\n",
            pct_spent(*spent, *allowance),
        ),
    }
}

/// Read every reachable machine's captures, decide what the guard should do
/// about it, and render both. Returns the decision alongside the text so the
/// caller can act on [`GuardAction::Stop`] with [`stop_for_guard`] — reading
/// and deciding are pure; only the caller's next step touches the outside
/// world.
pub fn run_guard(path: &Path, reach_timeout_secs: u64) -> (String, GuardAction) {
    let (report, discovery_note) = compute_report(path, reach_timeout_secs);

    let resolved = match crate::config::load_config(path) {
        Ok(validation) => crate::config::resolve_config(&validation.config, None, None),
        Err(_) => crate::config::ResolvedConfig::default(),
    };
    let action = decide_guard_action(&report, resolved.weekly_token_allowance, resolved.priority);

    let mut text = render(&report, discovery_note.as_deref());
    text.push('\n');
    text.push_str(&render_guard_action(&action));
    (text, action)
}

/// Carries out a [`GuardAction::Stop`]: ends every live scheduler on this
/// board through the same path `lisa schedulers --stop` uses, so a stopped
/// run is exactly as restartable and exactly as describable by
/// `lisa schedulers` afterward as one an operator stopped by hand
/// (T-072-01-02 AC: "ended cleanly, not killed"). Then tells the operator
/// through `rail tell --kind loop-degraded`, the closest existing fact to a
/// loop that stopped itself.
///
/// `rail` is best-effort: a stop that already happened must still say so
/// somewhere the operator will see it even when `rail` is absent or refuses,
/// so its failure falls back to stdout rather than turning a successful stop
/// into an `Err`.
pub fn stop_for_guard(root: &Path, spent: u64, allowance: u64) -> Result<(), String> {
    // Every recorded scheduler, not a pre-filtered "live" subset: `stop_one`
    // (inside `run_schedulers`) already tells a genuinely running one from a
    // stale record and cleans the stale one up rather than re-killing it, so
    // a second filter here would only disagree with it about the same
    // question and cost a second presence probe to do it.
    let roster = lisa_core::schedulers::read_roster(root);
    let ids: Vec<String> = roster
        .iter()
        .map(|record| record.scheduler_id.clone())
        .collect();

    if ids.is_empty() {
        println!("No scheduler has recorded itself here; nothing to stop.");
    } else {
        for id in &ids {
            crate::schedulers::run_schedulers(root, Some(id))?;
        }
    }

    let what = format!(
        "spent {spent} of its {allowance}-token weekly allowance ({}%) and is a low-priority \
         board",
        pct_spent(spent, allowance),
    );
    let do_ = if ids.is_empty() {
        "Lisa found nothing running to stop; restart with `lisa loop` once the allowance \
         resets or is raised."
            .to_string()
    } else {
        format!(
            "Lisa stopped {} recorded scheduler{} here; restart with `lisa loop` once the \
             allowance resets or is raised.",
            ids.len(),
            if ids.len() == 1 { "" } else { "s" },
        )
    };
    tell_rail(root, &what, &do_);
    Ok(())
}

/// `rail tell --kind loop-degraded`, or a stdout line naming the same fact
/// when `rail` cannot carry it — a stop that happened must never read as a
/// stop that went unreported.
fn tell_rail(root: &Path, what: &str, do_: &str) {
    let result = Command::new("rail")
        .args([
            "tell",
            "--kind",
            "loop-degraded",
            "--project",
            &root.display().to_string(),
            "--what",
            what,
            "--do",
            do_,
        ])
        .output();
    match result {
        Ok(output) if output.status.success() => {
            println!("Told rail (loop-degraded): {what}");
        }
        _ => {
            println!("Could not tell rail; saying so here instead: {what} — {do_}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(captured_at: u64, model: Option<&str>, input: u64, output: u64) -> CaptureRecord {
        CaptureRecord {
            pane_id: 0,
            session_id: "s".to_string(),
            captured_at,
            input_tokens: input,
            output_tokens: output,
            client: "claude".to_string(),
            model: model.map(str::to_string),
            effort: None,
        }
    }

    const NOW: u64 = 1_000_000;

    #[test]
    fn aggregate_buckets_by_model_and_by_machine_within_the_week() {
        let per_host = vec![
            (
                "here".to_string(),
                Ok(vec![
                    record(NOW - 10, Some("claude-opus-5"), 100, 10),
                    record(NOW - 10, Some("claude-sonnet-5"), 20, 2),
                ]),
            ),
            (
                "mini".to_string(),
                Ok(vec![record(NOW - 10, Some("claude-opus-5"), 5, 1)]),
            ),
        ];
        let report = aggregate(NOW, &per_host);

        assert_eq!(report.week.by_model["claude-opus-5"].input_tokens, 105);
        assert_eq!(report.week.by_model["claude-sonnet-5"].input_tokens, 20);
        assert_eq!(report.week.by_machine["here"].input_tokens, 120);
        assert_eq!(report.week.by_machine["mini"].input_tokens, 5);
        assert_eq!(report.week.total.turns, 3);
        assert_eq!(report.reached, vec!["here", "mini"]);
    }

    #[test]
    fn records_outside_the_week_are_dropped_and_within_the_day_are_double_counted() {
        let per_host = vec![(
            "here".to_string(),
            Ok(vec![
                record(NOW - 60 * 60, Some("m"), 1, 1), // within day and week
                record(NOW - 3 * DAY_SECS, Some("m"), 1, 1), // within week, not day
                record(NOW - 8 * DAY_SECS, Some("m"), 1, 1), // outside both
            ]),
        )];
        let report = aggregate(NOW, &per_host);

        assert_eq!(report.day.total.turns, 1);
        assert_eq!(report.week.total.turns, 2);
    }

    #[test]
    fn unreachable_hosts_are_kept_out_of_totals_not_zeroed() {
        let per_host = vec![
            (
                "here".to_string(),
                Ok(vec![record(NOW - 1, Some("m"), 3, 3)]),
            ),
            (
                "mini (ssh mini)".to_string(),
                Err("Connection refused".to_string()),
            ),
        ];
        let report = aggregate(NOW, &per_host);

        assert_eq!(report.week.total.turns, 1);
        assert_eq!(
            report.unreachable,
            vec![(
                "mini (ssh mini)".to_string(),
                "Connection refused".to_string()
            )]
        );
        assert!(!report.week.by_machine.contains_key("mini (ssh mini)"));
    }

    #[test]
    fn a_record_with_no_model_is_marked_unknown_not_folded_into_a_real_model() {
        let per_host = vec![(
            "here".to_string(),
            Ok(vec![
                record(NOW - 1, None, 7, 1),
                record(NOW - 1, Some("claude-opus-5"), 9, 1),
            ]),
        )];
        let report = aggregate(NOW, &per_host);

        assert_eq!(report.week.by_model[UNKNOWN_MODEL].input_tokens, 7);
        assert_eq!(report.week.by_model["claude-opus-5"].input_tokens, 9);
        // The unknown bucket still counts toward the honest total.
        assert_eq!(report.week.total.input_tokens, 16);
    }

    #[test]
    fn a_record_newer_than_now_counts_as_age_zero_rather_than_being_dropped() {
        let per_host = vec![(
            "here".to_string(),
            Ok(vec![record(NOW + 500, Some("m"), 1, 1)]),
        )];
        let report = aggregate(NOW, &per_host);
        assert_eq!(report.day.total.turns, 1);
    }

    #[test]
    fn render_names_an_unreachable_machine_instead_of_omitting_it() {
        let mut report = SpendReport::default();
        report.reached.push("here".to_string());
        report.unreachable.push((
            "mini (ssh mini)".to_string(),
            "timed out after 6s".to_string(),
        ));
        let text = render(&report, None);

        assert!(text.contains("mini (ssh mini): unreachable — timed out after 6s"));
        assert!(text.contains("almost certainly been spending"));
    }

    #[test]
    fn render_states_the_transcript_disclaimer_once() {
        let report = SpendReport::default();
        let text = render(&report, None);
        assert_eq!(
            text.matches("not your provider's own accounting").count(),
            1
        );
    }

    #[test]
    fn render_carries_a_discovery_note_when_rail_could_not_be_asked() {
        let report = SpendReport::default();
        let text = render(&report, Some("rail was not available"));
        assert!(text.starts_with("Desk spend"));
        assert!(text.contains("Note: rail was not available"));
    }

    #[test]
    fn render_flags_the_unknown_model_bucket_by_name() {
        let mut report = SpendReport::default();
        report.week.by_model.insert(
            UNKNOWN_MODEL.to_string(),
            Totals {
                input_tokens: 4,
                output_tokens: 1,
                turns: 2,
            },
        );
        let text = render(&report, None);
        assert!(text.contains("2 of the week's turns"));
        assert!(text.contains(UNKNOWN_MODEL));
    }

    #[test]
    fn parse_capture_lines_skips_blank_and_malformed_rows() {
        let good = record(1, Some("m"), 2, 3);
        let good_json = serde_json::to_string(&good).unwrap();
        let text = format!("\n{good_json}\nnot json\n  \n");
        let parsed = parse_capture_lines(&text);
        assert_eq!(parsed, vec![good]);
    }

    #[test]
    fn parse_rail_hosts_prefers_used_over_projects() {
        let doc = serde_json::json!({
            "ok": true,
            "data": {
                "hosts": [
                    {"name": "mini", "reach": "ssh mini", "local": false,
                     "projects": ["~/repo"], "used": ["/abs/repo"]},
                    {"name": "here", "reach": "", "local": true,
                     "projects": ["~/lisa"], "used": ["/home/john/lisa"]}
                ]
            }
        });
        let hosts = parse_rail_hosts(doc.to_string().as_bytes()).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "mini");
        assert_eq!(hosts[0].reach, "ssh mini");
        assert_eq!(hosts[0].projects, vec!["/abs/repo".to_string()]);
        assert!(hosts[1].is_local());
    }

    #[test]
    fn parse_rail_hosts_refuses_a_document_that_says_ok_false() {
        let doc = serde_json::json!({"ok": false, "data": null});
        assert_eq!(parse_rail_hosts(doc.to_string().as_bytes()), None);
    }

    #[test]
    fn parse_rail_hosts_refuses_unparseable_bytes() {
        assert_eq!(parse_rail_hosts(b"not json"), None);
    }

    #[test]
    fn discover_hosts_falls_back_to_the_one_project_and_says_why() {
        // No `rail` binary is assumed reachable in a unit test sandbox; this
        // documents the fallback shape independent of whether one happens to
        // be on PATH. The CLI-level integration test pins PATH to prove it.
        let (hosts, note) = discover_hosts(Path::new("/tmp/does-not-matter"));
        if note.is_some() {
            assert_eq!(hosts.len(), 1);
            assert_eq!(hosts[0].name, "here");
            assert!(hosts[0].is_local());
        }
    }

    #[test]
    fn display_label_names_the_reach_for_a_remote_host() {
        let host = DeskHost {
            name: "mini".to_string(),
            reach: "ssh mini".to_string(),
            projects: vec![],
        };
        assert_eq!(host.display_label(), "mini (ssh mini)");
    }

    use lisa_core::types::Priority;

    fn report_with_week_total(input: u64, output: u64) -> SpendReport {
        let mut report = SpendReport::default();
        report.week.total.input_tokens = input;
        report.week.total.output_tokens = output;
        report
    }

    #[test]
    fn no_allowance_configured_means_the_guard_never_acts() {
        let report = report_with_week_total(1_000, 0);
        let action = decide_guard_action(&report, None, Priority::Low);
        assert_eq!(action, GuardAction::NoAllowanceConfigured);
    }

    #[test]
    fn an_unreachable_host_refuses_to_act_even_over_threshold() {
        let mut report = report_with_week_total(950, 0);
        report
            .unreachable
            .push(("mini (ssh mini)".to_string(), "timed out".to_string()));
        let action = decide_guard_action(&report, Some(1_000), Priority::Low);
        assert_eq!(
            action,
            GuardAction::UnreadableSpend {
                unreachable: vec!["mini (ssh mini)".to_string()]
            }
        );
    }

    #[test]
    fn frontloading_under_the_threshold_is_legal_and_does_nothing() {
        // 80% spent, under the 90% mark — S-072-01 is explicit this is not an
        // error, even on a low-priority board.
        let report = report_with_week_total(800, 0);
        let action = decide_guard_action(&report, Some(1_000), Priority::Low);
        assert_eq!(
            action,
            GuardAction::BelowThreshold {
                spent: 800,
                allowance: 1_000
            }
        );
    }

    #[test]
    fn a_board_with_no_configured_priority_is_never_the_one_that_stops() {
        // Priority::default() is Medium — what an unconfigured board resolves
        // to (T-072-01-02 AC: "a board with no priority must not become the
        // one that gets stopped").
        let report = report_with_week_total(950, 0);
        let action = decide_guard_action(&report, Some(1_000), Priority::default());
        assert_eq!(
            action,
            GuardAction::NotEligible {
                spent: 950,
                allowance: 1_000,
                priority: Priority::Medium,
            }
        );
    }

    #[test]
    fn a_low_priority_board_over_threshold_stops() {
        let report = report_with_week_total(900, 50);
        let action = decide_guard_action(&report, Some(1_000), Priority::Low);
        assert_eq!(
            action,
            GuardAction::Stop {
                spent: 950,
                allowance: 1_000
            }
        );
    }

    #[test]
    fn exactly_the_threshold_stops_a_low_priority_board() {
        let report = report_with_week_total(900, 0);
        let action = decide_guard_action(&report, Some(1_000), Priority::Low);
        assert_eq!(
            action,
            GuardAction::Stop {
                spent: 900,
                allowance: 1_000
            }
        );
    }

    #[test]
    fn render_guard_action_covers_every_variant_without_panicking() {
        let variants = [
            GuardAction::NoAllowanceConfigured,
            GuardAction::UnreadableSpend {
                unreachable: vec!["mini".to_string()],
            },
            GuardAction::BelowThreshold {
                spent: 1,
                allowance: 2,
            },
            GuardAction::NotEligible {
                spent: 1,
                allowance: 2,
                priority: Priority::High,
            },
            GuardAction::Stop {
                spent: 1,
                allowance: 2,
            },
        ];
        for action in variants {
            let text = render_guard_action(&action);
            assert!(text.starts_with("Guard:"));
        }
    }
}
