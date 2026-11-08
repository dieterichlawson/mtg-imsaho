//! A pick the runner substituted is not the seat's answer (#1010).
//!
//! When a replayed pick's answer cannot be used, `mtg-draft-replay` takes
//! the pack's first card and `draft-replay.py` writes the row with
//! `"fallback": true`. The guide-replay instruments (`guide-effect.py`,
//! `judge-picks.py`) decide whether a draft guide is kept; counting a
//! substitution as an answer moved the no-guide noise and the settled set,
//! and sent a card nobody chose to the judge as the guide's. The resume
//! treated the row as done, so it was never asked again.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn have_python() -> bool {
    Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mtg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn rows(dir: &Path, name: &str, rows: &[(u32, &str, bool)]) {
    let text: String = rows
        .iter()
        .map(|(p, card, fb)| {
            format!(
                "{}\n",
                serde_json::json!({"position": format!("{p:04}"), "sample": 0, "action": card, "fallback": fb})
            )
        })
        .collect();
    std::fs::write(dir.join(format!("{name}.jsonl")), text).unwrap();
}

/// Four positions: 0 has a substituted no-guide pick, 1 a substituted
/// guided pick that differs from the settled card, 2 a real change on a
/// settled pick, 3 a settled pick the guide left alone. Card names are
/// ones `reports/draft-loop/stats.json` has 30+ games for.
fn night(dir: &Path) {
    let pack = ["Claustrophobia", "Kessig Wolf", "Spidery Grasp", "Traitorous Blood",
        "Ashmouth Hound", "Tormented Pariah", "Altar's Reap"];
    let positions: String = (0..4)
        .map(|i| {
            format!(
                "{}\n",
                serde_json::json!({"pack_number": 1, "pick_index": i + 1, "seat": 0,
                    "available": pack, "pool": []})
            )
        })
        .collect();
    std::fs::write(dir.join("positions.jsonl"), positions).unwrap();
    std::fs::write(dir.join("guide-current.md"), "A guide.\n").unwrap();
    rows(dir, "none-a", &[(0, "Claustrophobia", true), (1, "Spidery Grasp", false),
        (2, "Ashmouth Hound", false), (3, "Altar's Reap", false)]);
    rows(dir, "none-b", &[(0, "Kessig Wolf", false), (1, "Spidery Grasp", false),
        (2, "Ashmouth Hound", false), (3, "Altar's Reap", false)]);
    rows(dir, "current", &[(0, "Kessig Wolf", false), (1, "Traitorous Blood", true),
        (2, "Tormented Pariah", false), (3, "Altar's Reap", false)]);
}

