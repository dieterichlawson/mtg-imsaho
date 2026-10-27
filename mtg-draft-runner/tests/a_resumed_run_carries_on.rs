//! `--resume` carries a run on from where its snapshot stopped (issue #581).
//!
//! The snapshot used to hold the picks and nothing else, so an interruption
//! after the last pick re-billed every deck build and the whole tournament —
//! about fifteen times the draft's calls at the shipped defaults. It now
//! holds the built decks and every finished match, and a resume takes them
//! instead of paying for them again, and says so on the standings.
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A seat that answers every schema, and counts its calls in `$CALLS`.
const STUB: &str = r##"#!/usr/bin/env python3
import sys, json, os
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
msg = sys.stdin.read(); sid = ""; sc = None
for i, a in enumerate(argv):
    if a in ("--session-id", "--resume") and i + 1 < len(argv): sid = argv[i + 1]
    if a == "--json-schema" and i + 1 < len(argv): sc = argv[i + 1]
# Asked in text (the default) a seat sends no schema; the full one is beside it.
sc = sc or os.environ.get("MTG_LLM_ANSWER_SCHEMA")
with open(os.environ["CALLS"], "a") as f: f.write("x\n")
if os.environ.get("PROMPTS"):
    with open(os.environ["PROMPTS"], "a") as f: f.write(msg + "\n\x00\n")
def fill(s):
    if "enum" in s: return s["enum"][0]
    t = s.get("type")
    if t == "string": return "t"
    if t == "boolean": return False
    if t in ("integer", "number"): return s.get("minimum", 0)
    if t == "array":
        p = list(s.get("items", {}).get("enum", [0])); return p[:s.get("minItems", 0) or 0]
    if t == "object":
        pr = s.get("properties", {}); rq = s.get("required", list(pr))
        return {k: fill(pr[k]) for k in sorted(pr) if k in rq}
    return "t"
sch = json.loads(sc) if sc else {}
if "maindeck" in sch.get("properties", {}):
    rows = len(json.loads(os.environ["MTG_LLM_ANSWER_SCHEMA"])["properties"]["maindeck"]["prefixItems"])
    o = {"maindeck": [1 if i < 23 else 0 for i in range(rows)], "lands": {"Island": 9, "Swamp": 8}}
