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
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use super::{LlmBackend, ANTHROPIC_RESPONSE_FORMAT, GAME_RULES};

/// Environment variable naming the Claude Code binary; defaults to `claude`
/// on `PATH`.
pub const BINARY_ENV: &str = "CLAUDE_CODE_BIN";

/// How long one decision may take before the subprocess is killed and the
/// call retried. Print mode with thinking can run well past the API path's
/// two minutes.
const CALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Overrides [`CALL_TIMEOUT`], so the timeout path can be exercised in
/// seconds instead of five minutes. Not something a run should set.
const CALL_TIMEOUT_ENV: &str = "MTG_CLAUDE_CODE_TIMEOUT_SECS";

fn call_timeout() -> Duration {
    std::env::var(CALL_TIMEOUT_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(CALL_TIMEOUT, Duration::from_secs)
}

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

/// Process groups of `claude -p` children currently in flight, so a signal
/// can take them down with the run.
///
/// A slot holds a child's process-group id while its call is running and 0
/// otherwise. Fixed size and lock-free because the SIGINT/SIGTERM handler
/// reads it: everything a signal handler touches has to be
/// async-signal-safe, which rules out allocating or taking a lock (issue
/// #206). Four slots is more seats than a run has.
static LIVE_GROUPS: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];

/// Kill a child's whole process group.
///
/// `Child::kill` signals only the direct child. When that child is a
/// wrapper script — which is how `CLAUDE_CODE_BIN` is documented to be
/// exercised — the real work is a grandchild, which survives, keeps the
/// runner's stdout pipe open, and is orphaned to init (issues #203, #206).
/// The children are put in their own process group at spawn precisely so
/// this can reach all of them.
#[cfg(unix)]
fn kill_group(pgid: i32) {
    // Never signal our own group: that would take the runner with it. A
    // child whose `setpgid` failed is still in our group, so this is a real
    // guard, not a formality.
    if pgid > 0 && pgid != unsafe { libc::getpgrp() } {
        unsafe { libc::killpg(pgid, libc::SIGKILL) };
    }
}

#[cfg(not(unix))]
fn kill_group(_pgid: i32) {}

/// Kill every in-flight `claude -p` group, then die of the signal we were
/// sent. Async-signal-safe: `killpg`, `signal` and `raise` only.
#[cfg(unix)]
extern "C" fn handle_fatal_signal(sig: libc::c_int) {
    for slot in &LIVE_GROUPS {
        let pgid = slot.load(Ordering::Relaxed);
        if pgid > 0 && pgid != unsafe { libc::getpgrp() } {
            unsafe { libc::killpg(pgid, libc::SIGKILL) };
        }
    }
    // Re-raise with the default disposition so the exit status is the one
    // the caller expects from a Ctrl-C.
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

/// Take in-flight subprocesses down with the run on Ctrl-C or SIGTERM.
///
/// Without this the runner exits and its `claude -p` child (and whatever
/// that spawned) is reparented to init and keeps going — with a real seat,
/// a live model call still spending quota on a game that no longer exists
/// (issue #206).
#[cfg(unix)]
fn install_signal_handlers() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        libc::signal(libc::SIGINT, handle_fatal_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handle_fatal_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGHUP, handle_fatal_signal as *const () as libc::sighandler_t);
    });
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

/// Whether a process id belongs to a live process.
#[cfg(unix)]
fn pid_is_alive(pid: i32) -> bool {
    // `kill` reads 0 as "my whole process group" and negatives as other
    // groups, so only a positive pid is a question about one process. No
    // scratch directory is named with a non-positive pid anyway.
    if pid <= 0 {
        return false;
    }
    // ESRCH means no such process; EPERM means it exists and is not ours.
    unsafe { libc::kill(pid, 0) == 0 || *libc::__errno_location() == libc::EPERM }
}

#[cfg(not(unix))]
fn pid_is_alive(_pid: i32) -> bool {
    true
}

