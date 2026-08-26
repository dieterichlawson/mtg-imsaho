//! Guards on the test suite itself.
//!
//! A test can stop testing anything without ever going red, and its comments
//! can stop being true without anything noticing — neither is compiled, run, or
//! reviewed against the thing it describes. These are the three kinds of rot
//! that had accumulated badly enough to be worth failing the build over.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/mtg-engine.
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file in the workspace, by file name.
fn source_file_names() -> BTreeSet<String> {
    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    files
        .iter()
        .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
        .collect()
}

/// Every test file except this one — the guards below quote the patterns they
/// forbid, so scanning themselves would always fail.
fn test_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"), &mut files);
    files.retain(|p| p.file_name().is_none_or(|n| n != "test_suite_guards.rs"));
    files.sort();
    files
}

/// A comment that points at `foo.rs` must point at a file that exists.
///
/// These references used to carry line numbers as well (`engine.rs:4085`), and
/// 19 of the 77 already pointed past the end of the file they named — one of
/// them 3000 lines past it. A line number is stale the moment anyone edits the
/// file above it, so the numbers are gone; the file names are worth keeping,
/// and worth checking.
#[test]
fn every_source_file_a_test_comment_names_exists() {
    let known = source_file_names();
    let re = regex_lite_rs_backtick();
    let mut offenders = Vec::new();

    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                continue;
            }
            for name in re(line) {
                if !known.contains(&name) {
                    offenders.push(format!(
                        "{}:{}: names `{name}`, which is not a file in this workspace",
                        path.file_name().unwrap().to_string_lossy(),
                        n + 1
                    ));
                }
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} stale file reference(s) in test comments:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// No test may claim it is expected to fail.
///
/// The suite had accumulated 62 comments saying "This test asserts the EXPECTED
/// CORRECT behavior, so it currently fails. It will start passing as soon as
/// Bug X is fixed." Every one of them was passing — the bugs had been fixed and
/// nobody went back to the comment. A reader who believes them mistrusts a
/// green suite, which is worse than having no comment at all.
///
/// A test that genuinely should not pass yet belongs behind `#[ignore]`, where
/// the runner reports it, rather than in prose the runner cannot see.
#[test]
fn no_test_comment_claims_the_test_is_failing() {
    const STALE: &[&str] = &[
        "EXPECTED CORRECT behavior",
        "currently fails",
        "will start passing",
        "this test should fail",
        "expected to fail",
        "expected to FAIL",
        "false positive",
    ];
    let mut offenders = Vec::new();

    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                continue;
            }
            let lower = line.to_lowercase();
            for needle in STALE {
                if lower.contains(&needle.to_lowercase()) {
                    offenders.push(format!(
                        "{}:{}: {}",
                        path.file_name().unwrap().to_string_lossy(),
                        n + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} comment(s) claim a passing test is failing:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// Pull `` `foo.rs` `` references out of a comment line. Backticks only: an
/// unquoted `.rs` in prose is usually part of a path being described rather
/// than a reference to a file in this tree.
fn regex_lite_rs_backtick() -> impl Fn(&str) -> Vec<String> {
    |line: &str| {
        let mut out = Vec::new();
        let bytes: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != '`' {
                i += 1;
                continue;
            }
            let Some(end) = bytes[i + 1..].iter().position(|c| *c == '`') else { break };
            let inner: String = bytes[i + 1..i + 1 + end].iter().collect();
            i += end + 2;
            if !inner.ends_with(".rs") {
                continue;
            }
            // `audits/AUDIT_BUGS.md)` style paths are handled by the .rs check;
            // take the last path segment as the file name.
            let name = inner.rsplit('/').next().unwrap_or(&inner).to_string();
            if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                out.push(name);
            }
        }
        out
    }
}

// ── Tests that call a card hook the card does not implement ──────────

/// The set of `fn on_*` / `fn resolve_*` methods each card overrides, by card
/// name, read out of `src/cards/`.
fn card_overrides() -> std::collections::BTreeMap<String, BTreeSet<String>> {
    let mut out = std::collections::BTreeMap::new();
    let mut files = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards"), &mut files);
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let Some(name) = text
            .split_once("name: \"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(n, _)| n.to_string())
        else { continue };
        let hooks: BTreeSet<String> = text
            .lines()
            .filter_map(|l| {
                let t = l.trim();
                let rest = t.strip_prefix("fn ")?;
                let hook = rest.split('(').next()?;
                (hook.starts_with("on_") || hook.starts_with("resolve_"))
                    .then(|| hook.to_string())
            })
            .collect();
        out.insert(name, hooks);
    }
    out
}

/// Calls that reach a hook the card leaves at its default, but where the
/// default is the point of the test. Each entry is (test file, hook).
const DEFAULT_IS_THE_POINT: &[(&str, &str)] = &[
    // CR 602.2a: the default `on_activate_ability` pushes the ability onto the
    // stack. These tests exist to check a card has NOT overridden it to apply
    // its effect at activation time.
    ("activated_no_stack.rs", "on_activate_ability"),
];

/// A test that calls `behavior.on_x(...)` for a card that never overrides
/// `on_x` is calling the trait default. For most hooks the default does
/// nothing at all, so the test's "and then nothing happened" holds no matter
/// what the card does — it would pass with the card's real behaviour deleted.
///
/// Four such tests were found by hand: one called `on_activate_ability` when
/// the ability lived in `resolve_activated_ability`, one called a damage hook
/// that had become a `replace_event` replacement, and two asserted an ability
/// had not fired yet on cards that never implemented that hook.
#[test]
fn no_test_calls_a_card_hook_the_card_leaves_at_its_default() {
    let overrides = card_overrides();
    let mut offenders = Vec::new();

    for path in test_files() {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let lines: Vec<&str> = text.lines().collect();

        for (start, end) in fn_spans(&lines) {
            let body = &lines[start..end];
            let blob = body.join("\n");
            let named = named_objects(&blob);
            let behaviors = behavior_bindings(&blob);

            for (offset, line) in body.iter().enumerate() {
                for (var, hook) in hook_calls(line) {
                    let Some(obj) = behaviors.get(&var) else { continue };
                    let Some(card) = named.get(obj) else { continue };
                    let Some(implemented) = overrides.get(card) else { continue };
                    if implemented.contains(&hook) {
                        continue;
                    }
                    if DEFAULT_IS_THE_POINT.iter().any(|(f, h)| *f == file && *h == hook) {
                        continue;
                    }
                    // The default `on_activate_ability` puts the ability on the
                    // stack (CR 602.2a). A test that then resolves the stack has
                    // driven the ability the way the engine does, so the card not
                    // overriding the hook is correct rather than suspicious.
                    if hook == "on_activate_ability" && blob.contains("resolve_top_of_stack") {
                        continue;
                    }
                    offenders.push(format!(
                        "{file}:{}: calls `{card}`'s {hook}(), which {card} does not implement — \
                         this exercises the trait default, not the card",
                        start + offset + 1
                    ));
                }
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} test(s) call a hook the card leaves at its default:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// Byte ranges of each `fn ...` item in a file, as (first line, one past last).
fn fn_spans(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("fn ") {
            let start = i;
            let mut depth = 0i32;
            let mut started = false;
            while i < lines.len() {
                depth += i32::try_from(lines[i].matches('{').count()).unwrap_or(0)
                    - i32::try_from(lines[i].matches('}').count()).unwrap_or(0);
                if lines[i].contains('{') {
                    started = true;
                }
                i += 1;
                if started && depth <= 0 {
                    break;
                }
            }
            out.push((start, i));
        } else {
            i += 1;
        }
    }
    out
}

/// `let <var> = named_creature(.., "<Card>", ..)` bindings in a function body.
fn named_objects(blob: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for line in blob.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("let ") else { continue };
        let Some((var, rhs)) = rest.split_once(" = ") else { continue };
        if !(rhs.starts_with("named_creature(")
            || rhs.starts_with("named_equipment(")
            || rhs.starts_with("named_card_in_graveyard("))
        {
            continue;
        }
        if let Some((_, after)) = rhs.split_once('"') {
            if let Some((card, _)) = after.split_once('"') {
                out.insert(var.trim().to_string(), card.to_string());
            }
        }
    }
    out
}

/// `let <var> = registry.get(state.get_object(<obj>).unwrap().card_id)` bindings.
fn behavior_bindings(blob: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for line in blob.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("let ") else { continue };
        let Some((var, rhs)) = rest.split_once(" = ") else { continue };
        let Some((_, after)) = rhs.split_once(".get_object(") else { continue };
        let Some((obj, tail)) = after.split_once(')') else { continue };
        if !tail.contains(".card_id") {
            continue;
        }
        out.insert(var.trim().to_string(), obj.trim().to_string());
    }
    out
}

/// `<var>.on_something(` calls on a line, as (var, hook).
fn hook_calls(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (idx, _) in line.match_indices('.') {
        let after = &line[idx + 1..];
        let Some(open) = after.find('(') else { continue };
        let hook = &after[..open];
        if !(hook.starts_with("on_") || hook.starts_with("resolve_")) {
            continue;
        }
        if !hook.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) {
            continue;
        }
        let before = &line[..idx];
        let var: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !var.is_empty() {
            out.push((var, hook.to_string()));
        }
    }
    out
}

