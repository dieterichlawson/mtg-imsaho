//! #1078: every write to the `--log` file discarded its error, so a full
//! disk cut the log mid-line and the run played on and exited 0 with
//! nothing said. The first failure is now kept for the binary to report
//! at its end.

#[cfg(target_os = "linux")]
#[test]
fn a_log_that_stops_accepting_writes_says_so() {
    use mtg_player::game_log;
    // Opens like any file; every write fails with ENOSPC, as a full disk's.
    game_log::init("/dev/full").expect("/dev/full opens for append");
    assert_eq!(game_log::write_failure(), None, "nothing has been written yet");
    game_log::write(file!(), line!(), "GAME", "a record the disk has no room for");
    let why = game_log::write_failure().expect("the failed write is kept, not discarded");
    assert!(why.contains("/dev/full") && why.contains("No space left"),
        "the failure names the log and the error: {why}");
    // Later records are not written and do not replace the first word.
    game_log::write(file!(), line!(), "GAME", "another");
    assert_eq!(game_log::write_failure().as_deref(), Some(why.as_str()));
}
