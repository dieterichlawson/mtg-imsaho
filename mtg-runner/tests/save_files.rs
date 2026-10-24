//! What the runner does to the files it writes.
//!
//! Two of them: the `--save` file the operator named, and the hot-reload
//! snapshot it writes to the temp dir before every decision whether or not
//! `--save` was given. Both had ways of surprising the operator — a save
//! that replaced a symlink instead of writing through it (#215), a save
//! unlinked at game over even when the path held something the runner never
//! created (#237, #242), a snapshot that panicked the game when the temp dir
//! was full (#214), and a world-readable snapshot of both players' hands
//! left behind on every abnormal exit (#234, #239).

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// A scratch directory of this test's own, also used as its `TMPDIR` so the
/// hot-reload snapshots of one test cannot be seen by another.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("mtg-save-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch { dir }
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A short random-vs-random game that runs to a natural game over.
fn play_a_game(scratch: &Scratch, save: &str) -> std::process::Output {
    runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .args(["--p1", "random", "--p2", "random", "--seed", "5", "--quiet"])
        .args(["--save", save])
        .output()
        .expect("failed to run the runner")
}

/// Issue #215: the atomic write renames onto the path, and `rename(2)` does
/// not follow a final symlink — so every save landed next to the link,
/// destroying it on the first write, while the target the operator pointed
/// at stayed empty. `--log`, which opens rather than renames, followed the
/// link as anyone would expect.
#[test]
fn a_symlinked_save_path_is_written_through_not_replaced() {
    let scratch = Scratch::new("symlink");
    let target = scratch.path("real.save");
    let link = scratch.path("link.save");
    std::fs::write(&target, "").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let out = play_a_game(&scratch, &link);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let link_meta = std::fs::symlink_metadata(&link).expect("the link is still there");
    assert!(
        link_meta.file_type().is_symlink(),
        "the symlink must survive: the runner was told to write a file, not to replace a link"
    );
    assert!(
        std::fs::metadata(&target).unwrap().len() > 0,
        "the save has to reach the path the link points at"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("is a symlink; writing through it"),
        "following the link is worth a word: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Issues #237 and #242: game over unlinked the path unconditionally — the
/// final position was gone, `--resume` on the path the operator had been
/// using all game failed, and a pre-existing file at that path (including
/// the very save a `--resume x --save x` was playing from) was destroyed.
#[test]
fn the_save_outlives_the_game_and_holds_the_final_position() {
    let scratch = Scratch::new("survives");
    let save = scratch.path("g.save");

    let out = play_a_game(&scratch, &save);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        Path::new(&save).exists(),
        "the file the operator named is still there after the game"
    );
    // A save holds both hands, both libraries in draw order and the Private
    // log lines — the answer key, the same as the hot-reload snapshot — so it
    // is written for its owner alone. It kept the umask and was 0644 (#709).
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&save).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the save is readable by its owner only, not {mode:o}");
    }

    // And it is the *final* position, which is now read back out of the
    // refusal: a decided game has nothing to continue, so `--resume` on it
    // exits nonzero and says who won, how long the game was and what turn
    // it ended on (issue #316). It used to announce a resume, do nothing,
    // print the game-over summary a second time and exit 0.
    let resumed = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .args(["--resume", &save, "--p1", "random", "--p2", "random", "--quiet"])
        .output()
        .expect("failed to run the runner");
    let err = String::from_utf8_lossy(&resumed.stderr);
    assert!(!resumed.status.success(),
        "resuming a decided game is a refusal, not a success: {err}");
    assert!(err.contains("is a finished game") && err.contains("nothing to resume"),
        "the refusal says why: {err}");
    assert!(err.contains("won") || err.contains("draw"),
        "and reads the outcome back out of the save: {err}");
    let (before, _) = err.split_once(" actions, turn ")
        .unwrap_or_else(|| panic!("the refusal reports the game's length: {err}"));
    let count: String = before.chars().rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>().into_iter().rev().collect();
    assert!(!count.is_empty() && count != "0",
        "the save holds the whole game, not an empty one: {err}");
    assert!(String::from_utf8_lossy(&resumed.stdout).is_empty()
        || !String::from_utf8_lossy(&resumed.stdout).contains("resuming from"),
        "and nothing claims a resume happened: {}", String::from_utf8_lossy(&resumed.stdout));
}

