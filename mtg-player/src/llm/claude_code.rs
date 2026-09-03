//! Claude Code backend: the LLM harness driven through `claude -p`.
//!
//! [`AnthropicBackend`](super::AnthropicBackend) talks to the Messages API
//! with an `ANTHROPIC_API_KEY`, which is metered and billed separately. This
//! backend runs the same prompt protocol through the Claude Code CLI in
//! print mode instead — one `claude -p` subprocess per decision, the
//! conversation kept in a Claude Code session (`--session-id` on the first
//! call, `--resume` after that) so the CLI's own prompt caching applies.
//! Whatever the CLI is logged into pays for it: for a subscription login
//! that is plan quota, not an API bill.
//!
//! The CLI runs with every tool disabled (`--tools ""`), so the model can
//! only answer the prompt — exactly like the API backend. Structured output
//! uses the CLI's `--json-schema`, which returns the parsed object in the
//! result's `structured_output` field.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{LlmBackend, ANTHROPIC_RESPONSE_FORMAT, GAME_RULES};

/// Environment variable naming the Claude Code binary; defaults to `claude`
/// on `PATH`.
pub const BINARY_ENV: &str = "CLAUDE_CODE_BIN";

/// How long one decision may take before the subprocess is killed and the
/// call retried. Print mode with thinking can run well past the API path's
/// two minutes.
const CALL_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_ATTEMPTS: u32 = 3;

/// The binary this process would run for a Claude Code seat.
#[must_use]
pub fn binary() -> String {
    std::env::var(BINARY_ENV).unwrap_or_else(|_| "claude".to_string())
}

