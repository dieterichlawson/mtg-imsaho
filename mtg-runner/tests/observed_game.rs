//! Watching a game does not change it (#956).
//!
//! `--check-invariants` (and a gui seat) has the game loop route the passes
//! it makes on a seat's behalf through the seat, so the checker sees every
//! submit. The LLM seat took that pass as an offer of its own: its plan
//! stopped there with another reason, the next prompt said so, and seed 24
//! played Island where it played Mountain without the flag. A game found
//! under the fuzz flags has to replay without them.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// #953's decks: a plan-making seat with a curve to plan.
const BR: &str = "7 Swamp\n6 Mountain\n4 Island\n4 Abattoir Ghoul\n3 Morkrut Banshee\n3 Brimstone Volley\n\
3 Crossway Vampire\n3 Vampire Interloper\n3 Civilized Scholar\n3 Reaper from the Abyss\n3 Skirsdag High Priest\n\
4 Bloodcrazed Neonate\n4 Village Ironsmith\n";
const RG: &str = "9 Mountain\n8 Forest\n4 Gatstaf Shepherd\n4 Kruin Outlaw\n3 Darkthicket Wolf\n3 Brimstone Volley\n\
3 Moonmist\n3 Village Ironsmith\n3 Festerhide Boar\n3 Somberwald Spider\n3 Nightbird's Clutches\n4 Reckless Waif\n";

/// The game's own lines (`GAME`) and the seat's plan, without timestamps.
fn game(dir: &std::path::Path, name: &str, extra: &[&str]) -> Vec<String> {
    let log = dir.join(format!("{name}.log"));
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
        .env("STUB_CALLS", "/dev/null")
        .args(["--p1", "cc", "--p2", "random", "--seed", "24", "--quiet"])
        .args(["--deck1", dir.join("br.txt").to_str().unwrap(), "--deck2", dir.join("rg.txt").to_str().unwrap()])
        .args(["--log", log.to_str().unwrap()])
        .args(extra)
        .output()
        .expect("the runner runs");
    assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::read_to_string(&log).unwrap().lines()
        .filter_map(|l| l.splitn(4, '\t').nth(3))
        .filter(|l| l.contains("\tGAME\t") || l.contains("PLAN_"))
        .map(str::to_string)
        .collect()
}

#[test]
fn checking_invariants_plays_the_same_game() {
    if !Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("skipping: no python3 to run the prompt stub with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-observed-game-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("br.txt"), BR).unwrap();
    std::fs::write(dir.join("rg.txt"), RG).unwrap();
    let plain = game(&dir, "plain", &[]);
    let watched = game(&dir, "watched", &["--check-invariants"]);
    assert!(plain.iter().any(|l| l.contains("PLAN_")), "the seat made a plan to keep: {plain:?}");
    let first = plain.iter().zip(&watched).position(|(a, b)| a != b);
    assert!(first.is_none() && plain.len() == watched.len(),
        "the games part at line {first:?}: {:?} / {:?}", first.map(|i| &plain[i]), first.map(|i| &watched[i]));
    let _ = std::fs::remove_dir_all(&dir);
}
