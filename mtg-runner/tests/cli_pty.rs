//! End-to-end smoke tests for the CLI harness, under a real pseudo-terminal.
//!
//! The engine has thousands of tests; the interactive harness had none —
//! the 2026-09-02 playtest queue put ~30 of its 43 bugs in mtg-player/cli
//! and mtg-runner, all found nightly by LLM crews with nothing in CI to
//! stop a regression in between. These tests drive the real binary through
//! a pty (openpty + TIOCSCTTY, so crossterm sees a genuine terminal),
//! send keystrokes, and assert on the raw output stream.
//!
//! Zero API cost: the seats are `cli` and `random` — the LLM player is
//! only constructed for `claude`/`gemini` specs and is never touched here.
//!
//! Assertions grep the raw byte stream (ANSI sequences included): the TUI
//! prints each label as one contiguous `Print(..)`, so menu text appears
//! as contiguous substrings. That makes these smoke tests — "the contract
//! holds and the game stays responsive" — not pixel tests.
#![cfg(unix)]

use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A game running under a pty: write keystrokes to `master`, read the
/// screen stream back from it.
struct PtyGame {
    master: std::fs::File,
    child: Child,
    /// Everything read so far — expectations search the whole history, so
    /// a race between two prompts can't lose an assertion.
    seen: String,
}

impl PtyGame {
    fn spawn(args: &[&str]) -> PtyGame {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        ws.ws_col = 150;
        ws.ws_row = 40;
        let rc = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut ws,
            )
        };
        assert_eq!(rc, 0, "openpty failed");

        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mtg-runner"));
        // Deck paths are workspace-relative; the test's own cwd is the
        // package directory.
        cmd.args(args)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
        unsafe {
            // Each stdio dups the slave end; the child leads its own session
            // with the pty as its controlling terminal, exactly like a run
            // from a real terminal.
            cmd.stdin(Stdio::from_raw_fd(libc::dup(slave)))
                .stdout(Stdio::from_raw_fd(libc::dup(slave)))
                .stderr(Stdio::from_raw_fd(libc::dup(slave)))
                .pre_exec(|| {
                    libc::setsid();
                    libc::ioctl(0, libc::TIOCSCTTY, 0);
                    Ok(())
                });
        }
        let child = cmd.spawn().expect("failed to spawn mtg-runner under pty");
        unsafe {
            libc::close(slave);
        }
        let master = unsafe { std::fs::File::from_raw_fd(master) };
        PtyGame { master, child, seen: String::new() }
    }

    fn send(&mut self, keys: &str) {
        self.master
            .write_all(keys.as_bytes())
            .expect("write to pty failed");
        self.master.flush().expect("flush to pty failed");
    }

    /// Send a response to a prompt that was just expected: let the prompt's
    /// reader arm first (there is a small window between the prompt text
    /// painting and raw mode + the event reader engaging, and keystrokes
    /// landing inside it are deliberately dropped as type-ahead, #71).
    fn answer(&mut self, keys: &str) {
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            self.pump(Duration::from_millis(100));
        }
        self.send(keys);
    }

    /// The stream with ANSI escape sequences removed: a styled menu row is
    /// several Print calls with style bytes in between ("  2" bold, then
    /// ": Concede"), so needles must match the visible text, not the raw
    /// bytes.
    fn stripped(&self) -> String {
        let mut out = String::new();
        let mut chars = self.seen.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                out.push(c);
                continue;
            }
            match chars.peek() {
                // CSI: ESC [ ... final byte in @..=~
                Some('[') => {
                    chars.next();
                    for d in chars.by_ref() {
                        if ('@'..='~').contains(&d) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] ... BEL
                Some(']') => {
                    chars.next();
                    for d in chars.by_ref() {
                        if d == '\x07' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Pump the master for up to `timeout`, returning as soon as the
    /// accumulated stream's visible text contains `needle`.
    fn expect(&mut self, needle: &str, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            let text = self.stripped();
            if text.contains(needle) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {needle:?};\nlast 2000 visible chars:\n{}",
                &text[text.len().saturating_sub(2000)..]
            );
            self.pump(Duration::from_millis(100));
        }
    }

    /// Read whatever arrives within `window` into `seen`.
    fn pump(&mut self, window: Duration) {
        use std::os::fd::AsRawFd;
        let mut pfd = libc::pollfd {
            fd: self.master.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ms = libc::c_int::try_from(window.as_millis()).unwrap_or(100);
        let n = unsafe { libc::poll(&raw mut pfd, 1, ms) };
        if n <= 0 {
            return;
        }
        let mut chunk = [0u8; 8192];
        // EIO here means the child hung up — the callers' deadlines handle it.
        if let Ok(got) = self.master.read(&mut chunk) {
            self.seen.push_str(&String::from_utf8_lossy(&chunk[..got]));
        }
    }

    /// Drain quietly for `window`, then assert the stream does NOT contain
    /// `needle` — for "this input must have done nothing" checks.
    fn expect_absent(&mut self, needle: &str, window: Duration) {
        let deadline = Instant::now() + window;
        while Instant::now() < deadline {
            self.pump(Duration::from_millis(100));
        }
        let text = self.stripped();
        assert!(
            !text.contains(needle),
            "{needle:?} appeared but must not have;\nlast 2000 visible chars:\n{}",
            &text[text.len().saturating_sub(2000)..]
        );
    }

    fn wait_exit(&mut self, timeout: Duration) -> std::process::ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("wait failed") {
                return status;
            }
            assert!(Instant::now() < deadline, "child did not exit in time");
            self.pump(Duration::from_millis(100));
        }
    }
}

