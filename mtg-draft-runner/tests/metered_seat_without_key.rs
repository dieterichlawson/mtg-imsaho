//! A metered seat whose key is not set is refused before anything is dealt
//! (#903). The backends `expect` the variable, so the hosted table dealt the
//! draft and then printed a panic and a backtrace into the host's terminal
//! at the seat's first pick, and the seat was lost for the whole table.
//! No key is set here, so nothing can be billed whatever happens.

use std::process::Command;

fn refused(mut cmd: Command, var: &str) {
    let out = cmd.env_remove("ANTHROPIC_API_KEY").env_remove("GEMINI_API_KEY")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .stdin(std::process::Stdio::null())
        .output().expect("it runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "refused: {stderr}");
    assert!(stderr.contains(&format!("calls a metered API and {var} is not set")), "{stderr}");
    assert!(!stderr.contains("panicked"), "refused, not crashed: {stderr}");
}

#[test]
fn the_hosted_table_refuses_a_metered_seat_with_no_key() {
    for (spec, var) in [("ai:claude:claude-haiku-4-5", "ANTHROPIC_API_KEY"), ("ai:gemini", "GEMINI_API_KEY")] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"));
        cmd.args(["--seats", &format!("human,{spec}"), "--port", "1", "--best-of", "1"]);
        refused(cmd, var);
    }
}

#[test]
fn the_batch_runner_refuses_a_metered_seat_with_no_key() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"));
    cmd.args(["--players", "2", "--model", "claude:claude-haiku-4-5"]);
    refused(cmd, "ANTHROPIC_API_KEY");
}
