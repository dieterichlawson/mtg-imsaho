//! #1080: a model spec with nothing after its colon (`cc:`, `ai:cc:`) was
//! accepted and sent as `--model ""`, so every call failed and the seat
//! was handed empty answers. Each binary that takes a spec refuses it
//! before anything is dealt. No stub and no key: nothing is called.

use std::process::Command;

fn refused(mut cmd: Command, spec: &str) {
    let out = cmd.env_remove("ANTHROPIC_API_KEY").env_remove("GEMINI_API_KEY")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .stdin(std::process::Stdio::null())
        .output().expect("it runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "refused: {stderr}");
    assert!(stderr.contains(&format!("'{spec}' names no model after the colon")), "{stderr}");
    assert!(!stderr.contains("panicked"), "refused, not crashed: {stderr}");
}

#[test]
fn the_batch_runner_refuses_an_empty_model() {
    for spec in ["cc:", "gemini:"] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"));
        cmd.args(["--players", "2", "--model", spec, "--log", "/dev/null"]);
        refused(cmd, spec);
    }
}

#[test]
fn the_hosted_table_refuses_an_empty_model() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"));
    cmd.args(["--seats", "human,ai:cc:", "--port", "1", "--best-of", "1"]);
    refused(cmd, "cc:");
}

#[test]
fn the_pick_replay_refuses_an_empty_model() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"));
    cmd.args(["--positions", "/nonexistent", "--model", "cc:"]);
    refused(cmd, "cc:");
}
