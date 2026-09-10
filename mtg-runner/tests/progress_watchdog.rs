//! A game that stops making progress stops.
//!
//! Nothing bounded the cast->cancel->recast cycle. A seat whose answer to an
//! additional-cost prompt is unusable gets the empty set substituted for it;
//! the engine rightly refuses 0 where 1 was required (CR 601.2c), cancels
//! the cast and returns priority; and the same cast is offered again, taken
//! again and cancelled again. 2,180 cancelled casts in 60 seconds, stuck on
//! turn 15, 81,815 log lines, and only the kill ended it. `max_actions` is
//! 50,000, which for a `cc` seat is 50,000 `claude -p` subprocesses spent
//! re-asking one question (issue #462).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A stub `claude` that casts whatever it can and cannot answer the
/// exile-cost prompt — a successful call (`exit 0`, `is_error: false`)
/// whose result carries no structured object, which is what a refusal, a
/// truncation or an answer to the wrong schema all look like. No backend
/// failure anywhere, so nothing keyed off a retry would see it.
fn casts_but_cannot_pay(dir: &PathBuf) -> PathBuf {
    let bin = dir.join("claude");
    std::fs::write(&bin, r##"#!/bin/sh
if [ "$1" = "--version" ]; then echo "9.9.9 (stub)"; exit 0; fi
SCHEMA=""
prev=""
for a in "$@"; do
  if [ "$prev" = "--json-schema" ]; then SCHEMA="$a"; fi
  prev="$a"
done
PROMPT=$(cat)
case "$SCHEMA" in
  *indices*)
    printf '{"type":"result","is_error":false,"result":"I cannot help with that.","session_id":"s","usage":{}}\n'
    exit 0 ;;
esac
PICK=$(printf '%s' "$PROMPT" | tr ',' '\n' | sed -n 's/^ *\([0-9][0-9]*\): Cast .*/\1/p' | head -1)
if [ -z "$PICK" ]; then
  PICK=$(printf '%s' "$PROMPT" | tr ',' '\n' | sed -n 's/^ *\([0-9][0-9]*\): Play .*/\1/p' | head -1)
fi
[ -z "$PICK" ] && PICK=0
printf '{"type":"result","is_error":false,"result":"{\\"thoughts\\":\\"t\\",\\"action\\":%s}","structured_output":{"thoughts":"t","action":%s},"session_id":"s","usage":{}}\n' "$PICK" "$PICK"
exit 0
"##).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_seat_that_cannot_finish_a_cast_does_not_re_announce_it_forever() {
    let dir = std::env::temp_dir().join(format!("mtg-watchdog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = casts_but_cannot_pay(&dir);
    // Stitched Drake's additional cost is "exile a creature from your
    // graveyard": min = max = 1, so an empty answer is refused every time.
    let deck = dir.join("drake.txt");
    std::fs::write(&deck, "14 Island\n4 Mountain\n12 Grizzly Bears\n30 Stitched Drake\n").unwrap();
    let log = dir.join("game.log");

    // The runner has to come back on its own. Given a deadline of our own,
    // so a regression is a failed assertion rather than a suite that hangs
    // — which is exactly what this defect does to an operator.
    let errfile = dir.join("stderr.txt");
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "cc", "--p2", "random", "--seed", "4", "-q", "--check-invariants"])
        .args(["--deck1", deck.to_str().unwrap()])
        .args(["--deck2", "decks/coverage/wg-coverage.txt"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&errfile).unwrap())
        .spawn()
        .expect("the runner runs");

    let deadline = started + std::time::Duration::from_secs(90);
    let status = loop {
        match child.try_wait().expect("wait") {
            Some(status) => break Some(status),
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    };
    let took = started.elapsed();
    let stderr = std::fs::read_to_string(&errfile).unwrap_or_default();
    let logged = std::fs::read_to_string(&log).unwrap_or_default();
    let cancels = logged.matches("cast cancelled").count();
    let _ = std::fs::remove_dir_all(&dir);

    let status = status.unwrap_or_else(|| panic!(
        "the game never stopped: {cancels} cancelled casts in {took:?} and still going. \
         For a real seat every one of those is a subprocess."));

    // The cycle really happened — otherwise this test proves nothing.
    assert!(cancels > 10, "the repro reached the cast/cancel cycle (saw {cancels})");

    // And it ended on its own, promptly.
    assert_eq!(status.code(), Some(1),
        "a stalled game is a clean stop, not a crash and not a win.\nstderr: {stderr}");
    assert!(cancels < 500,
        "bounded well short of the 50,000-action ceiling — for a real seat \
         each of these is a subprocess (saw {cancels})");

    // The message says which seat, which prompt, and what the game last did,
    // so a reader does not have to go looking.
    assert!(stderr.contains("stopped making progress"), "stderr: {stderr}");
    assert!(stderr.contains("(p0)"), "it names the seat: {stderr}");
    assert!(stderr.contains("Exile-choice rejected"),
        "and what the game kept doing: {stderr}");
    assert!(!stderr.contains("panicked"), "no panic: {stderr}");
}
