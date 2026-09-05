//! The Claude Code backend drives `claude -p` as a subprocess. These tests
//! stand in a fake `claude` (a shell script that records its argv and stdin
//! and prints a canned print-mode JSON result) and check the contract the
//! real CLI is spoken to with: session creation then resumption, the
//! system prompt on every call, `--json-schema` for structured decisions,
//! tools disabled, and the error/fallback path.
#![cfg(unix)]

use mtg_player::llm::MatchFormat;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A scratch directory holding the fake binary and its recordings.
struct Fake {
    dir: PathBuf,
}

impl Fake {
    /// `script_body` runs with `$LOG` (argv/stdin recording file) and
    /// `$CALL` (1-based call counter) set.
    fn new(name: &str, script_body: &str) -> Fake {
        let dir = std::env::temp_dir().join(format!("mtg-fake-claude-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("claude");
        let script = format!(
            "#!/bin/sh\nLOG=\"{log}\"\nCOUNT=\"{count}\"\n\
             if [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\n\
             CALL=$(( $(cat \"$COUNT\" 2>/dev/null || echo 0) + 1 )); echo $CALL > \"$COUNT\"\n\
             {{ echo \"=== call $CALL\"; for a in \"$@\"; do printf 'ARG: %s\\n' \"$a\"; done; \
             echo '--- stdin'; cat; echo; echo '--- end'; }} >> \"$LOG\"\n\
             {body}\n",
            log = dir.join("log.txt").display(),
            count = dir.join("count").display(),
            body = script_body
        );
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        Fake { dir }
    }

    fn bin(&self) -> String {
        self.dir.join("claude").display().to_string()
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.dir.join("log.txt")).unwrap_or_default()
    }

    fn calls(&self) -> Vec<String> {
        self.log()
            .split("=== call ")
            .skip(1)
            .map(str::to_string)
            .collect()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A fake that always succeeds, echoing back a session id and a structured
/// object whose `action` is 3 (and `word` is "ok" for other schemas).
const OK_BODY: &str = r#"
SID=""
prev=""
for a in "$@"; do
  if [ "$prev" = "--session-id" ] || [ "$prev" = "--resume" ]; then SID="$a"; fi
  prev="$a"
done
printf '{"type":"result","subtype":"success","is_error":false,"session_id":"%s","result":"{\"action\":3}","structured_output":{"action":3,"word":"ok","ready":true},"usage":{"input_tokens":10,"output_tokens":2,"cache_read_input_tokens":5,"cache_creation_input_tokens":1}}\n' "$SID"
"#;

fn arg_after<'a>(call: &'a str, flag: &str) -> Option<&'a str> {
    let lines: Vec<&str> = call.lines().collect();
    let idx = lines.iter().position(|l| *l == format!("ARG: {flag}"))?;
    lines.get(idx + 1)?.strip_prefix("ARG: ")
}

#[test]
fn first_call_creates_a_session_and_later_calls_resume_it() {
    let fake = Fake::new("session", OK_BODY);
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());

    assert_eq!(p.backend_send_for_test("pick"), "3");
    assert_eq!(p.backend_send_for_test("pick again"), "3");
    let calls = fake.calls();
    assert_eq!(calls.len(), 2, "two decisions, two subprocesses:\n{}", fake.log());

    let sid = arg_after(&calls[0], "--session-id").expect("first call sets --session-id");
    assert_eq!(sid.len(), 36, "session id is a uuid: {sid}");
    assert!(arg_after(&calls[0], "--resume").is_none());
    assert_eq!(arg_after(&calls[1], "--resume"), Some(sid), "second call resumes the same session");
    assert!(arg_after(&calls[1], "--session-id").is_none());

    for c in &calls {
        assert!(c.contains("ARG: -p\n"), "print mode");
        assert_eq!(arg_after(c, "--output-format"), Some("json"));
        assert_eq!(arg_after(c, "--tools"), Some(""), "tools disabled — the seat only answers prompts");
        let sys = arg_after(c, "--system-prompt").expect("system prompt every call");
        assert!(sys.contains("Magic: The Gathering"), "the game rules prompt is the system prompt");
        assert!(c.contains("--- stdin\npick"), "the decision prompt goes on stdin:\n{c}");
        let schema = arg_after(c, "--json-schema").expect("action calls are schema-constrained");
        assert!(schema.contains("\"action\""));
    }
    assert_eq!(p.conversation_len_for_test(), 4, "two exchanges");
    assert_eq!(p.model_name_for_test(), "claude-code");
}

