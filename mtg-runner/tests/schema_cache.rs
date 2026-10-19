//! The schema an LLM seat sends is part of its cached prefix, so it must
//! be the same bytes every time the same kind of question is asked.
//!
//! On the Messages API a changed `output_config.format` invalidates the
//! prompt cache, and through `claude -p` the schema is a tool definition
//! that renders ahead of the system prompt. A menu's `enum: [0..n]` and a
//! main-phase plan's card names made nearly every call's schema new: 60 to
//! 95% of a real game's calls read nothing from the cache and wrote the
//! whole system prompt again at 1.25x the input price, where a read costs
//! 0.1x (reports/llm-cost.md, phase 3). `cache_stable_schema` sends the
//! schema without its per-call constraints; this plays stub games and
//! holds every kind of question to one schema.

#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Every property name in `schema`, with its path: what a question asks
/// for, and so what makes two schemas the same kind of question.
fn kind(schema: &serde_json::Value, path: &str, out: &mut BTreeSet<String>) {
    if let Some(props) = schema.get("properties").and_then(serde_json::Value::as_object) {
        for (key, sub) in props {
            let inner = format!("{path}/{key}");
            out.insert(inner.clone());
            kind(sub, &inner, out);
        }
    }
    if let Some(items) = schema.get("items") {
        kind(items, &format!("{path}[]"), out);
    }
}

/// The stub's record of each call of one game: the sent schema and its hash.
fn play(name: &str, args: &[&str]) -> Vec<(BTreeSet<String>, String, serde_json::Value)> {
    let dir = std::env::temp_dir().join(format!("mtg-schema-cache-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let calls = dir.join("calls.jsonl");
    let out = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .current_dir(repo_root())
        .env("TMPDIR", &dir)
        .env("CLAUDE_CODE_BIN", repo_root().join("scripts/llm-prompt-stub.py"))
        .env("STUB_CALLS", &calls)
        .args(["--p1", "cc", "--p2", "random", "--quiet"])
        .args(args)
        .output()
        .expect("the runner runs");
    assert!(out.status.success(), "{name} plays out: {}", String::from_utf8_lossy(&out.stderr));
    let records = std::fs::read_to_string(&calls).expect("the stub recorded its calls");
    let _ = std::fs::remove_dir_all(&dir);
    records.lines().map(|line| {
        let r: serde_json::Value = serde_json::from_str(line).unwrap();
        let mut k = BTreeSet::new();
        kind(&r["schema"], "", &mut k);
        (k, r["schema_sha"].as_str().unwrap().to_string(), r["schema"].clone())
    }).collect()
}

#[test]
fn every_kind_of_question_sends_one_schema_however_the_game_goes() {
    // X spells on both sides, paid from lands, rocks and a dork, so X
    // funding is asked on many different boards (#857: it was keyed by the
    // untapped groups and sent a schema of its own nearly every time).
    let decks = std::env::temp_dir().join(format!("mtg-schema-cache-decks-{}", std::process::id()));
    std::fs::create_dir_all(&decks).unwrap();
    let x_deck = decks.join("x.txt");
    std::fs::write(&x_deck, "8 Mountain\n7 Forest\n1 Kessig Wolf Run\n4 Sol Ring\n4 Devil's Play\n\
        2 Avacyn's Pilgrim\n4 Grizzly Bears\n4 Kalonian Tusker\n2 Lightning Bolt\n2 Brimstone Volley\n2 Garruk Relentless\n").unwrap();
    let x_deck = x_deck.to_str().unwrap().to_string();
    let x_args = ["--deck1", x_deck.as_str(), "--deck2", x_deck.as_str(), "--seed", "6"];
    let games: [(&str, &[&str]); 5] = [
        ("seed1", &["--seed", "1"]),
        ("seed2", &["--seed", "2"]),
        ("seed4", &["--seed", "4"]),
        ("drafted", &["--deck1", "decks/ub-zombies.txt", "--deck2", "decks/ug-spider-spawning.txt", "--seed", "12"]),
        ("x-spells", &x_args),
    ];
    let played: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = games.iter().map(|(name, args)| s.spawn(move || (*name, play(name, args)))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut by_kind: BTreeMap<BTreeSet<String>, BTreeMap<String, serde_json::Value>> = BTreeMap::new();
    let mut calls = 0;
    for (name, records) in &played {
        let distinct: BTreeSet<&String> = records.iter().map(|(_, sha, _)| sha).collect();
        let kinds: BTreeSet<&BTreeSet<String>> = records.iter().map(|(k, _, _)| k).collect();
        eprintln!("{name}: {} calls, {} kinds of question, {} different schemas",
            records.len(), kinds.len(), distinct.len());
        calls += records.len();
        for (k, sha, schema) in records {
            by_kind.entry(k.clone()).or_default().insert(sha.clone(), schema.clone());
        }
    }
    let _ = std::fs::remove_dir_all(&decks);
    assert!(calls >= 60, "the games asked enough to say anything: {calls} calls");
    let funded = played.iter().flat_map(|(_, r)| r).filter(|(k, _, _)| k.contains("/floating")).count();
    assert!(funded >= 3, "X funding was asked often enough to compare: {funded} calls");

    for (k, schemas) in &by_kind {
        assert!(schemas.len() == 1,
            "the question {k:?} went out as {} different schemas; anything that changes \
             from call to call belongs in the prompt, not the schema:\n{}",
            schemas.len(),
            schemas.values().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
        let text = schemas.values().next().unwrap().to_string();
        for constraint in ["\"enum\"", "\"minItems\"", "\"maxItems\""] {
            assert!(!text.contains(constraint), "{k:?} sent a per-call {constraint}: {text}");
        }
    }
}
