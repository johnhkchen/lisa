//! One bounded, read-only provider pass that returns a validated proposal JSON.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use lisa_core::client::AgentClient;
use lisa_core::triage::TriageProposal;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct TriageAgentArgs {
    pub root: PathBuf,
    pub client: AgentClient,
    pub model: Option<String>,
    pub ticket_path: PathBuf,
    pub disposition_path: PathBuf,
    pub timeout_secs: u64,
    pub agent_bin: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriageAgentError {
    Failed(String),
    TimedOut,
}

pub fn run_triage_agent(args: &TriageAgentArgs) -> Result<String, TriageAgentError> {
    if args.timeout_secs == 0 {
        return Err(TriageAgentError::Failed(
            "triage timeout must be positive".to_string(),
        ));
    }
    let prompt = build_prompt(args);
    let mut command = build_command(args, &prompt);
    let mut stdout = tempfile::tempfile()
        .map_err(|error| TriageAgentError::Failed(format!("prepare stdout: {error}")))?;
    let mut stderr = tempfile::tempfile()
        .map_err(|error| TriageAgentError::Failed(format!("prepare stderr: {error}")))?;
    command
        .current_dir(&args.root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone().map_err(|error| {
            TriageAgentError::Failed(format!("prepare stdout: {error}"))
        })?))
        .stderr(Stdio::from(stderr.try_clone().map_err(|error| {
            TriageAgentError::Failed(format!("prepare stderr: {error}"))
        })?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command
        .spawn()
        .map_err(|error| TriageAgentError::Failed(format!("start triage agent: {error}")))?;
    let deadline = Duration::from_secs(args.timeout_secs);
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= deadline => {
                terminate(&mut child);
                let _ = child.wait();
                return Err(TriageAgentError::TimedOut);
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                terminate(&mut child);
                let _ = child.wait();
                return Err(TriageAgentError::Failed(format!(
                    "observe triage agent: {error}"
                )));
            }
        }
    };

    let stdout = read_capture(&mut stdout)
        .map_err(|error| TriageAgentError::Failed(format!("read stdout: {error}")))?;
    let stderr = read_capture(&mut stderr)
        .map_err(|error| TriageAgentError::Failed(format!("read stderr: {error}")))?;
    if !status.success() {
        return Err(TriageAgentError::Failed(
            first_line(&stderr).unwrap_or_else(|| format!("triage agent exited with {status}")),
        ));
    }

    let candidate = extract_candidate(args.client, &stdout).map_err(TriageAgentError::Failed)?;
    let proposal = TriageProposal::parse(candidate.as_bytes()).map_err(TriageAgentError::Failed)?;
    serde_json::to_string(&proposal)
        .map_err(|error| TriageAgentError::Failed(format!("serialize proposal: {error}")))
}

fn build_prompt(args: &TriageAgentArgs) -> String {
    format!(
        "You are Lisa's first responder. Read the blocked ticket at {} and its Review disposition at {}. Read every repository evidence path cited by either document. Do not modify files and do not run mutating commands. Return only one JSON object with: summary (one plain sentence naming what happened), recommendation (the action the operator should choose), and prepared_steps (one or more tagged objects). A command step is {{\"kind\":\"command\",\"description\":\"...\",\"command\":\"...\"}}. A file edit step is {{\"kind\":\"file-edit\",\"description\":\"...\",\"path\":\"repository/relative/path\",\"old\":\"exact old text\",\"new\":\"exact replacement\"}}. Propose only; the operator decides and applies it.",
        args.ticket_path.display(),
        args.disposition_path.display()
    )
}

fn build_command(args: &TriageAgentArgs, prompt: &str) -> Command {
    match args.client {
        AgentClient::Codex => {
            let mut command = Command::new(
                args.agent_bin
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("codex")),
            );
            command.args([
                "-a",
                "never",
                "exec",
                "--json",
                "--skip-git-repo-check",
                "-s",
                "read-only",
                "-C",
            ]);
            command.arg(&args.root);
            if let Some(model) = &args.model {
                command.args(["--model", model]);
            }
            command.arg(prompt);
            command
        }
        AgentClient::Claude => {
            let mut command = Command::new(
                args.agent_bin
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("claude")),
            );
            command.args([
                "-p",
                "--permission-mode",
                "dontAsk",
                "--allowedTools",
                "Read,Glob,Grep",
                "--output-format",
                "json",
            ]);
            if let Some(model) = &args.model {
                command.args(["--model", model]);
            }
            command.arg(prompt);
            command
        }
    }
}

