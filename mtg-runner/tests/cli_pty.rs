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
        PtyGame::spawn_sized(150, 40, args)
    }

    /// A game in a terminal of a given size — the CLI's layout is
    /// width-dependent (the right panel, and with it the card search, only
    /// exists at 100 columns or more), so a test about a narrow terminal
    /// has to ask for one.
    fn spawn_sized(cols: u16, rows: u16, args: &[&str]) -> PtyGame {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        ws.ws_col = cols;
        ws.ws_row = rows;
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

    /// `expect` without the assertion: did `needle` arrive inside
    /// `timeout`? For a key that may legitimately have been dropped —
    /// anything typed before a prompt's reader arms is discarded as
    /// type-ahead (#71), and under a loaded machine that window is wider
    /// than the pause `answer` takes.
    fn expect_within(&mut self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.stripped().contains(needle) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            self.pump(Duration::from_millis(100));
        }
    }

    /// Send `keys` until `needle` shows up, for a keystroke that has no
    /// visible echo of its own to wait on.
    #[track_caller]
    fn answer_until(&mut self, keys: &str, needle: &str, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            self.answer(keys);
            if self.expect_within(needle, Duration::from_secs(3)) {
                return;
            }
            let text = self.stripped();
            assert!(
                Instant::now() < deadline,
                "{keys:?} never produced {needle:?};\nlast 2000 visible chars:\n{}",
                &text[text.len().saturating_sub(2000)..]
            );
        }
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

    /// Answer the numbered menu row whose label starts with `needle`, by its
    /// index. A test that hardcodes indices stops testing what it meant to
    /// the first time a row is added above the one it wanted.
    fn answer_option(&mut self, needle: &str, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(idx) = menu_index(&self.stripped(), needle) {
                self.answer(&format!("{idx}\r"));
                self.forget();
                return;
            }
            let text = self.stripped();
            assert!(
                Instant::now() < deadline,
                "no menu row labelled {needle:?} appeared;\nlast 2000 visible chars:\n{}",
                &text[text.len().saturating_sub(2000)..]
            );
            self.pump(Duration::from_millis(100));
        }
    }

    /// Forget everything read so far, so the next `expect` searches only
    /// what arrives from here on. Needed to assert that something is drawn
    /// *again* — `expect` searches the whole history, and a header that was
    /// on screen a moment ago would satisfy it whether or not it survived.
    fn forget(&mut self) {
        self.seen.clear();
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

/// The index of the most recently drawn menu row whose label starts with
/// `needle`, in a `  3: Cast Walking Corpse (tap 2x Swamp (your))` row.
///
/// The screen is painted with cursor moves rather than newlines, so the
/// visible text is one long line and a row is found by its `N: label`, not
/// by splitting.
fn menu_index(text: &str, needle: &str) -> Option<usize> {
    let tag = format!(": {needle}");
    let bytes = text.as_bytes();
    let mut found = text.rfind(&tag);
    while let Some(i) = found {
        let mut start = i;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start < i {
            return text[start..i].parse::<usize>().ok();
        }
        found = text[..i].rfind(&tag);
    }
    None
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

/// Issue #327: opening the `/` card search redrew the frame with the menu's
/// header dropped, so the rule above the option list went blank. At a
/// priority menu that costs the step name; at a mandatory prompt with no
/// Pass option it costs the question itself, because the header is the only
/// thing on screen saying what the numbered rows are for. The overlay
/// changes what is in the CARDS gutter, not what the game is asking.
#[test]
fn the_card_search_keeps_the_header_of_the_prompt_under_it() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("Pass priority", T);
    g.expect("MAIN PHASE", T);

    // From here on, only the search overlay's own repaint counts: the header
    // was on screen a moment ago, and that is not what is being asked.
    g.forget();
    g.answer("/");
    g.expect("MAIN PHASE", T);

    g.send("\x1b");
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
    g.answer("q\r"); // junk: must re-prompt, not concede
    g.expect("Please answer y or n", T);
    g.answer("n\r"); // decline, line-style
    // Back at the same window: the land play is still on offer, and the
    // game has not advanced past our turn or ended.
    g.expect("1: Play land", T);
    g.expect_absent("Game over", Duration::from_secs(1));

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// Issue #249: the confirmation is line-buffered like every other prompt, so
/// a word the player is still typing cannot end the game on its third
/// character. "maybe" contains a 'y'.
#[test]
fn typing_a_word_containing_y_does_not_concede() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("Pass priority", T);
    g.expect("2: Concede", T);
    g.answer("2\r");
    g.expect("Are you sure", T);

    // No Enter: nothing has been answered yet.
    g.send("maybe");
    g.expect_absent("Game over", Duration::from_secs(1));

    // And submitting it is junk, not a concession.
    g.answer("\r");
    g.expect("Please answer y or n", T);
    g.answer("n\r");
    g.expect("1: Play land", T);
    g.expect_absent("Game over", Duration::from_secs(1));

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// Issue #355: the prompts erased to the right edge of the TERMINAL.
///
/// Every prompt this CLI reads is drawn inside the middle panel, and #53,
/// #109 and #320 bounded what those prompts *print* to that panel's border.
/// What they *erase* was never bounded: a dozen sites cleared with
/// `Clear(ClearType::UntilNewLine)`, which runs from the cursor through the
/// frame's border and on across the CARDS pane beside it. Whichever line of
/// card text shared the row with the input line was blanked, on every
/// frame, at every prompt, before a key was pressed — sometimes a card's
/// name-and-cost line, so the entry below it rendered as a bare type line
/// with no name at all, and sometimes an oracle line, so Moonmist read as
/// preventing ALL combat damage with its Werewolf exception invisible.
///
/// `ESC [ K` is that erase and can only ever reach the terminal's edge, so
/// its absence from the stream is the property, not a proxy for it: this
/// CLI never erases past the panel it is drawing in. A bounded clear writes
/// spaces over exactly the columns it owns instead.
#[test]
fn no_prompt_ever_erases_past_the_panel_it_is_drawn_in() {
    let mut g = seeded_game();

    // The opening frame alone reproduced it — the CARDS pane is populated
    // from the opening hand and the mulligan prompt is drawn over it.
    g.expect("Keep opening hand", T);
    assert_no_line_erase(&g, "the opening mulligan frame");

    // A rejected entry repaints the notice row and re-clears the input row
    // (#35, #291), and each typed character repaints the input line (#281).
    g.answer("zz\r");
    g.expect("Invalid input", T);
    g.answer("0\r");
    g.expect("keeps (0 mulligans)", T);
    g.expect("Pass priority", T);
    g.send("12");
    g.pump(Duration::from_millis(300));
    g.answer("\r");
    g.pump(Duration::from_millis(300));
    assert_no_line_erase(&g, "the mulligan, notice and priority prompts");

    g.send("\x03");
    assert_clean_exit(&mut g);
}

#[track_caller]
fn assert_no_line_erase(g: &PtyGame, what: &str) {
    // CSI K, in all the spellings crossterm could emit for "erase in line":
    // a bare ESC [ K and its explicit parameter forms.
    for seq in ["\x1b[K", "\x1b[0K", "\x1b[1K", "\x1b[2K"] {
        assert!(
            !g.seen.contains(seq),
            "{what} emitted {seq:?}, an erase that runs to the terminal's \
             right edge and through the CARDS pane (#355); clear the \
             columns the panel owns instead"
        );
    }
}

/// A deck that reaches combat on a fixed line: nothing but Swamps and a
/// two-mana 2/2, so the route to a declare-attackers prompt is the same
/// every run and needs no card the seed has to cooperate about.
fn swamps_and_zombies() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mtg-cli-pty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("swamps-and-zombies.txt");
    std::fs::write(&path, "30 Swamp\n30 Walking Corpse\n").expect("write deck");
    path
}

/// Issue #360: a keystroke typed at a combat prompt was executed by the
/// NEXT priority menu.
///
/// #71's rule is that a keystroke must never answer a prompt the player has
/// not been shown, and `LAST_DECISION_IDENTITY` enforces it by draining
/// type-ahead whenever the decision changes identity. Only the priority
/// menu ever registered itself there, so the two combat prompts — and the
/// specialised prompts reached by an early return out of `choose_action` —
/// were invisible to it: the menu after combat compared itself against the
/// menu *before* combat, saw no change and skipped the drain. In a hotseat
/// game that combat prompt belongs to the other seat, so `0:0` ⏎ `4` ⏎ at
/// p1's declare-blockers prompt opened p0's concede dialog.
///
/// Driven here in the same-seat direction, which needs one seat and reaches
/// the same reader: `all` ⏎ `9` ⏎ in one burst at the declare-attackers
/// prompt. `9` is off the end of the menu that follows, so if it survives
/// the boundary it is refused *visibly* — which makes the leak assertable
/// without taking an irreversible action to detect it.
#[test]
fn a_key_typed_at_a_combat_prompt_does_not_reach_the_next_menu() {
    let deck = swamps_and_zombies();
    let deck = deck.to_str().expect("utf-8 temp path");
    let mut g = PtyGame::spawn(&[
        "--p1", "cli", "--p2", "random",
        "--deck1", deck, "--deck2", deck,
        "--seed", "2301", "--on-the-play", "1", "--quiet",
    ]);

    g.expect("Keep opening hand", T);
    g.answer("0\r");

    // Land, then auto-pass to our next turn; land and a 2/2, then auto-pass
    // again; land, and pass into combat, which is where the 2/2 attacks.
    g.expect("MAIN PHASE 1", T);
    g.answer_option("Play land", T);
    g.expect("Pass priority", T);
    g.answer("f\r");

    g.expect("Play land", T);
    g.answer_option("Play land", T);
    g.expect("Cast Walking Corpse", T);
    g.answer_option("Cast Walking Corpse", T);
    g.expect("Pass priority", T);
    g.answer("f\r");

    // The last hop is an explicit pass rather than another `f`: under
    // auto-pass the menus after combat return without reading, and what the
    // first menu that DOES read is handed is the whole point here.
    g.expect("Play land", T);
    g.answer_option("Play land", T);
    g.expect("Pass priority", T);
    g.answer_option("Pass priority", T);

    g.expect("DECLARE ATTACKERS", T);
    g.expect("Attack (numbers/all/none", T);
    g.forget();

    // One burst, no pause: the declaration and a key that belongs to nothing.
    g.answer("all\r9\r");

    // Then make one more decision of our own and wait for its answer. Keys
    // are read in order, so once the confirmation this seat asked for is on
    // screen, a `9` that survived the combat boundary has already been read
    // and refused, and its refusal is already in the history behind it.
    // That makes this an ordering rather than a race with a redraw.
    g.expect("MAIN PHASE 2", T);
    g.answer_option("Concede", T);
    g.expect("Are you sure", T);
    g.expect_absent("Invalid input", Duration::from_millis(200));

    // And the seat is still playing: declining leaves the menu as it was.
    g.answer("n\r");
    g.expect("Pass priority", T);

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// Issue #361: the concede confirmation was the one reader in the file with
/// no drain at all — neither `read_line_redrawing`'s unconditional one nor
/// the identity check — so a `y` already queued when Concede was picked
/// answered a dialog that was never drawn. `2` ⏎ `y` ⏎ in one burst ended
/// the game with the "Are you sure?" row never on screen, which is the
/// exact accident the confirmation exists to prevent (#42, #125, #127,
/// #249).
#[test]
fn a_queued_y_cannot_answer_a_concede_dialog_that_was_never_drawn() {
    let mut g = seeded_game();

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("Pass priority", T);
    g.expect("2: Concede", T);

    // One burst: the menu index and an answer to the question it raises,
    // with no pause in between and nothing drawn between them.
    g.answer("2\ry\r");

    // The confirmation is asked, and is still waiting.
    g.expect("Are you sure", T);
    g.expect_absent("Game over", Duration::from_secs(2));

    // And it still answers normally, to a key typed after it was seen.
    g.answer("n\r");
    g.expect("1: Play land", T);
    g.expect_absent("Game over", Duration::from_secs(1));

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// Issue #366: `/` was the one key in this CLI that produced no reaction of
/// any kind.
///
/// The card search box is part of the right panel, which only exists at 100
/// columns or more. #107 stopped it entering an invisible modal mode on a
/// narrower terminal — but the key was still intercepted, and the caller
/// then repainted an identical frame. No box, no message, and not even the
/// `Invalid input` every other unusable key gets. A silent re-render is
/// indistinguishable from a hung game (#76); this was the last place that
/// still was one.
#[test]
fn the_card_search_says_why_it_cannot_open_on_a_narrow_terminal() {
    let mut g = PtyGame::spawn_sized(80, 24, &[
        "--p1", "cli", "--p2", "random",
        "--deck1", "decks/rb-vampires.txt", "--deck2", "decks/gw-humans.txt",
        "--seed", "2301", "--on-the-play", "1", "--quiet",
    ]);

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("Pass priority", T);
    // At this width the hint line does not offer it, which #107 got right.
    // Checked against the whole history rather than a fresh frame: the menu
    // is already painted and nothing repaints it until a key is pressed, so
    // forgetting first would wait for a frame that is not coming.
    g.expect("[d=deck]", T);
    g.expect_absent("[/=search]", Duration::from_millis(200));

    // Pressing it anyway says why, instead of doing nothing at all.
    g.answer_until("/", "Card search needs a terminal at least 100 columns wide", T);

    g.send("\x03");
    assert_clean_exit(&mut g);
}

/// Issue #357: no pane in the game ever printed a permanent's color, and
/// intimidate (CR 702.13a) is decided entirely by it.
///
/// The `i` inspector's detail page is where a player looks at one
/// permanent's characteristics, and it listed type, keywords, P/T,
/// controller, tapped and id — everything but the one that decides whether
/// a creature may block. For a face with no mana cost, whose color CR 204.2
/// states with an indicator instead, it was not merely unhighlighted but
/// genuinely unobtainable: a defender facing a Gatstaf Howler could learn
/// it was green only by reading back which of their own creatures the
/// engine had already allowed to block it.
#[test]
fn the_inspector_names_a_permanents_color() {
    let deck = swamps_and_zombies();
    let deck = deck.to_str().expect("utf-8 temp path");
    let mut g = PtyGame::spawn(&[
        "--p1", "cli", "--p2", "random",
        "--deck1", deck, "--deck2", deck,
        "--seed", "2301", "--on-the-play", "1", "--quiet",
    ]);

    g.expect("Keep opening hand", T);
    g.answer("0\r");
    g.expect("MAIN PHASE 1", T);
    g.answer_option("Play land", T);
    g.expect("Pass priority", T);
    g.answer("f\r");
    g.expect("Play land", T);
    g.answer_option("Play land", T);
    g.expect("Cast Walking Corpse", T);
    g.answer_option("Cast Walking Corpse", T);
    g.expect("Pass priority", T);

    // Open the inspector on the 2/2 that is now on the battlefield.
    g.forget();
    g.answer("i\r");
    g.expect("INSPECT BATTLEFIELD", T);
    g.answer_option("Walking Corpse", T);
    // Walking Corpse is {1}{B}: black, and the page says so.
    g.expect("Color: Black", T);

    // Back out of the viewer before quitting: the exit path this asserts is
    // the one at a menu, and each screen wants its own Enter.
    g.forget();
    g.answer_until("\r", "Enter number for details", T);
    g.forget();
    g.answer_until("\r", "Pass priority", T);
    g.send("\x03");
    assert_clean_exit(&mut g);
}