/// The prefix every seat's scratch directory is named with.
const WORKDIR_PREFIX: &str = "mtg-claude-code-";

/// Delete scratch directories left behind by runs that are no longer
/// running.
///
/// `Drop` removes a seat's own directory, but no destructor runs on a
/// signal or a `kill -9`, so these piled up in `/tmp` (issue #206). A
/// directory is named `mtg-claude-code-<pid>-<nonce>`; one whose pid is
/// gone belongs to a dead run and is ours to remove. A live pid — including
/// an unrelated process that has since been given that number — is left
/// alone.
fn sweep_stale_workdirs() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix(WORKDIR_PREFIX) else { continue };
        let Some((pid, _nonce)) = rest.split_once('-') else { continue };
        let Ok(pid) = pid.parse::<i32>() else { continue };
        if !pid_is_alive(pid) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// A registration in [`LIVE_GROUPS`], cleared when the call ends.
struct LiveGroup(Option<usize>);

impl LiveGroup {
    fn register(pgid: i32) -> Self {
        if pgid > 0 {
            for (i, slot) in LIVE_GROUPS.iter().enumerate() {
                if slot
                    .compare_exchange(0, pgid, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return Self(Some(i));
                }
            }
        }
        // More concurrent calls than slots: the call still runs, it just
        // isn't covered by the signal handler.
        Self(None)
    }
}

impl Drop for LiveGroup {
    fn drop(&mut self) {
        if let Some(i) = self.0.take() {
            LIVE_GROUPS[i].store(0, Ordering::SeqCst);
        }
    }
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
        install_signal_handlers();
        sweep_stale_workdirs();
        let workdir = std::env::temp_dir().join(format!(
            "{WORKDIR_PREFIX}{}-{}",
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

        // Give the child its own process group, so the timeout and the
        // signal handler can reach everything it spawns and not just the
        // wrapper script we launched (issues #203, #206).
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                // setpgid(0, 0): the child becomes leader of a new group
                // whose id is its own pid.
                if libc::setpgid(0, 0) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }

        let mut child = cmd.spawn().map_err(|e| format!("cannot run {}: {e}", self.binary))?;
        {
            let mut stdin = child.stdin.take().ok_or("no stdin")?;
            stdin.write_all(message.as_bytes()).map_err(|e| format!("write to claude stdin: {e}"))?;
        }
        let mut stdout = child.stdout.take().ok_or("no stdout")?;
        let mut stderr = child.stderr.take().ok_or("no stderr")?;

        // The group to signal. `setpgid` in `pre_exec` makes it the child's
        // own pid; if that call failed the child is still in ours, which
        // `kill_group` refuses to signal.
        let pgid = i32::try_from(child.id()).unwrap_or(0);
        let group = LiveGroup::register(pgid);

        // Read stdout on its own thread and wait on the *result*, not on
        // EOF. Waiting on EOF is what made the timeout toothless: the pipe
        // only closes when every process holding its write end is gone, so
        // one surviving grandchild kept the game blocked forever, long past
        // the timeout, with the killed child never even reaped (issue
        // #203).
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut out = String::new();
            let read = stdout.read_to_string(&mut out).map(|_| out);
            let _ = tx.send(read);
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });

        let timeout = call_timeout();
        let Ok(read) = rx.recv_timeout(timeout) else {
            // Take the whole group down, not just the process we spawned,
            // then reap our own child — `wait` on it returns as soon as it
            // dies, whatever its descendants are doing. The reader threads
            // are left to end when the pipes finally close; the call is
            // over either way, which is the point of the timeout.
            kill_group(pgid);
            drop(group);
            let _ = child.wait();
            return Err(format!("timed out after {}s", timeout.as_secs()));
        };
        let status = child.wait();
        drop(group);
        let err_text = stderr_reader.join().unwrap_or_default();
        let out = read.map_err(|e| format!("read claude stdout: {e}"))?;

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