fn extract_candidate(client: AgentClient, bytes: &[u8]) -> Result<String, String> {
    let text = String::from_utf8_lossy(bytes);
    let candidate = match client {
        AgentClient::Claude => {
            let value: Value = serde_json::from_str(text.trim())
                .map_err(|error| format!("invalid Claude result envelope: {error}"))?;
            value
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or_else(|| text.trim())
                .to_string()
        }
        AgentClient::Codex => {
            let mut last = None;
            for line in text.lines() {
                let Ok(value) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let item = value.get("item").or_else(|| value.get("payload"));
                let kind = item
                    .and_then(|item| item.get("type").or_else(|| item.get("item_type")))
                    .and_then(Value::as_str);
                if matches!(
                    kind,
                    Some("agent_message" | "assistant_message" | "message")
                ) {
                    last = item
                        .and_then(|item| item.get("text").or_else(|| item.get("content")))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
            }
            last.ok_or_else(|| "Codex returned no final agent message".to_string())?
        }
    };
    Ok(strip_fence(candidate.trim()).to_string())
}

fn strip_fence(value: &str) -> &str {
    value
        .strip_prefix("```json")
        .or_else(|| value.strip_prefix("```"))
        .and_then(|inner| inner.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(value)
}

fn read_capture(file: &mut File) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(1024 * 1024).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn first_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(240).collect())
}

#[cfg(unix)]
fn terminate(child: &mut std::process::Child) {
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate(child: &mut std::process::Child) {
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn executable_script(body: &str) -> (tempfile::TempDir, PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake-agent");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        (dir, path)
    }

    fn args(root: PathBuf, agent_bin: PathBuf, timeout_secs: u64) -> TriageAgentArgs {
        TriageAgentArgs {
            root,
            client: AgentClient::Claude,
            model: None,
            ticket_path: PathBuf::from("ticket.md"),
            disposition_path: PathBuf::from("review-disposition.json"),
            timeout_secs,
            agent_bin: Some(agent_bin),
        }
    }

    #[test]
    fn extracts_both_provider_envelopes() {
        let proposal = r#"{"summary":"The criterion conflicts with the evidence.","recommendation":"Amend it.","prepared_steps":[{"kind":"command","description":"Apply it.","command":"true"}]}"#;
        let claude = serde_json::json!({"result": proposal}).to_string();
        assert_eq!(
            extract_candidate(AgentClient::Claude, claude.as_bytes()).unwrap(),
            proposal
        );
        let codex = serde_json::json!({
            "type":"item.completed",
            "item":{"type":"agent_message","text":proposal}
        })
        .to_string();
        assert_eq!(
            extract_candidate(AgentClient::Codex, codex.as_bytes()).unwrap(),
            proposal
        );
    }

    #[test]
    fn prompt_names_exact_inputs_and_read_only_contract() {
        let args = TriageAgentArgs {
            root: PathBuf::from("/repo"),
            client: AgentClient::Codex,
            model: None,
            ticket_path: PathBuf::from("docs/ticket.md"),
            disposition_path: PathBuf::from("docs/work/review-disposition.json"),
            timeout_secs: 2,
            agent_bin: None,
        };
        let prompt = build_prompt(&args);
        assert!(prompt.contains("docs/ticket.md"));
        assert!(prompt.contains("review-disposition.json"));
        assert!(prompt.contains("Do not modify files"));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_returns_valid_proposal_and_surfaces_failure() {
        let proposal = r#"{"summary":"The criterion conflicts with the evidence.","recommendation":"Amend it.","prepared_steps":[{"kind":"command","description":"Apply it.","command":"true"}]}"#;
        let (dir, agent) = executable_script(&format!(
            "printf '%s\\n' '{{\"result\":{}}}'",
            serde_json::to_string(proposal).unwrap()
        ));
        let output = run_triage_agent(&args(dir.path().to_path_buf(), agent, 2)).unwrap();
        assert!(output.contains("criterion conflicts"));

        let (dir, agent) = executable_script("echo provider-down >&2; exit 9");
        let error = run_triage_agent(&args(dir.path().to_path_buf(), agent, 2)).unwrap_err();
        assert_eq!(error, TriageAgentError::Failed("provider-down".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn bounded_runner_kills_timeout_near_the_configured_deadline() {
        let (dir, agent) = executable_script("sleep 30");
        let started = Instant::now();
        let error = run_triage_agent(&args(dir.path().to_path_buf(), agent, 1)).unwrap_err();
        assert_eq!(error, TriageAgentError::TimedOut);
        assert!(started.elapsed() >= Duration::from_millis(900));
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
