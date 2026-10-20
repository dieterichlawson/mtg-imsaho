//! A pick a draft kept is asked again as the prompt it was live.
//!
//! `MTG_DRAFT_POSITIONS` keeps every pick's position; `mtg-draft-replay`
//! asks those positions again, which is what the draft's pick-quality eval
//! is built on (`scripts/decision-eval/draft-replay.py`). The eval compares
//! models and builds on the same positions, so a position that replayed as
//! a different prompt than the live one would score the replay, not the
//! seat. The prompt stub answers by hashing the prompt, so a replayed pick
//! that matches the live one was asked the same question.
#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

#[test]
fn every_kept_pick_replays_as_the_pick_it_was_live() {
    if !Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("skipping: no python3 to run the prompt stub with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-kept-picks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let positions = dir.join("picks.jsonl");
    let stub = repo_root().join("scripts/llm-prompt-stub.py");

    let draft = Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .env("MTG_DRAFT_POSITIONS", &positions)
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", dir.join("draft.log").to_str().unwrap()])
        .output()
        .expect("the draft runs");
    assert!(draft.status.success(), "{}", String::from_utf8_lossy(&draft.stderr));
    let kept: Vec<serde_json::Value> = std::fs::read_to_string(&positions).expect("positions were kept")
        .lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(kept.len(), 78, "two seats, three packs of fourteen, each but its last card asked");
    assert!(kept.iter().all(|p| p["available"].as_array().is_some_and(|a| !a.is_empty())
        && p["notes"].is_null() == (p["pack_number"] == 1 && p["pick_index"] == 1)),
        "each position carries its pack and the notes the seat brought to it");

    let replay = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .args(["--positions", positions.to_str().unwrap(), "--model", "cc"])
        .output()
        .expect("the replay runs");
    assert!(replay.status.success(), "{}", String::from_utf8_lossy(&replay.stderr));
    let replayed: Vec<serde_json::Value> = String::from_utf8_lossy(&replay.stdout)
        .lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(replayed.len(), kept.len());
    for r in &replayed {
        assert_eq!(r["chosen"], r["live"], "position {} was asked as another prompt: {r}", r["id"]);
        assert_eq!(r["usage"]["calls"], 1, "one call a pick: {r}");
    }

    // A metered seat is refused, not spent on.
    let metered = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
        .current_dir(repo_root())
        .args(["--positions", positions.to_str().unwrap(), "--model", "claude"])
        .output()
        .expect("the replay runs");
    assert!(!metered.status.success() && String::from_utf8_lossy(&metered.stderr).contains("--metered"));
    let _ = std::fs::remove_dir_all(&dir);
}