// ── Tests that assert a card's own data back at itself ───────────────

/// A test whose every assertion reads a `CardData` binding, and which never
/// builds a `GameState`, is restating the card file. `power: Some(1)` in the
/// card, `assert_eq!(data.power, Some(1))` in the test — there is no second
/// source of truth here for it to disagree with, so it can only fail when
/// somebody edits the card, and then it fails without telling them anything
/// the diff did not.
///
/// 64 of these were removed. `card_data_invariants.rs` checks the
/// *relationships* between the fields across all cards instead, and
/// `characteristics_card_sweep.rs` checks that what a card prints is what the
/// accessors report once it is on the battlefield.
#[test]
fn no_test_asserts_a_cards_data_back_at_itself() {
    let mut offenders = Vec::new();

    for path in test_files() {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        // The invariant files exist to read card data; that is the point.
        if file == "card_data_invariants.rs" || file == "characteristics_card_sweep.rs" {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let lines: Vec<&str> = text.lines().collect();

        for (start, end) in fn_spans(&lines) {
            let body = &lines[start..end];
            let blob = body.join("\n");
            if !lines[start.saturating_sub(4)..start].iter().any(|l| l.contains("#[test]")) {
                continue;
            }
            let asserts: Vec<&&str> = body.iter()
                .filter(|l| l.trim_start().starts_with("assert"))
                .collect();
            if asserts.is_empty() {
                continue;
            }
            // A test that builds a game is testing behaviour, whatever else it reads.
            if ["game_at_step", "GameState::new", "create_object", "named_creature",
                "move_object", "submit_action"].iter().any(|m| blob.contains(m)) {
                continue;
            }
            let data_vars = card_data_bindings(&blob);
            if data_vars.is_empty() {
                continue;
            }
            if asserts.iter().all(|a| data_vars.iter().any(|v| mentions(a, v))) {
                let name = body[0].trim().trim_start_matches("fn ")
                    .split('(').next().unwrap_or("?").to_string();
                offenders.push(format!("{file}:{}: {name} asserts only card data", start + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} test(s) restate a card's own data:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// `let <var> = ....card_data(..)` / `.back_face_data()` bindings.
fn card_data_bindings(blob: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in blob.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("let ") else { continue };
        let Some((var, rhs)) = rest.split_once(" = ") else { continue };
        if rhs.contains("card_data(") || rhs.contains("back_face_data()") {
            out.push(var.trim().to_string());
        }
    }
    out
}

/// Whether `line` uses `var` as a whole identifier.
fn mentions(line: &str, var: &str) -> bool {
    let bytes: Vec<char> = line.chars().collect();
    let target: Vec<char> = var.chars().collect();
    bytes.windows(target.len().max(1)).enumerate().any(|(i, w)| {
        w == target.as_slice()
            && bytes.get(i.wrapping_sub(1)).is_none_or(|c| !c.is_alphanumeric() && *c != '_')
            && bytes.get(i + target.len()).is_none_or(|c| !c.is_alphanumeric() && *c != '_')
    })
}

/// A test must not perform a step's or a turn's own bookkeeping by hand.
///
/// Three tests used to end a turn by clearing `until_end_of_turn` themselves —
/// one of them replayed the whole cleanup step inline, under the comment
/// "matching engine.rs cleanup" — and then asserted the result. What they
/// asserted was that their own copy of the rule worked: deleting the engine's
/// cleanup step outright left all three green. Ending the turn for real found a
/// once-per-turn ability that was never re-enabled between turns.
///
/// `common::advance_to_cleanup` / `advance_to_next_turn` run the real steps.
#[test]
fn no_test_ends_the_turn_by_hand() {
    // Fields the engine resets as part of a step or turn transition. A test
    // that writes one is standing in for the engine.
    const ENGINE_BOOKKEEPING: &[&str] = &[
        "until_end_of_turn.clear()",
        "abilities_activated_this_turn.clear()",
        "num_spells_cast_this_turn.clear()",
        "regeneration_shields = 0",
        "damage_marked = 0",
    ];
    let mut offenders = Vec::new();
    for path in test_files() {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(pat) = ENGINE_BOOKKEEPING.iter().find(|p| line.contains(**p)) {
                offenders.push(format!("{file}:{}: {pat}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} test(s) do a step's own bookkeeping instead of running the step \
         (use common::advance_to_cleanup / advance_to_next_turn):\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// Every source file in the crate, with its path relative to `src/`.
fn crate_sources() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    files.sort();
    files.iter()
        .filter_map(|p| {
            let rel = p.strip_prefix(&src).ok()?.to_string_lossy().replace('\\', "/");
            Some((rel, fs::read_to_string(p).ok()?))
        })
        .collect()
}

/// Nothing outside `damage.rs` marks damage on a permanent.
///
/// `damage.rs`'s own module doc has said so since the pipeline was unified —
/// "Engine and card code must never write `damage_marked` directly: every
/// hand-rolled copy of this logic has historically missed at least one check" —
/// and nothing enforced it. Eleven tests in `inline_damage.rs` each named one
/// card that had missed one: Balefire Dragon skipped protection, Blasphemous
/// Act skipped Unbreathing Horde's replacement, Devil's Play wrote
/// `damage_marked` on a planeswalker instead of removing loyalty.
///
/// A card that writes the field itself gets none of `deal_damage`'s work:
/// combat-damage prevention, protection (CR 702.16e), prevent-and-remove-a-
/// counter (CR 614.1a), damage multipliers, planeswalker loyalty (CR 120.3c),
/// deathtouch, `damaged_by` tracking, lifelink, or the damage event. Setting it
/// to zero is a different act — clearing marked damage at cleanup, on
/// destruction, on a zone change — and stays allowed.
#[test]
fn only_the_damage_pipeline_marks_damage() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if rel == "damage.rs" {
            continue;
        }
        // Unit tests inside a module set up damaged creatures on purpose.
        let test_mod = text.find("#[cfg(test)]").unwrap_or(text.len());
        for (n, line) in text[..test_mod].lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") {
                continue;
            }
            let writes = l.contains("damage_marked +=")
                || l.contains("damage_marked -=")
                || (l.contains("damage_marked =") && !l.contains("damage_marked = 0"));
            if writes {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "damage must go through `damage::deal_damage`, which applies protection, \
         prevention, multipliers, loyalty removal, deathtouch and lifelink; \
         writing `damage_marked` gets none of it:\n  {}",
        offenders.join("\n  "));
}

/// Nothing outside the loyalty-cost machinery takes loyalty counters off a
/// planeswalker.
///
/// CR 120.3c: damage dealt to a planeswalker removes that many loyalty
/// counters. That is `deal_damage`'s job — Stensia Bloodhall used to decrement
/// loyalty itself and so skipped the protection and prevention checks that a
/// planeswalker is entitled to like anything else.
#[test]
fn only_the_damage_pipeline_removes_loyalty_for_damage() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        // `damage.rs` implements CR 120.3c; `cards/mod.rs` and the planeswalker
        // cards pay and add loyalty as an activation cost (CR 606.3).
        if rel == "damage.rs" {
            continue;
        }
        let test_mod = text.find("#[cfg(test)]").unwrap_or(text.len());
        for (n, line) in text[..test_mod].lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || !l.contains("CounterType::Loyalty") {
                continue;
            }
            if l.contains("remove_counters") && !l.contains("cost") {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "damage to a planeswalker removes loyalty through `damage::deal_damage` \
         (CR 120.3c), so that protection and prevention still apply:\n  {}",
        offenders.join("\n  "));
}
