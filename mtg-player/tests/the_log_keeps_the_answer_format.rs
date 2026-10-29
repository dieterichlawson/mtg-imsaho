//! #910: a text-answering `claude -p` seat ends every prompt with the
//! "Answer with only this JSON object: ..." line and its fields, after the
//! `PROMPT` record is written. The log held none of it, so a re-ask could
//! not be traced to the line that invited it. The seat writes it as its
//! own `FORMAT` record, byte for byte what it put under the prompt.
//!
//! Its own test binary: the game log is one per process.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

#[test]
fn a_text_answering_seat_logs_the_format_line_it_sends() {
    let dir = std::env::temp_dir().join(format!("mtg-format-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("claude");
    std::fs::write(&bin, format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 9.9.9; exit 0; fi\ncat > \"{}\"\n\
         printf '{{\"type\":\"result\",\"is_error\":false,\"session_id\":\"s\",\"result\":\"{{\\\\\"word\\\\\": \\\\\"ok\\\\\", \\\\\"thoughts\\\\\": \\\\\"t\\\\\"}}\",\"usage\":{{}}}}\\n'\n",
        dir.join("stdin.txt").display())).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let log = dir.join("game.log");
    mtg_player::game_log::init(log.to_str().unwrap()).unwrap();

    let mut p = mtg_player::llm::LlmPlayer::new_claude_code_with_binary("Seat1", bin.to_str().unwrap())
        .with_text_answers(true);
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"word": {"type": "string", "description": "A word"}, "thoughts": {"type": "string"}},
        "required": ["word", "thoughts"]
    });
    let got = p.backend_send_with_schema_for_test("choose", &schema);
    assert_eq!(got["word"], "ok", "{got}");

    let sent = std::fs::read_to_string(dir.join("stdin.txt")).unwrap();
    let logged = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let (_, record) = logged.split_once("\tFORMAT [Seat1]\n").unwrap_or_else(|| panic!("a FORMAT record:\n{logged}"));
    // The record's body runs to the next timestamped line.
    let body: Vec<&str> = record.lines().take_while(|l| !l.starts_with("20")).collect();
    let body = body.join("\n");
    assert!(body.starts_with("Answer with only this JSON object: "), "{body}");
    assert!(sent.trim_end().ends_with(body.trim_end()), "the record is what was sent under the prompt:\n{body}\n---\n{sent}");
}