/// A path that already holds something gets a word about it, since the save
/// overwrites from the first decision (issue #242's first case).
#[test]
fn an_existing_file_at_the_save_path_is_announced() {
    let scratch = Scratch::new("existing");
    let save = scratch.path("precious.txt");
    std::fs::write(&save, "my important tournament notes\n").unwrap();

    let out = play_a_game(&scratch, &save);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("already exists and will be overwritten"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Issue #214: the hot-reload snapshot — which nobody asked for — was
/// written with `.expect()`, so a full temp dir panicked the game with a
/// backtrace and exit 101, while the `--save` sibling three lines away
/// failed cleanly.
#[test]
fn a_snapshot_that_cannot_be_written_warns_instead_of_panicking() {
    let scratch = Scratch::new("nospace");
    let save = scratch.path("g.save");

    let out = runner()
        .current_dir(repo_root())
        .env("TMPDIR", "/nonexistent-dir-for-this-test")
        .args(["--p1", "random", "--p2", "random", "--seed", "5", "--quiet"])
        .args(["--save", &save])
        .output()
        .expect("failed to run the runner");

    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("panicked"), "no panic, no backtrace: {err}");
    assert_eq!(out.status.code(), Some(0), "the game the operator asked for still finishes: {err}");
    assert!(err.contains("cannot write the hot-reload snapshot"), "and says so once: {err}");
    assert_eq!(
        err.matches("cannot write the hot-reload snapshot").count(),
        1,
        "once, not before every decision: {err}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Game over!"),
        "the game played to its end"
    );
    assert!(Path::new(&save).exists(), "and the save the operator did ask for was written");
}

/// Issues #239 and #234: the snapshot is a complete game state — both
/// hands and both libraries in draw order — and was written mode 644 into a
/// shared /tmp, then left there by every exit but the normal one. One night
/// of playtesting stranded 91 files and 8.6 MB.
#[test]
fn the_snapshot_is_private_and_a_dead_run_does_not_keep_it() {
    let scratch = Scratch::new("private");
    let save = scratch.path("g.save");

    // A game slow enough to still be running when we look at its snapshot.
    let mut child = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .args(["--p1", "random", "--p2", "random", "--seed", "4242", "--quiet"])
        .args(["--check-invariants", "--save", &save])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run the runner");
    let snapshot = scratch.dir.join(format!("mtg-hot-reload-{}.json", child.id()));

    let mut mode = None;
    for _ in 0..100 {
        if let Ok(meta) = std::fs::metadata(&snapshot) {
            mode = Some(meta.permissions().mode() & 0o777);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let mode = mode.expect("the snapshot is written before every decision");
    assert_eq!(mode, 0o600, "nothing but this process reads it, and it holds both hands");

    // A kill no cleanup can survive strands it...
    let _ = child.kill();
    let _ = child.wait();
    assert!(snapshot.exists(), "precondition: a killed run cannot clean up after itself");

    // ...and the next run reaps it, because its pid is gone.
    let out = play_a_game(&scratch, &save);
    assert!(out.status.success());
    assert!(
        !snapshot.exists(),
        "a snapshot whose run is dead is swept: {}",
        snapshot.display()
    );
}

/// #735: a refused `--resume` left the `--log` it had opened with no record
/// of why the run stopped; the reason was on stderr only.
#[test]
fn a_refused_resume_says_why_in_the_log() {
    let scratch = Scratch::new("refused-log");
    let save = scratch.path("not-a-save.json");
    std::fs::write(&save, "{ not json").unwrap();
    let log = scratch.path("run.log");
    let out = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .args(["--p1", "random", "--p2", "random", "--quiet", "--resume", &save, "--log", &log])
        .output()
        .expect("failed to run the runner");
    assert!(!out.status.success());
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(text.contains("FATAL save file") && text.contains("is not a valid game save"),
        "the log says why the run stopped:\n{text}");
}

/// #836: the notes block — the tail of an LLM seat's last reasoning — is
/// how a seat sent no history carries a plan to its next decision, and the
/// save did not hold it, so the first prompt after `--resume` had the
/// recap and the board and not the plan made just before the save.
///
/// A stub seat kills the runner partway through, the way an operator's
/// Ctrl-C or a dead machine would; the resumed game's first prompt has to
/// carry the notes the save was written with.
#[test]
fn a_resumed_llm_seat_gets_its_notes_back() {
    let scratch = Scratch::new("notes");
    let stub = scratch.path("seat.py");
    std::fs::write(&stub, format!(r#"#!/usr/bin/env python3
import os, signal, subprocess, sys
stub = {real:?}
if sys.argv[1:2] == ["--version"]:
    os.execv(sys.executable, [sys.executable, stub, "--version"])
counter = {counter:?}
n = int(open(counter).read()) if os.path.exists(counter) else 0
open(counter, "w").write(str(n + 1))
if os.environ.get("KILL_AT") and n + 1 == int(os.environ["KILL_AT"]):
    os.kill(os.getppid(), signal.SIGKILL)
    sys.exit(1)
msg = sys.stdin.read()
r = subprocess.run([sys.executable, stub, *sys.argv[1:]], input=msg, capture_output=True, text=True)
sys.stdout.write(r.stdout)
"#, real = repo_root().join("scripts/llm-prompt-stub.py").to_string_lossy(),
        counter = scratch.path("calls")))
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let save = scratch.path("game.json");

    let killed = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .env("KILL_AT", "12")
        .args(["--p1", "cc", "--p2", "random", "--seed", "5", "--quiet", "--save", &save])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("the runner runs");
    assert!(!killed.success(), "the stub killed the run mid-game");
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&save).unwrap()).unwrap();
    let notes = saved["seat_notes"][0].as_str()
        .unwrap_or_else(|| panic!("the save holds seat p0's notes: {}", saved["seat_notes"]))
        .to_string();
    assert!(saved["seat_notes"][1].is_null(), "the random seat keeps none");

    let log = scratch.path("resumed.log");
    let resumed = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .env("CLAUDE_CODE_BIN", &stub)
        .env("STUB_CALLS", "/dev/null")
        .args(["--resume", &save, "--seed", "5", "--quiet", "--log", &log])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("the resumed runner runs");
    assert!(resumed.success(), "the resumed game plays out: {resumed}");
    let logged = std::fs::read_to_string(&log).unwrap();
    let first_prompt = logged.split("\tPROMPT [").nth(1).expect("the resumed seat was asked something");
    let first_prompt = first_prompt.split("\n20").next().unwrap();
    assert!(first_prompt.contains(&format!("Your notes from your last decision:\n{notes}")),
        "the first prompt after the resume carries the saved notes {notes:?}:\n{first_prompt}");
}