/// Whether the Claude Code binary can be executed at all — the seat's
/// equivalent of "is the API key set", checked up front so a run refuses
/// cleanly instead of failing on the first decision.
#[must_use]
pub fn available() -> bool {
    Command::new(binary())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub(super) struct ClaudeCodeBackend {
    binary: String,
    /// Model alias/name passed as `--model`; `None` leaves the CLI default.
    model: Option<String>,
    /// Label used for usage accounting and `model_name()`.
    label: String,
    system_prompt: String,
    /// The session id once the first call has created it; `--resume`d after.
    session_id: Option<String>,
    /// Completed request/response exchanges in the current session.
    turns: usize,
    /// Scratch working directory for the subprocess, so no project
    /// `CLAUDE.md`, settings, or hooks from the caller's cwd leak into the
    /// game prompt.
    workdir: PathBuf,
}

impl ClaudeCodeBackend {
    pub(super) fn new(model: Option<&str>) -> Self {
        Self::with_binary(&binary(), model)
    }

    pub(super) fn with_binary(binary: &str, model: Option<&str>) -> Self {
        let workdir = std::env::temp_dir().join(format!(
            "mtg-claude-code-{}-{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        let _ = std::fs::create_dir_all(&workdir);
        let label = match model {
            Some(m) => format!("claude-code:{m}"),
            None => "claude-code".to_string(),
        };
        Self {
            binary: binary.to_string(),
            model: model.map(str::to_string),
            label,
            system_prompt: format!("{ANTHROPIC_RESPONSE_FORMAT}{GAME_RULES}"),
            session_id: None,
            turns: 0,
            workdir,
        }
    }

    /// A fresh RFC 4122 version-4 id for `--session-id`.
    fn fresh_session_id() -> String {
        let mut b = rand::random::<u128>().to_be_bytes();
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let h: Vec<String> = b.iter().map(|x| format!("{x:02x}")).collect();
        format!(
            "{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}",
            h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10], h[11], h[12], h[13], h[14], h[15]
        )
    }

    /// Run one `claude -p` call with `message` on stdin. Returns the CLI's
    /// JSON result object. Retries on spawn failure, non-zero exit, a
    /// timeout, unparsable output, or `is_error`; after the last attempt
    /// returns `None` and the caller falls back like the API path does.
    fn call(&mut self, message: &str, schema: Option<&serde_json::Value>) -> Option<serde_json::Value> {
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(2u64.pow(attempt)));
            }
            let started = std::time::Instant::now();
            match self.call_once(message, schema) {
                Ok(json) => {
                    let elapsed_ms = started.elapsed().as_millis();
                    if json["is_error"].as_bool() == Some(true) {
                        let msg = format!(
                            "claude -p reported an error (attempt {}/{}, {}ms): {}",
                            attempt + 1, MAX_ATTEMPTS, elapsed_ms,
                            json["result"].as_str().unwrap_or("").chars().take(200).collect::<String>()
                        );
                        eprintln!("{msg}");
                        crate::game_log::write(file!(), line!(), "API_RETRY", &msg);
                        continue;
                    }
                    if let Some(sid) = json["session_id"].as_str() {
                        // The id the CLI reports is authoritative; on the
                        // first call it is the one we asked for.
                        self.session_id = Some(sid.to_string());
                    }
                    self.turns += 1;
                    let usage = &json["usage"];
                    super::record_llm_usage(
                        &self.label,
                        usage["input_tokens"].as_u64().unwrap_or(0),
                        usage["output_tokens"].as_u64().unwrap_or(0),
                        usage["cache_read_input_tokens"].as_u64().unwrap_or(0),
                        usage["cache_creation_input_tokens"].as_u64().unwrap_or(0),
                    );
                    return Some(json);
                }
                Err(e) => {
                    let msg = format!(
                        "claude -p failed (attempt {}/{}, {}ms): {e}",
                        attempt + 1, MAX_ATTEMPTS, started.elapsed().as_millis()
                    );
                    eprintln!("{msg}");
                    crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                }
            }
        }
        let msg = format!("claude -p exhausted all {MAX_ATTEMPTS} attempts");
        eprintln!("{msg}");
        crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
        None
    }

    fn call_once(&mut self, message: &str, schema: Option<&serde_json::Value>) -> Result<serde_json::Value, String> {
        let mut cmd = Command::new(&self.binary);
        cmd.arg("-p")
            .args(["--output-format", "json"])
            .args(["--tools", ""])
            // The system prompt is sent every call: a resumed session
            // keeps its history, and restating the prompt is harmless.
            .args(["--system-prompt", &self.system_prompt]);
        match &self.session_id {
            Some(id) => {
                cmd.args(["--resume", id]);
            }
            None => {
                cmd.args(["--session-id", &Self::fresh_session_id()]);
            }
        }
        if let Some(m) = &self.model {
            cmd.args(["--model", m]);
        }
        if let Some(s) = schema {
            cmd.args(["--json-schema", &s.to_string()]);
        }
        cmd.current_dir(&self.workdir)
            // A Claude Code session marks its environment so nested
            // interactive sessions are refused; a print-mode seat spawned
            // from inside one is fine and must not inherit the mark.
            .env_remove("CLAUDECODE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("cannot run {}: {e}", self.binary))?;
        {
            let mut stdin = child.stdin.take().ok_or("no stdin")?;
            stdin.write_all(message.as_bytes()).map_err(|e| format!("write to claude stdin: {e}"))?;
        }
        let mut stdout = child.stdout.take().ok_or("no stdout")?;
        let mut stderr = child.stderr.take().ok_or("no stderr")?;

        // Watchdog: kill the child if it outlives the call timeout. Reading
        // stdout to EOF below then returns, and the wait sees the kill.
        let child = Arc::new(Mutex::new(child));
        let done = Arc::new(AtomicBool::new(false));
        let timed_out = Arc::new(AtomicBool::new(false));
        {
            let child = Arc::clone(&child);
            let done = Arc::clone(&done);
            let timed_out = Arc::clone(&timed_out);
            std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + CALL_TIMEOUT;
                while std::time::Instant::now() < deadline {
                    if done.load(Ordering::SeqCst) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                if !done.load(Ordering::SeqCst) {
                    timed_out.store(true, Ordering::SeqCst);
                    if let Ok(mut c) = child.lock() {
                        let _ = c.kill();
                    }
                }
            });
        }
        let stderr_reader = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });
        let mut out = String::new();
        let read = stdout.read_to_string(&mut out);
        done.store(true, Ordering::SeqCst);
        let status = child.lock().map_err(|_| "child lock poisoned".to_string())?.wait();
        let err_text = stderr_reader.join().unwrap_or_default();
        read.map_err(|e| format!("read claude stdout: {e}"))?;

        if timed_out.load(Ordering::SeqCst) {
            return Err(format!("timed out after {}s", CALL_TIMEOUT.as_secs()));
        }
        let status = status.map_err(|e| format!("wait: {e}"))?;
        if !status.success() {
            // The CLI reports refusals (a usage limit, a bad model name) as
            // a result object on stdout with a non-zero exit and an empty
            // stderr — surface whichever stream says why.
            let reason = if err_text.trim().is_empty() { out.trim() } else { err_text.trim() };
            let reason = serde_json::from_str::<serde_json::Value>(reason)
                .ok()
                .and_then(|j| j["result"].as_str().map(str::to_string))
                .unwrap_or_else(|| reason.to_string());
            let snippet: String = reason.chars().take(300).collect();
            return Err(format!("exit {status}: {snippet}"));
        }
        serde_json::from_str(out.trim())
            .map_err(|e| format!("unparsable result JSON ({e}): {}", out.trim().chars().take(200).collect::<String>()))
    }

    /// The structured object of a result: the CLI's parsed
    /// `structured_output` when a schema was given, else the result text
    /// parsed as JSON.
    fn structured(json: &serde_json::Value) -> Option<serde_json::Value> {
        if json["structured_output"].is_object() {
            return Some(json["structured_output"].clone());
        }
        json["result"].as_str().and_then(|t| serde_json::from_str(t.trim()).ok())
    }
}