else:
    o = fill(sch)
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": sid, "result": json.dumps(o), "structured_output": o,
                  "usage": {"input_tokens": 10, "output_tokens": 2,
                            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"##;

fn stub(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(&bin, STUB).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn calls(path: &Path) -> usize {
    std::fs::read_to_string(path).map_or(0, |s| s.lines().count())
}

fn have_python() -> bool {
    std::process::Command::new("python3").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
}

/// A scratch directory with the stub seat in it, and a way to run the
/// runner against it.
struct Pod {
    dir: PathBuf,
    bin: PathBuf,
}

impl Pod {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mtg-draft-resume-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let bin = stub(&dir);
        Self { dir, bin }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// `--players 2 --best-of 1 --seed 7`, each unless `extra` names it.
    fn run(&self, extra: &[&str], counter: &str, log: &str) -> std::process::Output {
        self.run_env(extra, counter, log, &[])
    }

    /// As [`Pod::run`], with the settings that change a seat's system
    /// prompt at their defaults unless `env` sets them.
    fn run_env(&self, extra: &[&str], counter: &str, log: &str, env: &[(&str, &str)]) -> std::process::Output {
        let defaults = [("--players", "2"), ("--best-of", "1"), ("--seed", "7")];
        std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
            .args(["--model", "cc"])
            .args(defaults.iter().filter(|(f, _)| !extra.contains(f)).flat_map(|(f, v)| [*f, *v]))
            .args(["--log", self.path(log).to_str().unwrap()])
            .args(extra)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .env("CLAUDE_CODE_BIN", &self.bin)
            .env("CALLS", self.path(counter))
            .env_remove("MTG_LLM_HISTORY")
            .env_remove("MTG_CLAUDE_CODE_DRAFT_ANSWERS")
            .env_remove("MTG_CLAUDE_CODE_ANSWERS")
            .env_remove("MTG_LLM_PASS_UNTIL")
            .envs(env.iter().copied())
            .output()
            .expect("the runner runs")
    }

    /// A whole run's snapshot, at `name`.
    fn finished_snapshot(&self, name: &str) -> serde_json::Value {
        let snap = self.path(name);
        let first = self.run(&["--save", snap.to_str().unwrap()], "first.calls", "first.log");
        assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
        serde_json::from_str(&std::fs::read_to_string(&snap).unwrap()).unwrap()
    }

    fn write(&self, name: &str, save: &serde_json::Value) -> String {
        let path = self.path(name);
        std::fs::write(&path, save.to_string()).unwrap();
        path.to_str().unwrap().to_string()
    }
}

#[test]
fn a_resume_after_the_tournament_replays_nothing_and_says_so() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("carry");
    let snap = pod.path("snap.json");
    let saved = pod.finished_snapshot("snap.json");
    assert!(calls(&pod.path("first.calls")) > 0);
    assert_eq!(saved["decks"].as_array().map(Vec::len), Some(2), "both built decks are in the snapshot");
    assert_eq!(saved["matches"].as_array().map(Vec::len), Some(1), "the one match is in the snapshot");

    let second = pod.run(&["--resume", snap.to_str().unwrap()], "second.calls", "second.log");
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(second.status.success(), "{stderr}");
    assert_eq!(calls(&pod.path("second.calls")), 0,
        "the picks, the decks and the match all come from the snapshot — nothing is asked again:\n{stderr}");
    let standings: Vec<&str> = stderr.lines().skip_while(|l| !l.contains("Final Standings")).skip(1).take(2).collect();
    assert_eq!(standings.len(), 2, "{stderr}");
    for row in &standings {
        assert!(row.contains("[1 match from snapshot]"), "a carried result says so on its row: {row}");
    }
    let log = std::fs::read_to_string(pod.path("second.log")).unwrap();
    assert!(log.contains("MATCH FROM SNAPSHOT"), "the log says which match this run did not play");
    assert!(log.contains("DECK FROM SNAPSHOT"), "and which decks it did not build");
}

/// #732: the resumed run's own `--save` was written after the decks and
/// then only when a match of its own finished, so a resume that carried
/// every match left a snapshot holding none of them — and
/// `--resume X --save X` erased them from the only copy.
#[test]
fn a_resumed_runs_own_save_keeps_the_matches_it_carried() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("resave");
    let snap = pod.path("snap.json");
    pod.finished_snapshot("snap.json");
    let snap = snap.to_str().unwrap();

    let again = pod.run(&["--resume", snap, "--save", snap], "again.calls", "again.log");
    assert!(again.status.success(), "{}", String::from_utf8_lossy(&again.stderr));
    let resaved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(snap).unwrap()).unwrap();
    assert_eq!(resaved["matches"].as_array().map(Vec::len), Some(1),
        "the carried match is still in the snapshot the resume wrote");
}

/// #733: an impossible replayed pick was found only when the replay reached
/// it, after `--save` had been rewritten with the steps before it, so
/// `--resume X --save X` refused the snapshot and destroyed it in one go.
#[test]
fn a_refused_pick_leaves_the_snapshot_as_it_was() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("badpick");
    let mut save = pod.finished_snapshot("snap.json");
    save["decks"] = serde_json::json!([]);
    save["matches"] = serde_json::json!([]);
    save["picks"].as_array_mut().unwrap().truncate(40);
    let bad = save["picks"].as_array_mut().unwrap().iter_mut()
        .find(|r| r["round"] == 1 && r["pick"] == 10 && r["seat"] == 1).unwrap();
    bad["card"] = "Black Lotus".into();
    let path = pod.write("bad.json", &save);
    let before = std::fs::read_to_string(&path).unwrap();

    let out = pod.run(&["--resume", &path, "--save", &path], "bad.calls", "bad.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("impossible pick (seat 1, pack 1, pick 10, Black Lotus)"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before,
        "the refused snapshot is left as it was found");
    assert_eq!(calls(&pod.path("bad.calls")), 0);
}