#[test]
fn guide_effect_leaves_a_substituted_pick_out_of_every_figure() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let dir = scratch("guide-effect-fallback");
    night(&dir);
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/decision-eval/guide-effect.py"))
        .arg(&dir)
        .arg("current")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(
        stdout.contains("positions answered by all 3: 2 of 4; settled without a guide (a == b): 2"),
        "a substituted pick counted as an answer:\n{stdout}"
    );
    assert!(stdout.contains("none-a 1 (0000)") && stdout.contains("current 1 (0001)"), "{stdout}");
    let changed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("changed.json")).unwrap()).unwrap();
    let ids: Vec<u64> =
        changed["changed"].as_array().unwrap().iter().map(|c| c["position"].as_u64().unwrap()).collect();
    assert_eq!(ids, vec![2], "only the real change is a changed settled pick");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn judge_picks_never_sends_a_substituted_pick_to_the_judge() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let dir = scratch("judge-fallback");
    night(&dir);
    let log = dir.join("judge-calls.txt");
    let stub = dir.join("judge-stub.py");
    write_exe(
        &stub,
        &format!(
            "#!/usr/bin/env python3\nimport json, sys\nopen({:?}, 'a').write(sys.stdin.read() + '\\n---\\n')\n\
             print(json.dumps({{'result': json.dumps({{'reason': 'r', 'better': 'even'}})}}))\n",
            log.to_str().unwrap()
        ),
    );
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/decision-eval/judge-picks.py"))
        .args(["--dir", dir.to_str().unwrap(), "--config", "current", "--jobs", "1"])
        .env("CLAUDE_CODE_BIN", &stub)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(calls.matches("---").count(), 1, "one judgement, position 2 only:\n{calls}");
    assert!(calls.contains("Tormented Pariah") && !calls.contains("A: Traitorous Blood")
        && !calls.contains("B: Traitorous Blood"), "{calls}");
    assert!(stderr.contains("current: 1 substituted pick left out (0001)"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn draft_replay_asks_a_substituted_pick_again_on_resume() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let dir = scratch("draft-replay-fallback");
    let positions = dir.join("picks.jsonl");
    std::fs::write(&positions, "{\"seat\": 0}\n{\"seat\": 1}\n").unwrap();
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    // A run that stopped with 0000 substituted and 0001 answered.
    rows(&out_dir, "cfg", &[(0, "Kessig Wolf", true), (1, "Spidery Grasp", false)]);
    let log = dir.join("bin-calls.txt");
    let bin = dir.join("fake-replay.py");
    write_exe(
        &bin,
        &format!(
            "#!/usr/bin/env python3\nimport sys\np = sys.argv[sys.argv.index('--positions') + 1]\n\
             open({:?}, 'a').write(open(p).read())\nprint('{{\"chosen\": \"Ashmouth Hound\", \"usage\": {{}}}}')\n",
            log.to_str().unwrap()
        ),
    );
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/decision-eval/draft-replay.py"))
        .args(["--positions", positions.to_str().unwrap(), "--out", out_dir.to_str().unwrap()])
        .args(["--name", "cfg", "--model", "cc:stub", "--jobs", "1", "--bin", bin.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(calls.trim(), "{\"seat\": 0}", "only the substituted position is asked again:\n{stderr}");
    assert!(stderr.contains("cfg: 1 pick to replay (1 already done)"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compare_counts_a_substitution_but_never_scores_it() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let dir = scratch("compare-fallback");
    let row = |sample: u32, card: &str, fb: bool| {
        serde_json::json!({"position": "0000", "sample": sample, "action": card, "fallback": fb,
            "usage": {"calls": 1, "out": 0, "thinking": 0}, "system_bytes": 1, "prompt_bytes": 1})
        .to_string()
    };
    // The reference answered Y and was substituted X: one sample, settled on Y.
    std::fs::write(dir.join("ref.jsonl"), format!("{}\n{}\n", row(0, "X", true), row(1, "Y", false))).unwrap();
    // The candidate answered Y once; its substituted X is counted, not scored.
    std::fs::write(dir.join("cand.jsonl"), format!("{}\n{}\n", row(0, "Y", false), row(1, "X", true))).unwrap();
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/decision-eval/compare.py"))
        .args(["--out", dir.to_str().unwrap(), "--ref", "ref", "cand"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("1 positions, 1 sample(s) each, 1 settled"), "{stdout}");
    let cand = stdout.lines().find(|l| l.starts_with("cand")).unwrap();
    assert!(cand.contains("100.0% 1/1"), "the substitution was scored: {cand}");
    assert!(cand.trim_end().ends_with(" 1"), "the substitution is still counted: {cand}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1010's review: the judge's own cache kept a verdict by position only,
/// so one given on a substituted card outlived the pick being asked again.
#[test]
fn judge_picks_judges_again_when_a_cached_verdict_is_on_other_cards() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let dir = scratch("judge-stale");
    night(&dir);
    // Position 1 was asked again and answered; position 2's verdict stands.
    rows(&dir, "current", &[(0, "Kessig Wolf", false), (1, "Traitorous Blood", true),
        (1, "Claustrophobia", false), (2, "Tormented Pariah", false), (3, "Altar's Reap", false)]);
    let verdict = |pos: u32, guide: &str, reference: &str| {
        serde_json::json!({"position": pos, "pack": 1, "pick": pos + 1, "a": guide, "b": reference,
            "guide_card": guide, "ref_card": reference, "said": "A", "verdict": "guide", "reason": "r"})
        .to_string()
    };
    std::fs::write(dir.join("judge-current.jsonl"), format!("{}\n{}\n",
        verdict(1, "Traitorous Blood", "Spidery Grasp"), verdict(2, "Tormented Pariah", "Ashmouth Hound"))).unwrap();
    let log = dir.join("judge-calls.txt");
    let stub = dir.join("judge-stub.py");
    write_exe(
        &stub,
        &format!(
            "#!/usr/bin/env python3\nimport json, sys\nopen({:?}, 'a').write(sys.stdin.read() + '\\n---\\n')\n\
             print(json.dumps({{'result': json.dumps({{'reason': 'r', 'better': 'even'}})}}))\n",
            log.to_str().unwrap()
        ),
    );
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/decision-eval/judge-picks.py"))
        .args(["--dir", dir.to_str().unwrap(), "--config", "current", "--jobs", "1"])
        .env("CLAUDE_CODE_BIN", &stub)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stderr}");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(calls.matches("---").count(), 1, "position 1 is judged again, 2 is not:\n{calls}");
    assert!(calls.contains(": Claustrophobia\n") && !calls.contains(": Traitorous Blood\n"), "{calls}");
    assert!(stdout.contains("of 2 judged") && stdout.contains("better 1, worse 0, even 1"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1010's review: a game row's `fallback` is what `answered()` reads, so it
/// has to be the harness's own fallback: `log_rejected`'s MALFORMED or
/// NO_ANSWER, or a VALIDATION that trimmed or filled the answer. A
/// per-attempt BLOCKER_VALIDATION or API_ERROR can still end in the seat's
/// own answer.
#[test]
fn a_game_rows_fallback_is_the_harness_answering() {
    if !have_python() {
        eprintln!("skipping: no python3");
        return;
    }
    let cases = [
        ("MALFORMED [p0]\tsomething", true),
        ("NO_ANSWER [p0]\tclaude -p exited 1; {}", true),
        ("VALIDATION [p0]\ttarget-set: chose 3 of at most 2; keeping the first 2", true),
        ("VALIDATION [p0]\tobject-set: chose 0 but required at least 1; filling from the options", true),
        // The seat's own answer, sent as given; the engine cancels the cast.
        ("VALIDATION [p0]\texile-choice: chose 1 but required exactly 2; engine will cancel cast", false),
        ("BLOCKER_VALIDATION [p0]\tattempt 1 errors: []", false),
        ("API_ERROR [p0]\tretrying", false),
        ("CHOSE [p0]\t1 target(s)", false),
    ];
    for (label, want) in cases {
        let log = format!("2026-11-08T00:00:00Z\tERROR\tt1\tllm.rs:1\t{label}\n");
        let out = Command::new("python3")
            .args(["-c", "import sys; sys.path.insert(0, sys.argv[1]); import replay; print(replay.substituted(sys.argv[2]))"])
            .arg(repo_root().join("scripts/decision-eval"))
            .arg(&log)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), if want { "True" } else { "False" }, "{label}");
    }
}
