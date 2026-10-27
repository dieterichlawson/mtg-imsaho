//! A resumed table says what its AI seats were asked under (#948, #949).
//!
//! The batch runner's snapshot recorded each seat's guide (#579); the
//! hosted table's recorded none, nor `MTG_LLM_HISTORY` or the answer
//! format, and `mtg-draft-server --resume` read `--guide` afresh: a table
//! started with a guide and resumed without one drafted the rest under
//! another system prompt, and nothing said so.
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::Server;
use mtg_draft_runner::lobby::{SeatKind, TablePrompt};

#[test]
fn a_table_resumed_without_its_guide_says_so() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let files = std::env::temp_dir().join(format!("mtg-lobby-guide-resume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&files);
    std::fs::create_dir_all(&files).unwrap();
    let guide = files.join("guide.md");
    std::fs::write(&guide, "Take red cards.\n").unwrap();
    let (guide, save) = (guide.to_str().unwrap().to_string(), files.join("table.save.json"));
    let save = save.to_str().unwrap().to_string();

    // The host's terminal seat waits; the AI seat takes its first pick.
    let first = Server::start("guide-resume-a", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--guide", &guide, "--save", &save]);
    let deadline = Instant::now() + Duration::from_secs(60);
    let saved: serde_json::Value = loop {
        let text = std::fs::read_to_string(&save).unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v["picks"].as_array().is_some_and(|p| !p.is_empty()) {
                break v;
            }
        }
        assert!(Instant::now() < deadline, "the AI seat's pick never reached the snapshot:\n{}", first.stderr());
        std::thread::sleep(Duration::from_millis(100));
    };
    drop(first);
    assert_eq!(saved["prompt"]["guide_path"].as_str(), Some(guide.as_str()), "the guide is recorded: {saved}");
    assert!(saved["prompt"]["guide_hash"].as_str().is_some_and(|h| h.len() == 16), "and its contents: {saved}");

    let resumed = files.join("resumed.save.json");
    let second = Server::start("guide-resume-b", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--resume", &save, "--save", resumed.to_str().unwrap()]);
    let said = format!("made under guide {guide}, and the rest of this draft will be made under no guide");
    let stderr = second.wait_for_stderr("WARN --resume", Duration::from_secs(30));
    assert!(stderr.contains(&said) && stderr.contains("its system prompt is not the live one"),
        "the terminal names both, in mtg-draft-replay's words:\n{stderr}");
    let log = second.wait_for_log("WARN --resume", Duration::from_secs(30));
    assert!(log.contains(&said), "and the log, which outlives it:\n{log}");
    drop(second);

    // A snapshot from before the record says what it cannot know.
    let mut old = saved.clone();
    old.as_object_mut().unwrap().remove("prompt");
    let old_path = files.join("old.save.json");
    std::fs::write(&old_path, old.to_string()).unwrap();
    let third = Server::start("guide-resume-c", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--resume", old_path.to_str().unwrap(),
          "--save", files.join("old-resumed.save.json").to_str().unwrap()]);
    let stderr = third.wait_for_stderr("predates the guide and settings record", Duration::from_secs(30));
    assert!(!stderr.contains("WARN --resume"), "{stderr}");
    drop(third);
    let _ = std::fs::remove_dir_all(&files);
}

/// The settings besides the guide, said in `mtg-draft-replay`'s words; and
/// a table with no AI seat has nothing to say about any of them.
#[test]
fn a_table_prompt_names_each_setting_that_moved() {
    let ai = [SeatKind::Human, SeatKind::Ai("cc".into())];
    let live = TablePrompt::live(&ai, None, None);
    let mut was = live.clone();
    was.settings.history = Some(live.settings.history.unwrap() + 2);
    was.settings.text_answers = live.settings.text_answers.map(|t| !t);
    let said = was.unlike(&live);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].contains("MTG_LLM_HISTORY="), "{said:?}");
    assert!(said[1].contains("MTG_CLAUDE_CODE_DRAFT_ANSWERS="), "{said:?}");
    assert!(live.unlike(&live).is_empty(), "the same settings say nothing");

    let people = [SeatKind::Human, SeatKind::Human];
    let mut was = TablePrompt::live(&people, Some("g.md"), Some("0123456789abcdef"));
    was.settings.history = Some(5);
    assert!(was.unlike(&TablePrompt::live(&people, None, None)).is_empty(),
        "no AI seat is asked anything: {:?}", was.unlike(&TablePrompt::live(&people, None, None)));
}
