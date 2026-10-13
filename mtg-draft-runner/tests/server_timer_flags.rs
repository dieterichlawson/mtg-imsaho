//! The pick and build timers' flags take 1 second to a day (#812).
//!
//! Any u64 was accepted: 18446744073709551615 overflowed the deadline and
//! panicked inside a person's connection while it held the lobby, which
//! wedged the table; 0 built every person's deck the moment their draft
//! ended, so there was no build phase at all.

use std::process::Command;

fn server(args: &[&str]) -> (bool, String) {
    // --help after the flag: the flags are parsed in order, so a refused
    // value stops the server before --help prints and nothing is dealt.
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"))
        .args(args).arg("--help").output().expect("the server runs");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn a_timer_outside_one_second_to_a_day_is_refused() {
    for flag in ["--pick-seconds", "--build-seconds"] {
        for bad in ["0", "86401", "18446744073709551615"] {
            let (ok, stderr) = server(&[flag, bad]);
            assert!(!ok, "{flag} {bad} is refused");
            assert!(stderr.contains(&format!("{flag}: a timer runs 1 to 86400 seconds")), "{flag} {bad}: {stderr}");
        }
        for good in ["1", "86400"] {
            let (ok, stderr) = server(&[flag, good]);
            assert!(ok, "{flag} {good} is taken: {stderr}");
        }
    }
}
