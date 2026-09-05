//! What the runner does to the files it writes.
//!
//! The `--save` file the operator named had two ways of surprising them: it
//! replaced a symlink instead of writing through it (#215), and it was
//! unlinked at game over even when the path held something the runner never
//! created (#237, #242).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

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
