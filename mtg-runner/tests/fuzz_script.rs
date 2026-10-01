//! `scripts/fuzz.sh` bounds every game.
//!
//! The progress watchdog stops a game that makes no progress; nothing
//! stopped one that keeps making progress slowly. A flood board takes
//! 37-55 s per playout at 2,000 permanents and grows cubically, and an
//! unbounded game held its xargs worker until the nightly job's 90-minute
//! cap cancelled the shard — and a cancelled shard files no issue, so the
//! slowdown surfaced as a timed-out workflow with no seed attached (#644).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

#[test]
fn a_game_that_outlives_its_budget_is_a_finding_with_its_seed() {
    let have_timeout = std::process::Command::new("timeout").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success());
    if !have_timeout {
        eprintln!("skipping: no coreutils `timeout`");
        return;
    }
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let dir = std::env::temp_dir().join(format!("mtg-fuzz-script-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // A runner that never finishes — and says nothing, as a --quiet game does.
    let stub = dir.join("runner.sh");
    std::fs::write(&stub, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let deck = root.join("decks/coverage").read_dir().unwrap()
        .map(|e| e.unwrap().path()).find(|p| p.extension().is_some_and(|x| x == "txt")).unwrap();

    let started = std::time::Instant::now();
    let out = std::process::Command::new("bash")
        .arg(root.join("scripts/fuzz.sh")).args(["1", "77"])
        .current_dir(&dir)
        .env("FUZZ_RUNNER", &stub)
        .env("FUZZ_DECKS", &deck)
        .env("FUZZ_JOBS", "1")
        .env("FUZZ_GAME_TIMEOUT", "1")
        .output().unwrap();
    let elapsed = started.elapsed();
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The script cds to the repo root, so its logs land there.
    let logs: Vec<_> = std::fs::read_dir(root.join("logs")).into_iter().flatten()
        .filter_map(Result::ok).map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("fuzz-")))
        .collect();
    let mut timed_out_log = None;
    for d in &logs {
        for f in std::fs::read_dir(d).into_iter().flatten().filter_map(Result::ok) {
            let text = std::fs::read_to_string(f.path()).unwrap_or_default();
            if f.path().to_string_lossy().contains("-seed77.txt") && text.starts_with("TIMEOUT:") {
                timed_out_log = Some((d.clone(), text));
            }
        }
    }
    if let Some((d, _)) = &timed_out_log {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_dir_all(&dir);

    assert!(elapsed < std::time::Duration::from_secs(40),
        "the game ran {elapsed:?} — the per-game budget did not stop it");
    assert!(!out.status.success(), "a timed-out game is a failure: {stdout}");
    assert!(stdout.contains("seed 77 timed out"), "the seed is named: {stdout}");
    assert!(timed_out_log.is_some(),
        "the kept log says TIMEOUT, so the filing step does not skip it as empty: {stdout}");
}
