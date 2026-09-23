//! A record's level comes from its label, not from the call site.
//!
//! `LogLevel::Error` is documented as "recoverable errors: malformed LLM
//! responses, API retries that eventually succeeded, fallback activations",
//! and `grep ERROR` over a log is how an operator asks "did this run hit
//! the usage limit?".
//!
//! The level was each call site's own decision, and the sites disagreed.
//! The same `claude -p` `is_error` retry — the shape a usage limit, a bad
//! model name or a refused key arrives in — was written at `Error` by the
//! draft backend and at `Info` by the game backend, so that question was
//! answerable for a draft and not for a game (#583). It was one fix (#399,
//! `deab6417`) that reached one of two copies; four more game-side records
//! had drifted the same way, including an `API_FATAL` written at `Info`
//! immediately before `process::exit(1)`.
#![cfg(unix)]

use mtg_player::game_log::{self, LogLevel};

/// The level column of every record in the log, paired with its label.
fn records(path: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            // <timestamp>\t<LEVEL>\t<thread>\t<file:line>\t<LABEL>\t<content>
            (f.len() >= 5).then(|| (f[4].to_string(), f[1].to_string()))
        })
        .collect()
}

#[test]
fn an_api_record_is_an_error_record_whichever_call_site_writes_it() {
    let dir = std::env::temp_dir().join(format!("mtg-log-levels-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("game.log");
    let path_s = path.to_str().unwrap().to_string();
    game_log::init(&path_s).expect("the log opens");

    // The whole API family, written the way a call site that has not
    // thought about the level writes it.
    for label in ["API_RETRY", "API_WARN", "API_ERROR", "API_FATAL", "MALFORMED"] {
        game_log::write(file!(), line!(), label, "something recoverable happened");
    }
    // And written by a call site that asks for Info explicitly: a record
    // cannot be demoted below what its label is worth, which is what stops
    // one copy of a duplicated request path from drifting from the other.
    game_log::write_at(LogLevel::Info, file!(), line!(), "API_RETRY", "demoted on purpose");
    // A record outside the family is untouched.
    game_log::write(file!(), line!(), "PROMPT", "an ordinary record");

    let got = records(&path_s);
    let offenders: Vec<&(String, String)> = got
        .iter()
        .filter(|(label, level)| {
            (label.starts_with("API_") || label == "MALFORMED") && level != "ERROR"
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "these records are invisible to `grep ERROR`: {offenders:?}"
    );
    assert!(
        got.iter().any(|(l, lv)| l == "PROMPT" && lv == "INFO"),
        "an ordinary record is still INFO: {got:?}"
    );

    // The rule itself, stated once.
    assert_eq!(game_log::level_floor("API_RETRY"), LogLevel::Error);
    assert_eq!(game_log::level_floor("MALFORMED"), LogLevel::Error);
    assert_eq!(game_log::level_floor("PROMPT"), LogLevel::Info);

    let _ = std::fs::remove_dir_all(&dir);
}