/// #730: the saved decks were played as they stood, so a deck of cards its
/// seat never drafted played the tournament and a name that is not a card
/// panicked a match worker.
#[test]
fn a_saved_deck_its_pool_could_not_build_is_refused() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("baddeck");
    let mut save = pod.finished_snapshot("snap.json");
    save["matches"] = serde_json::json!([]);
    let other_seats_card = save["decks"][1]["deck"]["maindeck"][0].clone();
    let seat0_pool: Vec<serde_json::Value> = ["maindeck", "sideboard"].iter()
        .flat_map(|k| save["decks"][0]["deck"][k].as_array().unwrap().clone())
        .collect();
    assert!(!seat0_pool.contains(&other_seats_card), "the test needs a card seat 0 never drafted");

    for (name, card, needle) in [
        ("fake", serde_json::Value::from("Not A Real Card"), "'Not A Real Card' is not in your drafted pool"),
        ("theirs", other_seats_card.clone(), "is not in your drafted pool"),
    ] {
        let mut edited = save.clone();
        edited["decks"][0]["deck"]["maindeck"][0] = card;
        let path = pod.write(&format!("{name}.json"), &edited);
        let counter = format!("{name}.calls");
        let out = pod.run(&["--resume", &path], &counter, &format!("{name}.log"));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{name}: {stderr}");
        assert!(stderr.contains("seat 0's deck is not one its pool builds") && stderr.contains(needle),
            "{name}: {stderr}");
        assert!(!stderr.contains("panicked"), "{name}: refused, not crashed: {stderr}");
        assert_eq!(calls(&pod.path(&counter)), 0, "{name}: refused before any game");
    }

    // A sideboard of a card the seat never drafted (#886's review).
    let mut side = save.clone();
    side["decks"][0]["deck"]["sideboard"].as_array_mut().unwrap().push(other_seats_card.clone());
    let path = pod.write("side.json", &side);
    let out = pod.run(&["--resume", &path], "side.calls", "side.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && stderr.contains("seat 0's sideboard is not the rest of its pool"), "{stderr}");
    assert_eq!(calls(&pod.path("side.calls")), 0, "refused before any game");

    let mut short = save.clone();
    short["decks"][0]["deck"]["maindeck"].as_array_mut().unwrap().truncate(5);
    let path = pod.write("short.json", &short);
    let out = pod.run(&["--resume", &path], "short.calls", "short.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && stderr.contains("need at least 40"), "{stderr}");
}

/// #729: `--best-of` came from the flags, not the snapshot, so a best-of-1
/// save resumed at the default played its next round as best-of-3 and
/// ranked game wins from both formats on one table.
#[test]
fn a_resume_plays_at_the_match_length_its_snapshot_was_played_at() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("bestof");
    let save = pod.finished_snapshot("snap.json");
    assert_eq!(save["best_of"], 1, "the snapshot records its match length");
    let path = pod.path("snap.json");

    let out = pod.run(&["--resume", path.to_str().unwrap(), "--best-of", "3"], "bo3.calls", "bo3.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("note: --best-of comes from the save (3 -> 1)"), "{stderr}");
    assert!(stderr.contains("best-of-1 ==="), "{stderr}");
    assert_eq!(calls(&pod.path("bo3.calls")), 0, "the best-of-1 match is still the one carried");
    let log = std::fs::read_to_string(pod.path("bo3.log")).unwrap();
    assert!(log.contains("NOTE --best-of comes from the save (3 -> 1)"),
        "the log says where its header's best-of came from");

    // A snapshot from before the field: checked against the flag, and the
    // refusal says why rather than calling a finished match unfinished.
    let mut old = save.clone();
    old.as_object_mut().unwrap().remove("best_of");
    let path = pod.write("old.json", &old);
    let out = pod.run(&["--resume", &path, "--best-of", "3"], "old.calls", "old.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("does not record its match length"), "{stderr}");
    let out = pod.run(&["--resume", &path], "old1.calls", "old1.log");
    assert!(out.status.success(), "with the --best-of it was played at, it resumes: {}",
        String::from_utf8_lossy(&out.stderr));
    let log = std::fs::read_to_string(pod.path("old1.log")).unwrap();
    assert!(log.contains("NOTE --best-of 1 is the flag's"), "and the log says the length is assumed");
}

/// #731: a saved match was carried as it stood, so a 7-0 best-of-1 went
/// into the standings; and one the pairings never took was dropped and the
/// match re-played, both in silence.
#[test]
fn a_saved_match_this_tournament_did_not_finish_is_refused() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("badmatch");
    let save = pod.finished_snapshot("snap.json");
    let mut score = save.clone();
    score["matches"][0]["result"]["wins_a"] = 7.into();
    score["matches"][0]["result"]["wins_b"] = 0.into();
    let mut swapped = save.clone();
    let r = &mut swapped["matches"][0]["result"];
    let (a, b) = (r["player_a"].clone(), r["player_b"].clone());
    (r["player_a"], r["player_b"]) = (b, a);
    let (wa, wb) = (r["wins_a"].clone(), r["wins_b"].clone());
    (r["wins_a"], r["wins_b"]) = (wb, wa);

    for (name, edited, needle) in [
        ("score", score, "says 7-0"),
        ("swapped", swapped, "is not a match this tournament pairs"),
    ] {
        let path = pod.write(&format!("{name}.json"), &edited);
        let counter = format!("{name}.calls");
        let out = pod.run(&["--resume", &path], &counter, &format!("{name}.log"));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{name}: {stderr}");
        assert!(stderr.contains(needle), "{name}: {stderr}");
        assert_eq!(calls(&pod.path(&counter)), 0, "{name}: refused, not re-played");
    }
}

/// #734: a snapshot's seat count replaced `--players` with no floor, and
/// `"players": 0` panicked indexing the log header's first seat.
#[test]
fn a_snapshot_with_no_seats_or_no_games_is_refused() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("zero");
    let save = pod.finished_snapshot("snap.json");
    for (name, field, needle) in [("p0", "players", "0 seats"), ("b0", "best_of", "best-of-0")] {
        let mut edited = save.clone();
        edited[field] = 0.into();
        let path = pod.write(&format!("{name}.json"), &edited);
        let out = pod.run(&["--resume", &path], &format!("{name}.calls"), &format!("{name}.log"));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{name}: refused, not panicked: {stderr}");
        assert!(stderr.contains(needle) && !stderr.contains("panicked"), "{name}: {stderr}");
    }
}