/// The prompt bodies of a log, in order.
fn prompt_bodies(log: &str) -> Vec<String> {
    let mut bodies = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in log.lines() {
        let is_header = line.len() > 4 && line.as_bytes()[..4].iter().all(u8::is_ascii_digit);
        if is_header {
            if let Some(body) = current.take() {
                bodies.push(body.join("\n"));
            }
            if line.contains("\tPROMPT [") {
                current = Some(Vec::new());
            }
        } else if let Some(body) = current.as_mut() {
            body.push(line);
        }
    }
    if let Some(body) = current {
        bodies.push(body.join("\n"));
    }
    bodies
}

/// A save holds the LLM seat's whole memory — notes, how far into the log
/// it has been shown, a pass-until in progress — so a resumed seat's next
/// prompt is the one it was sent live, byte for byte, with no recap of the
/// game in front of it. That is what lets `--resume <save> --stop-after 1`
/// replay a saved position as one decision (`scripts/decision-eval/`).
///
/// `capture-claude.py` keeps the save at every model call of a stub game;
/// each kept position is then resumed for one decision, and its prompt
/// must be one the live game sent.
#[test]
fn every_saved_position_resumes_as_the_prompt_it_was_live() {
    let scratch = Scratch::new("exact");
    let save = scratch.path("live.json");
    let positions = scratch.path("positions");
    let live_log = scratch.path("live.log");
    let live = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .env("CLAUDE_CODE_BIN", repo_root().join("scripts/decision-eval/capture-claude.py"))
        .env("EVAL_REAL_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
        .env("EVAL_SAVE", &save)
        .env("EVAL_POSITIONS", &positions)
        .env("STUB_CALLS", "/dev/null")
        .args(["--p1", "cc", "--p2", "random", "--seed", "3", "--quiet", "--save", &save, "--log", &live_log])
        .output()
        .expect("the runner runs");
    assert!(live.status.success(), "the live game plays out: {}", String::from_utf8_lossy(&live.stderr));
    let live_prompts = prompt_bodies(&std::fs::read_to_string(&live_log).unwrap());
    let mut kept: Vec<PathBuf> = std::fs::read_dir(&positions).expect("positions were kept")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "save"))
        .collect();
    kept.sort();
    assert!(kept.len() >= 20, "a game's worth of positions was kept: {}", kept.len());

    // Every fourth position, and the last: enough to cross a pass-until,
    // a combat prompt and a turn boundary without replaying all of them.
    let mut chosen: Vec<&PathBuf> = kept.iter().step_by(4).collect();
    chosen.push(kept.last().unwrap());
    for position in chosen {
        let log = scratch.path("replay.log");
        let _ = std::fs::remove_file(&log);
        let out = runner()
            .current_dir(repo_root())
            .env("TMPDIR", &scratch.dir)
            .env("CLAUDE_CODE_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
            .env("STUB_CALLS", "/dev/null")
            .args(["--resume", position.to_str().unwrap(), "--p1", "cc", "--p2", "random", "--quiet"])
            .args(["--stop-after", "1", "--log", &log])
            .output()
            .expect("the resumed runner runs");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success() && stdout.lines().any(|l| l.starts_with("DECISION 1 p0 ")),
            "{} replays as one decision of the LLM seat:\n{stdout}\n{}", position.display(),
            String::from_utf8_lossy(&out.stderr));
        let replayed = std::fs::read_to_string(&log).unwrap();
        assert!(!replayed.contains("\tRESUME ("), "no recap is sent to a seat whose memory was saved:\n{replayed}");
        let first = prompt_bodies(&replayed).into_iter().next()
            .unwrap_or_else(|| panic!("{} asked the model something:\n{replayed}", position.display()));
        assert!(live_prompts.contains(&first),
            "{} resumed with a prompt the live game never sent:\n{first}", position.display());
    }
}

