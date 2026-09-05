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

    // And it is the *final* position: saves are written before each
    // decision, so the last one written during play was a state one action
    // from the end. Resuming a finished game replays no actions.
    let resumed = runner()
        .current_dir(repo_root())
        .env("TMPDIR", &scratch.dir)
        .args(["--resume", &save, "--p1", "random", "--p2", "random", "--quiet"])
        .output()
        .expect("failed to run the runner");
    let text = String::from_utf8_lossy(&resumed.stdout);
    assert!(text.contains("Game over!"), "resumed a finished game: {text}");
    assert!(text.contains("Total actions: 0"), "nothing was left to play: {text}");
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
