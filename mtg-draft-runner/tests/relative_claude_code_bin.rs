//! A `CLAUDE_CODE_BIN` given as a relative path is the same binary at the
//! check and at the call.
//!
//! The runners check the Claude Code binary up front so that a machine
//! without it refuses before a whole pod has been drafted and billed. That
//! check runs `<binary> --version` in the *process's* working directory,
//! and every real call runs the same binary with `current_dir(workdir)` —
//! the seat's scratch directory, which exists so no project `CLAUDE.md`
//! leaks into the prompt. So a relative path resolved during the check and
//! failed to resolve on every call afterwards: the guard passed a run it
//! exists to refuse, the run then spent the full retry budget per seat, and
//! the operator was told a file was missing rather than that their path was
//! relative (issue #540).
#![cfg(unix)]

use std::path::PathBuf;

/// A stub that always fails, loudly and identifiably. Whether it *ran* is
/// the whole question here, so it does not need to answer anything.
fn failing_stub(dir: &std::path::Path) -> PathBuf {
    let bin = dir.join("seat.sh");
    std::fs::write(
        &bin,
        "#!/usr/bin/env bash\n\
         if [ \"$1\" = \"--version\" ]; then echo '1.0.0 (stub)'; exit 0; fi\n\
         cat >/dev/null\n\
         echo 'stub: reached and refusing' >&2\n\
         exit 17\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_relative_binary_is_found_by_the_calls_and_not_only_by_the_check() {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    // Under `target/`, which is gitignored, so the relative path the runner
    // is handed is a real relative path from the directory it runs in.
    let rel = format!("target/relative-bin-{}", std::process::id());
    let dir = root.join(&rel);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    failing_stub(&dir);
    let relative_bin = format!("{rel}/seat.sh");

    let errfile = dir.join("stderr.txt");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", dir.join("run.log").to_str().unwrap()])
        .current_dir(root)
        .env("CLAUDE_CODE_BIN", &relative_bin)
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "2")
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&errfile).unwrap())
        .status()
        .expect("the runner runs");
    let stderr = std::fs::read_to_string(&errfile).unwrap_or_default();

    // The stub refuses every call, so the run is expected to fail. What
    // matters is *how*.
    assert!(!status.success(), "the stub refuses every call, so the run fails");
    assert!(
        !stderr.contains("No such file or directory"),
        "the calls could not find `{relative_bin}`, though the up-front check ran it \
         from this very directory — the binary is being resolved against the seat's \
         scratch workdir: {stderr}"
    );
    assert!(
        stderr.contains("stub: reached"),
        "the stub was never run by a call at all, only by the availability check: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bare_binary_name_is_still_a_path_lookup() {
    // One test, not three: these set a process-wide variable, and the
    // harness runs tests in the same binary on threads.
    let restore = std::env::var("CLAUDE_CODE_BIN").ok();

    // A bare name has to stay a bare name. `PATH` is searched in the child
    // and is not affected by where the child starts, so resolving it here
    // would break the default (`claude` on `PATH`) to fix the relative case.
    std::env::set_var("CLAUDE_CODE_BIN", "claude");
    assert_eq!(mtg_player::llm::claude_code_binary(), "claude");

    // An absolute path is already the answer.
    std::env::set_var("CLAUDE_CODE_BIN", "/opt/bin/claude");
    assert_eq!(mtg_player::llm::claude_code_binary(), "/opt/bin/claude");

    // A relative path with a separator resolves against this process's
    // working directory, which is where it was meant relative to.
    std::env::set_var("CLAUDE_CODE_BIN", "stub/seat.sh");
    let resolved = mtg_player::llm::claude_code_binary();
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "a relative binary path should be resolved once, here: {resolved:?}"
    );
    assert!(resolved.ends_with("stub/seat.sh"), "and still name the same file: {resolved:?}");
    assert_eq!(
        resolved,
        std::env::current_dir().unwrap().join("stub/seat.sh").to_string_lossy(),
        "resolved against the process's own directory, not the seat's workdir"
    );

    match restore {
        Some(v) => std::env::set_var("CLAUDE_CODE_BIN", v),
        None => std::env::remove_var("CLAUDE_CODE_BIN"),
    }
}
