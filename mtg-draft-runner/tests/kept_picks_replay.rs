//! A pick a draft kept is asked again as the prompt it was live.
//!
//! `MTG_DRAFT_POSITIONS` keeps every asked pick's position (a pack's last
//! card is taken without asking); `mtg-draft-replay`
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
    assert!(!String::from_utf8_lossy(&replay.stderr).contains("not the live one"),
        "the same set file says nothing: {}", String::from_utf8_lossy(&replay.stderr));

    // #951: a position kept under another set file — a card removed, the
    // set's name edited — is said, as another history is.
    assert!(kept.iter().all(|p| p["reference"].as_str().is_some_and(|d| d.len() == 16)),
        "each position records the set its system prompt described");
    let other = dir.join("other-set.jsonl");
    std::fs::write(&other, kept.iter().take(2).map(|p| {
        let mut p = p.clone();
        p["reference"] = "0000000000000000".into();
        format!("{p}\n")
    }).collect::<String>()).unwrap();
    let moved = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .args(["--positions", other.to_str().unwrap(), "--model", "cc"])
        .output()
        .expect("the replay runs");
    let stderr = String::from_utf8_lossy(&moved.stderr);
    assert!(moved.status.success(), "{stderr}");
    let said: Vec<&str> = stderr.lines().filter(|l| l.contains("its system prompt is not the live one")).collect();
    assert_eq!(said.len(), 1, "said once, by the line it was first seen on: {stderr}");
    assert!(said[0].contains("line 1") && said[0].contains("is not the one it was asked about") && said[0].ends_with("(set isd)"),
        "{stderr}");

    // A metered seat is refused, not spent on.
    let metered = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
        .current_dir(repo_root())
        .args(["--positions", positions.to_str().unwrap(), "--model", "claude"])
        .output()
        .expect("the replay runs");
    assert!(!metered.status.success() && String::from_utf8_lossy(&metered.stderr).contains("--metered"));
    // A mistyped provider is named as one, with or without --metered, and
    // is not sent to --metered (#922).
    for extra in [&[][..], &["--metered"][..]] {
        let typo = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
            .current_dir(repo_root())
            .args(["--positions", positions.to_str().unwrap(), "--model", "CC"])
            .args(extra)
            .output()
            .expect("the replay runs");
        let err = String::from_utf8_lossy(&typo.stderr);
        assert!(!typo.status.success() && err.contains("unknown provider 'CC'") && !err.contains("metered seat"), "{err}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// #920: a guided seat's position records its guide and is asked under
/// it again; `--guide` asks every position under one file instead.
#[test]
fn a_guided_seats_pick_replays_under_its_guide() {
    if !Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("skipping: no python3 to run the prompt stub with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-kept-guided-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let positions = dir.join("picks.jsonl");
    let guide = dir.join("guide.md");
    std::fs::write(&guide, "Take removal first.\n").unwrap();
    let stub = repo_root().join("scripts/llm-prompt-stub.py");

    let draft = Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .env("MTG_DRAFT_POSITIONS", &positions)
        .env_remove("MTG_LLM_HISTORY")
        .env_remove("MTG_CLAUDE_CODE_DRAFT_ANSWERS")
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "8", "-q"])
        .args(["--guide-0", guide.to_str().unwrap()])
        .args(["--log", dir.join("draft.log").to_str().unwrap()])
        .output()
        .expect("the draft runs");
    assert!(draft.status.success(), "{}", String::from_utf8_lossy(&draft.stderr));
    let kept: Vec<serde_json::Value> = std::fs::read_to_string(&positions).expect("positions were kept")
        .lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    for p in &kept {
        let want = if p["seat"] == 0 { serde_json::json!("Take removal first.\n") } else { serde_json::Value::Null };
        assert_eq!(p["guide"], want, "the seat's guide is kept with its pick: {p}");
        assert_eq!(p["history"], 0);
        assert_eq!(p["text_answers"], false, "a cc seat asked in the schema");
    }

    let replay_under = |extra: &[&str], env: &[(&str, &str)]| -> (Vec<serde_json::Value>, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
            .current_dir(repo_root())
            .env("CLAUDE_CODE_BIN", &stub)
            .env("STUB_CALLS", "/dev/null")
            .env_remove("MTG_LLM_HISTORY")
            .env_remove("MTG_CLAUDE_CODE_DRAFT_ANSWERS")
            .envs(env.iter().copied())
            .args(["--positions", positions.to_str().unwrap(), "--model", "cc"])
            .args(extra)
            .output()
            .expect("the replay runs");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        (String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect(),
         String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let replay = |extra: &[&str]| {
        let (rows, stderr) = replay_under(extra, &[]);
        assert!(stderr.is_empty(), "nothing to warn of: {stderr}");
        rows
    };
    let system = |rows: &[serde_json::Value], seat: u64| -> Vec<u64> {
        let mut sizes: Vec<u64> = rows.iter().filter(|r| r["seat"] == seat)
            .map(|r| r["system_bytes"].as_u64().unwrap()).collect();
        sizes.dedup();
        sizes
    };
    let section = "\n## Draft Guide\n\nTake removal first.\n\n".len() as u64;
    let rows = replay(&[]);
    let (guided, plain) = (system(&rows, 0), system(&rows, 1));
    assert_eq!((guided.len(), plain.len()), (1, 1), "one system prompt a seat: {guided:?} {plain:?}");
    assert_eq!(guided[0], plain[0] + section, "seat 0 is asked with its guide, seat 1 without");

    let rows = replay(&["--guide", guide.to_str().unwrap()]);
    assert_eq!(system(&rows, 1), guided, "--guide asks every seat under the one file");

    // Asked in text where the live seat was asked in the schema, or under
    // another history: said once each, in the words draft-replay.py reads.
    let (_, stderr) = replay_under(&[], &[("MTG_CLAUDE_CODE_DRAFT_ANSWERS", "text"), ("MTG_LLM_HISTORY", "2")]);
    let unlike: Vec<&str> = stderr.lines().filter(|l| l.contains(": its system prompt is not the live one: ")).collect();
    assert_eq!(unlike.len(), 2, "{stderr}");
    assert!(unlike.iter().any(|l| l.contains("MTG_CLAUDE_CODE_DRAFT_ANSWERS=schema")), "{stderr}");
    assert!(unlike.iter().any(|l| l.contains("MTG_LLM_HISTORY=0")), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}