/// #735: the log header said "resumed from … (N picks replayed)" before the
/// snapshot was checked, so a refused resume left a log asserting a replay
/// that never happened, with no word of the refusal.
#[test]
fn a_refused_resume_writes_no_log_claiming_a_replay() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("refusedlog");
    let mut save = pod.finished_snapshot("snap.json");
    save["picks"][4]["seat"] = 2.into();
    let path = pod.write("bad.json", &save);
    let out = pod.run(&["--resume", &path], "bad.calls", "bad.log");
    assert!(!out.status.success());
    let log = std::fs::read_to_string(pod.path("bad.log")).unwrap_or_default();
    assert!(!log.contains("resumed from"), "no header for a replay that never happened:\n{log}");
}

/// #836: a seat's notes — the tail of its last pick's reasoning, the only
/// way a seat sent no history carries its plan to the next pick — are in
/// the snapshot, and a resume gives them back. A replayed pick asks no
/// model, so the first live pick after a resume opened with no plan.
#[test]
fn a_resumed_draft_gives_each_seat_its_notes_back() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("notes");
    let mut saved = pod.finished_snapshot("snap.json");
    let notes = saved["notes"].as_array().expect("the snapshot keeps the seats' notes").clone();
    assert_eq!(notes.len(), 2, "one per seat: {notes:?}");
    assert!(notes.iter().all(|n| n.is_string()), "both seats left notes: {notes:?}");

    // Back to after pack 1 pick 1, with notes only this test could have
    // written, so the first live pick shows whether they came back.
    saved["picks"] = serde_json::Value::Array(saved["picks"].as_array().unwrap()[..2].to_vec());
    saved["decks"] = serde_json::json!([]);
    saved["matches"] = serde_json::json!([]);
    saved["notes"] = serde_json::json!(["seat 0 plan: blue-black fliers", "seat 1 plan: red aggro"]);
    let snap = pod.write("early.json", &saved);

    let prompts = pod.path("prompts.txt");
    let resumed = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7"])
        .args(["--log", pod.path("resumed.log").to_str().unwrap(), "--resume", &snap])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &pod.bin)
        .env("CALLS", pod.path("resumed.calls"))
        .env("PROMPTS", &prompts)
        .output()
        .expect("the runner runs");
    assert!(resumed.status.success(), "{}", String::from_utf8_lossy(&resumed.stderr));
    let asked = std::fs::read_to_string(&prompts).unwrap();
    for plan in ["seat 0 plan: blue-black fliers", "seat 1 plan: red aggro"] {
        assert!(asked.contains(&format!("Your notes from your last pick:\n{plan}")),
            "the first live pick after the resume carries {plan:?}");
    }
}