/// #921's twin: `capture-claude.py` with a positions directory it cannot
/// write fails the runner's preflight, rather than warning at every model
/// call of a game that keeps nothing.
#[test]
fn a_capture_that_cannot_keep_positions_is_refused_before_the_game() {
    let scratch = Scratch::new("capture-unwritable");
    let save = scratch.path("live.json");
    let out = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .env("CLAUDE_CODE_BIN", repo_root().join("scripts/decision-eval/capture-claude.py"))
        .env("EVAL_REAL_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
        .env("EVAL_SAVE", &save)
        .env("EVAL_POSITIONS", "/dev/null/positions")
        .env("STUB_CALLS", "/dev/null")
        .args(["--p1", "cc", "--p2", "random", "--seed", "3", "--quiet", "--save", &save])
        .output()
        .expect("the runner runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "refused before the game: {stderr}");
    assert!(stderr.contains("not runnable") && stderr.contains("EVAL_POSITIONS '/dev/null/positions' cannot be written"),
        "the capture's own reason reaches the operator: {stderr}");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("DECISION"), "no decision was asked");
}

/// #920's game twin: a save records the history, the answer format and the
/// pass-until row (#931) an LLM seat's system prompt was made under, and a
/// resume under others says
/// so in the words `replay.py` reads; one under the same says nothing.
#[test]
fn a_position_resumed_under_other_prompt_settings_says_so() {
    let scratch = Scratch::new("unlike-live");
    let save = scratch.path("live.json");
    let run = |args: &[&str], env: &[(&str, &str)]| {
        runner()
            .current_dir(repo_root())
            .env("TMPDIR", &scratch.dir)
            .env("CLAUDE_CODE_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
            .env("STUB_CALLS", "/dev/null")
            .env_remove("MTG_LLM_HISTORY")
            .env_remove("MTG_CLAUDE_CODE_ANSWERS")
            .env_remove("MTG_LLM_PASS_UNTIL")
            .envs(env.iter().copied())
            .args(args)
            .output()
            .expect("the runner runs")
    };
    let live = run(&["--p1", "cc", "--p2", "random", "--seed", "3", "--quiet", "--save", &save, "--stop-after", "4"], &[]);
    assert!(live.status.success(), "{}", String::from_utf8_lossy(&live.stderr));
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&save).unwrap()).unwrap();
    assert_eq!((&saved["history"], &saved["text_answers"], &saved["pass_until"]),
        (&serde_json::json!(0), &serde_json::json!(true), &serde_json::json!(true)));

    let resume = ["--resume", save.as_str(), "--p1", "cc", "--p2", "random", "--quiet", "--stop-after", "1"];
    let same = run(&resume, &[]);
    assert!(same.status.success(), "{}", String::from_utf8_lossy(&same.stderr));
    assert!(!String::from_utf8_lossy(&same.stderr).contains("not the live one"), "{}", String::from_utf8_lossy(&same.stderr));
    let other = run(&resume, &[("MTG_LLM_HISTORY", "2"), ("MTG_CLAUDE_CODE_ANSWERS", "schema")]);
    let stderr = String::from_utf8_lossy(&other.stderr);
    let unlike: Vec<&str> = stderr.lines().filter(|l| l.contains(": its system prompt is not the live one: ")).collect();
    assert_eq!(unlike.len(), 2, "{stderr}");
    assert!(unlike.iter().any(|l| l.contains("MTG_LLM_HISTORY=0")) && unlike.iter().any(|l| l.contains("MTG_CLAUDE_CODE_ANSWERS=text")),
        "{stderr}");
    // #931: the pass-until row is in the system prompt and on every
    // priority menu, so a position resumed without it is another question.
    let off = run(&resume, &[("MTG_LLM_PASS_UNTIL", "off")]);
    let stderr = String::from_utf8_lossy(&off.stderr);
    assert!(off.status.success(), "{stderr}");
    let unlike: Vec<&str> = stderr.lines().filter(|l| l.contains(": its system prompt is not the live one: ")).collect();
    assert_eq!(unlike.len(), 1, "{stderr}");
    assert!(unlike[0].contains("MTG_LLM_PASS_UNTIL=on, and is resumed with the pass-until row off"), "{stderr}");
}
