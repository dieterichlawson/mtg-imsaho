//! #1078: a `--log` that stops accepting writes partway through a game was
//! dropped in silence — the game played to its end and exited 0 with the
//! log cut mid-line. The run now says so when it is cut and at its end,
//! and exits 1, as a `--save` that cannot be written does.

#[cfg(target_os = "linux")]
#[test]
fn a_log_cut_by_a_full_disk_is_said_and_the_run_does_not_exit_0() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "random", "--p2", "random", "--seed", "1", "-q", "--log", "/dev/full"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run mtg-runner");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "a run whose log was cut exits 1; stderr: {stderr}");
    assert_eq!(stderr.matches("warning: the log '/dev/full' stopped accepting writes").count(), 1,
        "said once, when it was cut: {stderr}");
    assert!(stderr.contains("the game finished but its log is incomplete"), "and at the end: {stderr}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("Game over!"), "the game still played out");
}