#[test]
fn init_restarts_the_session_and_carries_the_deck_info() {
    let fake = Fake::new("init", OK_BODY);
    let registry = mtg_engine::cards::CardRegistry::with_all_cards();
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    p.backend_send_for_test("warm");
    p.init_conversation(&[("Swamp".to_string(), 17)], "Swamp | Land", &registry, MatchFormat::SingleGame);
    assert_eq!(p.conversation_len_for_test(), 0, "init starts a fresh conversation");
    p.backend_send_for_test("after init");

    let calls = fake.calls();
    assert_eq!(calls.len(), 2);
    let first = arg_after(&calls[0], "--session-id").unwrap();
    let second = arg_after(&calls[1], "--session-id").expect("a fresh session after init");
    assert_ne!(first, second);
    // The system prompt is multi-line, so look at the whole recorded call.
    assert!(!calls[0].contains("17x Swamp") && calls[1].contains("17x Swamp"),
        "deck info reaches the system prompt after init:\n{}", calls[1]);
}

#[test]
fn structured_calls_pass_the_schema_and_return_the_object() {
    let fake = Fake::new("schema", OK_BODY);
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"word": {"type": "string"}, "thoughts": {"type": "string"}, "n": {"type": "integer", "minimum": 0}},
        "required": ["word", "thoughts"]
    });
    let got = p.backend_send_with_schema_for_test("choose", &schema);
    assert_eq!(got["word"], "ok");

    let call = &fake.calls()[0];
    let passed: serde_json::Value = serde_json::from_str(arg_after(call, "--json-schema").unwrap()).unwrap();
    // Sanitized exactly like the API path: no thoughts field, no numeric
    // bounds, additionalProperties pinned.
    assert!(passed["properties"].get("thoughts").is_none());
    assert!(passed["properties"]["n"].get("minimum").is_none());
    assert_eq!(passed["additionalProperties"], false);
    assert_eq!(passed["required"], serde_json::json!(["word"]));
}

#[test]
fn resume_delivers_the_recap_as_a_real_turn() {
    let fake = Fake::new("resume", OK_BODY);
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    p.backend_resume_for_test("Turn 3: you cast Doomed Traveler.");
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].contains("Doomed Traveler"), "recap went to the model");
    assert!(arg_after(&calls[0], "--json-schema").unwrap().contains("ready"));
    assert_eq!(p.conversation_len_for_test(), 2);
}

/// A refusal the CLI prints as a result object on stdout (a usage limit
/// looks exactly like this: exit 1, empty stderr, the reason in `result`).
#[test]
fn a_stdout_refusal_is_reported_by_its_reason() {
    let body = r#"printf '{"type":"result","is_error":true,"result":"You have hit your usage limit"}
'; exit 1"#;
    let fake = Fake::new("refusal", body);
    let log = std::env::temp_dir().join(format!("mtg-fake-claude-refusal-log-{}", std::process::id()));
    mtg_player::game_log::init(log.to_str().unwrap()).unwrap();
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    assert_eq!(p.backend_send_for_test("pick"), "0");
    let logged = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_file(&log);
    assert!(logged.contains("usage limit"), "the reason reaches the log:\n{logged}");
}

#[test]
fn a_failing_cli_is_retried_then_falls_back_to_pass() {
    // Exit non-zero on every call.
    let fake = Fake::new("fail", "echo 'boom' >&2; exit 3");
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    assert_eq!(p.backend_send_for_test("pick"), "0", "the API path's fallback: pass priority");
    assert_eq!(fake.calls().len(), 3, "three attempts before giving up");
    assert_eq!(p.conversation_len_for_test(), 0, "a failed call is not an exchange");
}