/// #924: the snapshot records the deal, and a run that deals other packs —
/// a build whose card pool moved a card no pick took — is refused before
/// anything is written, rather than replaying the picks onto them. A
/// snapshot written before the deal was recorded carries on as before.
#[test]
fn a_snapshot_of_another_deal_is_refused() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("deal");
    let mut save = pod.finished_snapshot("snap.json");
    assert!(save["deal"].as_str().is_some_and(|d| d.len() == 16), "the deal is recorded: {}", save["deal"]);
    save["deal"] = "0000000000000000".into();
    let path = pod.write("other.json", &save);
    let before = std::fs::read_to_string(&path).unwrap();
    let out = pod.run(&["--resume", &path, "--save", &path], "other.calls", "other.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("the packs dealt now are not the ones the table drafted"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "the refused snapshot is left as it was found");
    assert_eq!(calls(&pod.path("other.calls")), 0);

    save.as_object_mut().unwrap().remove("deal");
    let path = pod.write("old.json", &save);
    let out = pod.run(&["--resume", &path], "old.calls", "old.log");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

/// #949: the snapshot records the settings besides the guide that change a
/// seat's system prompt, and a resume under others says so on stderr and
/// in the log, as `mtg-draft-replay` does for kept positions. It said
/// nothing, and every later pick was asked another question.
#[test]
fn a_resume_under_other_prompt_settings_says_so() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("settings");
    let saved = pod.finished_snapshot("snap.json");
    for key in ["history", "text_answers", "game_text_answers", "pass_until"] {
        assert!(!saved["prompt"][key].is_null(), "the snapshot records {key}: {}", saved["prompt"]);
    }
    let game_text = saved["prompt"]["game_text_answers"].as_bool().unwrap();
    let pass_until = saved["prompt"]["pass_until"].as_bool().unwrap();
    let snap = pod.path("snap.json");
    let snap = snap.to_str().unwrap();

    for (var, value, said) in [
        ("MTG_LLM_HISTORY", "2", "made under MTG_LLM_HISTORY=0, and the rest of this draft is made under 2"),
        ("MTG_CLAUDE_CODE_DRAFT_ANSWERS", "text",
            "made under MTG_CLAUDE_CODE_DRAFT_ANSWERS=schema, and the rest of this draft is asked in text"),
        // The tournament's games are asked under the game seat's settings.
        ("MTG_CLAUDE_CODE_ANSWERS", if game_text { "schema" } else { "text" },
            if game_text { "played under MTG_CLAUDE_CODE_ANSWERS=text, and the rest of this tournament is asked in the schema" }
            else { "played under MTG_CLAUDE_CODE_ANSWERS=schema, and the rest of this tournament is asked in text" }),
        ("MTG_LLM_PASS_UNTIL", if pass_until { "0" } else { "1" },
            if pass_until { "played under MTG_LLM_PASS_UNTIL=1, and the rest of this tournament is played with the pass-until row off" }
            else { "played under MTG_LLM_PASS_UNTIL=0, and the rest of this tournament is played with the pass-until row on" }),
    ] {
        let log = format!("{var}.log");
        let out = pod.run_env(&["--resume", snap], &format!("{var}.calls"), &log, &[(var, value)]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{stderr}");
        assert!(stderr.contains("its system prompt is not the live one") && stderr.contains(said),
            "{var}={value} is said on stderr:\n{stderr}");
        let logged = std::fs::read_to_string(pod.path(&log)).unwrap();
        assert!(logged.contains(said), "and in the log:\n{logged}");
    }

    let same = pod.run(&["--resume", snap], "same.calls", "same.log");
    let stderr = String::from_utf8_lossy(&same.stderr);
    assert!(same.status.success(), "{stderr}");
    assert!(!stderr.contains("not the live one"), "the same settings say nothing:\n{stderr}");

    // A snapshot from before the record, with its guides recorded: the
    // rest is said to be unknown, not taken as agreement.
    let mut old = saved.clone();
    old.as_object_mut().unwrap().remove("prompt");
    let old = pod.write("old.json", &old);
    let out = pod.run(&["--resume", &old], "old.calls", "old.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("predates the MTG_LLM_HISTORY / answer-format / pass-until record"), "{stderr}");
}

/// #950: a guide edited in place keeps its path, and the warning named the
/// same guide on both sides of "and the rest of this draft".
#[test]
fn a_guide_edited_in_place_is_named_as_it_was_and_as_it_is() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("edited-guide");
    let guide = pod.path("guide.md");
    std::fs::write(&guide, "Take red cards.\n").unwrap();
    let guide = guide.to_str().unwrap();
    let snap = pod.path("snap.json");
    let first = pod.run(&["--guide", guide, "--save", snap.to_str().unwrap()], "first.calls", "first.log");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));

    std::fs::write(guide, "Take green cards.\n").unwrap();
    let out = pod.run(&["--guide", guide, "--resume", snap.to_str().unwrap()], "second.calls", "second.log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let warn = stderr.lines().find(|l| l.starts_with("WARN: seat 0")).unwrap_or_else(|| panic!("no warning:\n{stderr}"));
    assert!(warn.contains(&format!("guide {guide} as it was")) && warn.contains(&format!("guide {guide} as it is now")),
        "the warning says what changed: {warn}");
}
