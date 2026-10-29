//! #910, the draft seat's copy: under `MTG_CLAUDE_CODE_DRAFT_ANSWERS=text`
//! a pick's prompt ends with the "Answer with only this JSON object: ..."
//! line, which the pick's `PROMPT` record does not hold. The seat writes
//! it as `[Seat N] FORMAT`, byte for byte what it put under the prompt.
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
    std::fs::write(&bin, format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\ncat > \"{}\"\n\
         printf '{{\"type\":\"result\",\"is_error\":false,\"session_id\":\"s\",\"result\":\"{{\\\\\"pick\\\\\": 1, \\\\\"thoughts\\\\\": \\\\\"t\\\\\"}}\",\"usage\":{{}}}}\\n'\n",
        dir.join("stdin.txt").display())).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var(mtg_player::llm::CLAUDE_CODE_BINARY_ENV, &bin);
    std::env::set_var("MTG_CLAUDE_CODE_DRAFT_ANSWERS", "text");
    let log = dir.join("draft.log");
    mtg_player::game_log::init(log.to_str().unwrap()).unwrap();

    let mut client = mtg_draft_runner::llm_client::DraftLlmClient::new(
        "cc", "Innistrad", None, "Doomed Traveler {W} | Creature — Human Soldier 1/1",
        mtg_draft_runner::llm_client::Table { seat: 3, pod_size: 8, pack_size: 14 });
    client.send_pick_message("Pack 1, Pick 1", 3);

    let sent = std::fs::read_to_string(dir.join("stdin.txt")).unwrap();
    let logged = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let (_, record) = logged.split_once("\t[Seat 3] FORMAT\n").unwrap_or_else(|| panic!("a FORMAT record:\n{logged}"));
    let body: Vec<&str> = record.lines().take_while(|l| !l.starts_with("20")).collect();
    let body = body.join("\n");
    assert!(body.starts_with("Answer with only this JSON object: "), "{body}");
    assert!(body.contains("\"pick\""), "{body}");
    assert!(sent.trim_end().ends_with(body.trim_end()), "the record is what was sent under the prompt:\n{body}\n---\n{sent}");
}