#[test]
fn an_error_result_is_retried_and_a_later_success_wins() {
    // First call: is_error; second call: fine.
    let body = r#"
if [ "$CALL" = "1" ]; then
  printf '{"type":"result","is_error":true,"result":"rate limited","session_id":"s"}\n'
else
  printf '{"type":"result","is_error":false,"result":"{\"action\":5}","structured_output":{"action":5},"session_id":"s","usage":{}}\n'
fi
"#;
    let fake = Fake::new("error", body);
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    assert_eq!(p.backend_send_for_test("pick"), "5");
    assert_eq!(fake.calls().len(), 2);
}

#[test]
fn unstructured_text_results_fall_back_to_the_raw_text() {
    let body = r#"printf '{"type":"result","is_error":false,"result":"2","session_id":"s","usage":{}}\n'"#;
    let fake = Fake::new("text", body);
    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());
    assert_eq!(p.backend_send_for_test("pick"), "2");
}

// ── The subprocess contract: timeouts and cleanup (issues #203, #206) ─────

/// The backend's own call timeout, shortened so these run in seconds.
/// Read per call, so setting it here only caps the other tests in this
/// binary, which all answer immediately.
fn short_timeout(secs: u64) {
    std::env::set_var("MTG_CLAUDE_CODE_TIMEOUT_SECS", secs.to_string());
}

fn pid_alive(pid: i32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Issue #203: a wrapper script is the documented way to stand in for
/// `claude`, and any wrapper puts a process between the runner and the CLI.
/// Killing only the direct child left that descendant holding the stdout
/// pipe, so the read never saw EOF and the game blocked forever — long past
/// the timeout, with nothing logged and the child never even reaped.
#[test]
fn a_hung_call_times_out_even_when_a_grandchild_holds_stdout() {
    short_timeout(2);
    let fake = Fake::new(
        "hang",
        "sleep 120 & echo $! > \"$(dirname \"$LOG\")/grandchild.pid\"; sleep 120",
    );
    let bin = fake.bin();
    let dir = fake.dir.clone();

    // The call has to come back on its own. Running it on a thread means a
    // regression is a failed assertion rather than a test that hangs.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &bin);
        let _ = tx.send(p.backend_send_for_test("pick"));
    });

    // Three attempts of 2s each, plus the 2s and 4s retry backoffs.
    let answer = rx
        .recv_timeout(std::time::Duration::from_secs(60))
        .expect("a hung call must give up and return, not block the game forever");
    assert_eq!(answer, "0", "after every attempt times out, the seat falls back to pass");

    // And the whole tree is gone, not just the process we spawned.
    let pid: i32 = std::fs::read_to_string(dir.join("grandchild.pid"))
        .expect("the fake recorded its child")
        .trim()
        .parse()
        .expect("a pid");
    for _ in 0..50 {
        if !pid_alive(pid) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("grandchild {pid} outlived the call: killing the direct child is not enough");
}

/// Issue #206: `Drop` removes a seat's scratch directory, but nothing runs
/// on a signal or a `kill -9`, so `/tmp/mtg-claude-code-*` accumulated. A
/// new backend sweeps the directories of runs that are gone.
#[test]
fn a_new_backend_sweeps_scratch_directories_of_dead_runs() {
    let tmp = std::env::temp_dir();
    // A pid that really has exited, which is the case in the field.
    let mut done = std::process::Command::new("true").spawn().unwrap();
    let dead_pid = done.id();
    done.wait().unwrap();
    let dead = tmp.join(format!("mtg-claude-code-{dead_pid}-4242424"));
    std::fs::create_dir_all(dead.join("nested")).unwrap();
    std::fs::write(dead.join("nested/file"), "x").unwrap();
    // Our own run's directory must survive the sweep.
    let live = tmp.join(format!("mtg-claude-code-{}-4242425", std::process::id()));
    std::fs::create_dir_all(&live).unwrap();

    let fake = Fake::new("sweep", r#"printf '{"type":"result","is_error":false,"result":"1","session_id":"s","usage":{}}
'"#);
    let _p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("t", &fake.bin());

    assert!(!dead.exists(), "a dead run's scratch directory is swept, contents and all");
    assert!(live.exists(), "a live run's scratch directory is left alone");
    let _ = std::fs::remove_dir_all(&live);
}
