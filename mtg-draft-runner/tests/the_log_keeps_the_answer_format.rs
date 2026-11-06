//! #910, the draft seat's copy: under `MTG_CLAUDE_CODE_DRAFT_ANSWERS=text`
//! a pick's prompt ends with the "Answer with only this JSON object: ..."
//! line, which the pick's `PROMPT` record does not hold. The seat writes
//! it as `[Seat N] FORMAT`, byte for byte what it put under the prompt;
//! and a pick asked again writes the sentence the re-ask added.
//!
//! Its own test binary: the game log and the environment are per process.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

#[test]
fn a_text_answering_draft_seat_logs_the_format_line_it_sends() {
    let dir = std::env::temp_dir().join(format!("mtg-draft-format-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("claude");
    // The first answer is out of range, so the pick is asked again; the
    // second is taken. Each call's stdin is kept.
    std::fs::write(&bin, format!(
        "#!/usr/bin/env python3\nimport json, os, sys\nif sys.argv[1:2] == ['--version']: print('9.9.9'); sys.exit(0)\n\
         d = '{}'\nn = len([f for f in os.listdir(d) if f.startswith('stdin')])\n\
         open(os.path.join(d, 'stdin%d.txt' % n), 'w').write(sys.stdin.read())\n\
         print(json.dumps({{'type': 'result', 'is_error': False, 'session_id': 's', 'usage': {{}},\n\
           'result': json.dumps({{'pick': 99 if n == 0 else 1, 'thoughts': 't'}})}}))\n",
        dir.display())).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var(mtg_player::llm::CLAUDE_CODE_BINARY_ENV, &bin);
    std::env::set_var("MTG_CLAUDE_CODE_DRAFT_ANSWERS", "text");
    let log = dir.join("draft.log");
    mtg_player::game_log::init(log.to_str().unwrap()).unwrap();

    let mut client = mtg_draft_runner::llm_client::DraftLlmClient::new(
        "cc", "Innistrad", None, "Doomed Traveler {W} | Creature — Human Soldier 1/1",
        mtg_draft_runner::llm_client::Table { seat: 3, pod_size: 8, pack_size: 14 });
    client.send_pick_message("Pack 1, Pick 1", 3);

    let sent = std::fs::read_to_string(dir.join("stdin0.txt")).unwrap();
    let again = std::fs::read_to_string(dir.join("stdin1.txt")).unwrap();
    let logged = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let body = record(&logged, "[Seat 3] FORMAT");
    assert!(body.starts_with("Answer with only this JSON object: "), "{body}");
    assert!(body.contains("\"pick\""), "{body}");
    assert!(sent.trim_end().ends_with(body.trim_end()), "the record is what was sent under the prompt:\n{body}\n---\n{sent}");

    // The re-ask: what it added under the pick's prompt is in the log too,
    // above the format line that follows it.
    let reask = record(&logged, "[Seat 3] REASK_PROMPT");
    assert!(reask.starts_with("Your answer ") && reask.ends_with("Answer again."), "{reask}");
    assert!(again.contains(&format!("Pack 1, Pick 1\n\n{reask}\n\n{body}")), "{reask}\n---\n{again}");
}

/// The body of the first record labelled `label`: its lines up to the next
/// timestamped one.
fn record(logged: &str, label: &str) -> String {
    let (_, rest) = logged.split_once(&format!("\t{label}\n")).unwrap_or_else(|| panic!("a {label} record:\n{logged}"));
    rest.lines().take_while(|l| !l.starts_with("20")).collect::<Vec<_>>().join("\n")
}