impl Drop for PtyGame {
    fn drop(&mut self) {
        // A failing assertion must not leave the suite hanging on a live
        // game.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn seeded_game() -> PtyGame {
    PtyGame::spawn(&[
        "--p1", "cli", "--p2", "random",
        "--deck1", "decks/rb-vampires.txt", "--deck2", "decks/gw-humans.txt",
        "--seed", "2301", "--on-the-play", "1", "--quiet",
    ])
}

const T: Duration = Duration::from_secs(30);

/// Ctrl-C has two clean exit paths: read inside a raw-mode prompt it is a
/// keystroke (exit 0); landing between prompts, the cooked line discipline
/// turns it into SIGINT and the #78 restore-terminal handler exits 130.
/// Both restore the terminal; both are clean.
#[track_caller]
fn assert_clean_exit(g: &mut PtyGame) {
    let status = g.wait_exit(T);
    assert!(matches!(status.code(), Some(0) | Some(130)),
        "expected a clean Ctrl-C exit (0 or 130), got {status:?}");
}

/// The core interactive loop: boot to the mulligan prompt, keep, see the
/// seat-identified turn header (#115), get junk input rejected visibly
/// (#76's rule), open and close an info pane (#101), and quit cleanly on
/// Ctrl-C with the terminal restored path exercised.
#[test]
fn boots_answers_prompts_and_recovers_from_junk() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("keeps (0 mulligans)", T);
    // Seat identity in the turn bar (#115) and a live action menu.
    g.expect("you are p0", T);
    g.expect("Pass priority", T);

    // Junk input is rejected with a visible notice, and the menu survives.
    g.answer("zz\r");
    g.expect("Invalid input", T);

    // Info pane opens and returns (l = full log view, #101's pager).
    g.answer("l\r");
    g.expect("GAME LOG", T);
    g.answer("\r");
    g.expect("[enter=pass]", T);

    // Ctrl-C exits promptly and cleanly.
    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// A multi-line paste must never answer prompts (#50/#106): pasted at the
/// mulligan decision, its embedded newlines must not keep, mulligan, or
/// leak into later prompts. The explicit keystroke afterwards still works.
#[test]
fn a_bracketed_paste_never_submits_a_decision() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    // Paste "1\n0\n" as one bracketed paste: line one would mulligan, line
    // two would then keep the smaller hand — if any of it executed.
    g.answer("\x1b[200~1\r0\r\x1b[201~");
    g.expect_absent("mulligans to", Duration::from_secs(2));
    g.expect_absent("keeps (", Duration::from_secs(1));

    // Clear the pasted first line from the buffer, then answer for real.
    g.send("\x15"); // Ctrl-U
    g.answer("0\r");
    g.expect("keeps (0 mulligans)", T);

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// The concede confirmation declines safely: 'n' followed by Enter returns
/// to the same priority window without the trailing Enter passing priority
/// (#127), and junk keys re-prompt without ending the game (#42/#125).
#[test]
fn declining_a_concede_costs_nothing() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("Pass priority", T);
    // Concede is always the last entry; on this seed's first main-phase
    // menu it is entry 2 (pass / play land / concede — pinned by seed 2301
    // + --on-the-play 1).
    g.expect("2: Concede", T);
    g.answer("2\r");
    g.expect("Are you sure", T);
    g.answer("q"); // junk: must re-prompt, not concede
    g.expect("Please answer y or n", T);
    g.answer("n\r"); // decline, line-style
    // Back at the same window: the land play is still on offer, and the
    // game has not advanced past our turn or ended.
    g.expect("1: Play land", T);
    g.expect_absent("Game over", Duration::from_secs(1));

    g.send("\x03");
    assert_clean_exit(&mut g);
}