impl Drop for ClaudeCodeBackend {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.workdir);
    }
}

impl LlmBackend for ClaudeCodeBackend {
    fn send(&mut self, message: &str) -> String {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"action": {"type": "integer"}},
            "required": ["action"],
            "additionalProperties": false
        });
        let Some(json) = self.call(message, Some(&schema)) else {
            return "0".to_string();
        };
        if let Some(action) = Self::structured(&json).and_then(|s| s["action"].as_u64()) {
            return action.to_string();
        }
        json["result"].as_str().unwrap_or("0").trim().to_string()
    }

    fn send_with_schema(&mut self, message: &str, schema: &serde_json::Value) -> serde_json::Value {
        let sanitized = super::AnthropicBackend::sanitize_schema(schema);
        match self.call(message, Some(&sanitized)) {
            Some(json) => Self::structured(&json).unwrap_or_else(|| serde_json::json!({})),
            None => serde_json::json!({}),
        }
    }

    fn init(&mut self, deck_info: &str) {
        self.system_prompt = format!("{ANTHROPIC_RESPONSE_FORMAT}{GAME_RULES}{deck_info}");
        self.session_id = None;
        self.turns = 0;
    }

    fn resume(&mut self, recap: &str) {
        // The API backend fakes the acknowledgement turn; a CLI session is
        // real history, so the recap is delivered as a genuine turn.
        let schema = serde_json::json!({
            "type": "object",
            "properties": {"ready": {"type": "boolean"}},
            "required": ["ready"],
            "additionalProperties": false
        });
        let message = format!(
            "{recap}\n\nReview this history; the game continues from here. \
             Reply with {{\"ready\": true}}."
        );
        let _ = self.call(&message, Some(&schema));
    }

    fn conversation_len(&self) -> usize {
        self.turns * 2
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn model_name(&self) -> &str {
        &self.label
    }
}
