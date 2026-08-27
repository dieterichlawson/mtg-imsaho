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
        // Present-tense narration of a bug that has since been fixed. A doc
        // saying the test fails, attached to a test that passes, is worse than
        // no doc at all: the next reader believes it.
        "Today it doesn't",
        "the bug fires here",
        "still broken",
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

/// `let <var> = named_permanent(.., "<Card>", ..)` bindings in a function body.
fn named_objects(blob: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for line in blob.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("let ") else { continue };
        let Some((var, rhs)) = rest.split_once(" = ") else { continue };
        if !(rhs.starts_with("named_permanent(")
            || rhs.starts_with("named_permanent(")
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
            if ["game_at_step", "GameState::new", "create_object", "named_permanent",
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

/// No card cleans up after its own resolution.
///
/// `GameState::resolving_spell` has documented the rule for as long as it has
/// existed — "the ENGINE owns moving a resolved spell off the stack ... card
/// code must never call `move_spell_after_resolve` from a pending-effect
/// handler" — and sixteen call sites in `src/cards/` did it anyway.
///
/// Every one was redundant: `stack::resolve_spell` moves a spell that is still
/// on the stack when `on_resolve` returns, and
/// `engine::finish_spell_resolution_if_idle` moves it once a suspended choice
/// chain completes. Redundant is the good case. The bad case is CR 608.2m —
/// reaching the graveyard is the *final* step of resolution, so a card that
/// moves itself and then presents another choice has left the stack
/// mid-resolution, which is the Divine Reckoning bug `spell_cleanup.rs` was
/// written for.
///
/// A counterspell disposing of the spell it countered is a different act and
/// has its own entry point, `move_countered_spell` (CR 701.5a).
#[test]
fn no_card_moves_a_spell_off_the_stack_itself() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if !rel.starts_with("cards/") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with("///") {
                continue;
            }
            if l.contains("move_spell_after_resolve") {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "a resolving spell's trip to the graveyard is the engine's, not the \
         card's (CR 608.2m); to dispose of a *countered* spell use \
         `move_countered_spell`:\n  {}",
        offenders.join("\n  "));
}

/// No test builds a `CombatState` by hand.
///
/// Thirty-one sites across sixteen files did, in three different shapes, and
/// every one of them left `blocked_attackers` empty — a state the engine
/// never produces, because `declare_blockers` records blocked-ness and
/// CR 509.2 makes it permanent for the combat. One of them went further and
/// keyed `blocker_assignments` by the *blocker*, so the engine saw an
/// unblocked attacker and two creatures that were never in combat together;
/// the test's "neither took damage" was true with the effect under test
/// deleted.
///
/// `common::declare_combat` (and `attacks_unblocked` / `attacks_blocked_by`)
/// build what the engine builds.
#[test]
fn no_test_assembles_combat_state_by_hand() {
    let mut offenders = Vec::new();
    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        // `common/mod.rs` is where the helper that builds it lives.
        if name == "mod.rs" {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") {
                continue;
            }
            let builds = l.contains("CombatState")
                || l.contains(".attackers.insert(")
                || l.contains(".blocker_assignments.insert(");
            if builds {
                offenders.push(format!("{name}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "set combat up with `declare_combat` / `attacks_unblocked` / \
         `attacks_blocked_by`, which record blocked-ness the way the engine \
         does (CR 509.2):\n  {}",
        offenders.join("\n  "));
}

/// Pull the `//! - Card Name` bullets out of a module doc, ignoring the
/// wrapped continuation lines that a long bullet produces.
fn module_doc_cards(text: &str) -> Vec<String> {
    let mut cards = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("//! - ") else {
            if line.starts_with("//!") || line.trim().is_empty() {
                continue;
            }
            break; // past the module doc
        };
        // Wrapped continuation lines are `//!   ...`, so they never match the
        // `//! - ` prefix and are already excluded.
        cards.push(rest.trim().to_string());
    }
    cards
}

/// A `cards_*.rs` module doc lists the cards it covers, and that list is the
/// index for anyone looking for a card's acceptance tests. Nothing checked it,
/// and it had drifted three ways at once: `cards_vanilla_and_keywords.rs`
/// claimed twelve cards its body never mentions, `cards_complex_creatures.rs`
/// carried a `── Creepy Doll ──` header with no tests under it, and nineteen
/// cards were claimed by two or three files apiece.
#[test]
fn a_cards_file_covers_exactly_the_cards_its_module_doc_lists() {
    let mut problems = Vec::new();
    for path in test_files() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("cards_") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let listed = module_doc_cards(&text);
        let body = text.split("\nmod common;").nth(1).unwrap_or(&text);

        for card in &listed {
            if !body.contains(card.as_str()) {
                problems.push(format!("{name}: lists {card:?}, which the file never mentions"));
            }
        }
        // The stated count has to be the real one.
        if let Some(start) = text.find("Cards covered (") {
            let n: usize = text[start + 15..].split(')').next().unwrap_or("")
                .parse().unwrap_or(usize::MAX);
            if n != listed.len() {
                problems.push(format!(
                    "{name}: says \"Cards covered ({n})\" but lists {}", listed.len()));
            }
        }
        // A section header with no test under it is a claim with nothing behind it.
        let mut sections: Vec<(&str, usize)> = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if let Some(rest) = line.strip_prefix("// ── ") {
                sections.push((rest.trim_end_matches(['─', ' ']), i));
            }
        }
        let lines: Vec<&str> = text.lines().collect();
        for (idx, &(title, at)) in sections.iter().enumerate() {
            let end = sections.get(idx + 1).map_or(lines.len(), |&(_, n)| n);
            if !lines[at..end].iter().any(|l| l.trim() == "#[test]") {
                problems.push(format!("{name}: section \"{title}\" has no tests under it"));
            }
        }
    }
    assert!(problems.is_empty(),
        "a cards_*.rs module doc is the index for its cards; keep it true:\n  {}",
        problems.join("\n  "));
}

/// `docs/TESTING.md` says where a test goes. It can only do that if it names
/// the files — and it had fallen 32 rule-files behind, while still pointing at
/// three (`audit_bugs.rs`, `tier15_cards.rs`, `state.rs`) that no longer exist.
///
/// `cards_*.rs` files are exempt in both directions: the doc describes them as
/// a class, and each one's own module doc lists its cards.
#[test]
fn the_testing_guide_names_every_rule_test_file() {
    let doc_path = repo_root().join("docs/TESTING.md");
    let doc = fs::read_to_string(&doc_path).expect("docs/TESTING.md exists");

    // Only the tables and the guard list are the map. The opening paragraph
    // names files that were deliberately renamed away, and saying so is the
    // point of that paragraph.
    let mapped: String = doc.lines()
        .filter(|l| l.starts_with('|') || l.starts_with("- `"))
        .collect::<Vec<_>>()
        .join("\n");
    let named: BTreeSet<String> = mapped.match_indices('`')
        .filter_map(|(i, _)| {
            let rest = &mapped[i + 1..];
            let end = rest.find('`')?;
            let word = &rest[..end];
            word.ends_with(".rs").then(|| word.split("::").next().unwrap().to_string())
        })
        .collect();

    // `test_files()` skips this file; the guide names it, and should.
    let mut on_disk: BTreeSet<String> = test_files().iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .filter(|n| n != "mod.rs")
        .collect();
    on_disk.insert("test_suite_guards.rs".into());

    let missing: Vec<&String> = on_disk.difference(&named).collect();
    // A guard bullet may name the `src/` file whose invariant it protects.
    let sources = source_file_names();
    let stale: Vec<&String> = named.difference(&on_disk)
        .filter(|n| !sources.contains(n.as_str()))
        .collect();

    assert!(missing.is_empty() && stale.is_empty(),
        "docs/TESTING.md is the map of this suite.\n  \
         not in the guide: {missing:?}\n  \
         named but gone:  {stale:?}");
}

/// No card decides "is this a creature" by looking at `obj.power`.
///
/// `GameState::is_creature` is the accessor, and it is
/// `has_card_type(Creature) || obj.power.is_some()` — card types *plus* the
/// object-level P/T sentinel that tokens and `*/*` creatures carry. Sixty-six
/// sites across fifty-two ISD cards inlined one half or the other, and two
/// open-coded the whole union by hand.
///
/// Today the two agree everywhere the card pool can reach: verified by probe
/// that a registry creature, a token, a creature card in hand or graveyard,
/// and a DFC front face all set `obj.power`, and that nothing in the pool
/// grants the Creature type to something without P/T. The one case where they
/// diverge is exactly that — a permanent animated by an effect, which reads
/// `power: None` and `is_creature: true`. So this guard is about the trap, not
/// a live bug: the first animation card added to the pool would otherwise be
/// invisible to every one of those sites at once.
#[test]
fn no_card_uses_obj_power_as_a_creature_test() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if !rel.starts_with("cards/") {
            continue;
        }
        let test_mod = text.find("#[cfg(test)]").unwrap_or(text.len());
        for (n, line) in text[..test_mod].lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") {
                continue;
            }
            if l.contains(".power.is_some()") || l.contains(".power.is_none()") {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "ask `state.is_creature(id, registry)`; `obj.power` alone misses a \
         permanent that is a creature by type grant:\n  {}",
        offenders.join("\n  "));
}

/// An activated ability's effect belongs in `resolve_activated_ability`.
///
/// CR 602.2a: "the player announces their intentions ... the ability goes on
/// the stack." The effect happens later, when it resolves. `CardBehavior` used
/// to have an `on_activate_ability` hook whose *default body was that stack
/// push*, so overriding it to do the effect deleted the push: 46 of the set's
/// 53 activated abilities did, and Ghost Quarter destroyed a land the instant
/// it was activated, Elder of Laurels counted creatures at announcement rather
/// than at resolution, and no opponent ever got the priority CR 117.3b owes
/// them.
///
/// The hook is gone. `engine::actions::abilities::put_ability_on_stack` owns
/// the push, and cards get `pay_activation_cost` for a cost the
/// `ActivatedAbilityDef` cannot express (Moorland Haunt exiling a creature card
/// from a graveyard, Blazing Torch sacrificing the Equipment attached to the
/// creature the ability was activated on — CR 601.2h). This guard keeps the
/// name from coming back, in cards or in tests that would drive it.
#[test]
fn no_card_or_test_names_the_removed_activation_hook() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with("///") {
                continue;
            }
            if l.contains("on_activate_ability") && !l.contains("on_activate_mana_ability") {
                offenders.push(format!("src/{rel}:{}", n + 1));
            }
        }
    }
    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        if path.file_name().is_some_and(|f| f == "test_suite_guards.rs") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with("///") {
                continue;
            }
            if l.contains("on_activate_ability") && !l.contains("on_activate_mana_ability") {
                offenders.push(format!("{}:{}", path.display(), n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "`on_activate_ability` is gone: the engine puts an activated ability on \
         the stack (CR 602.2a), a card's effect goes in \
         `resolve_activated_ability`, and a cost the `ActivatedAbilityDef` \
         cannot express goes in `pay_activation_cost`. Tests drive both halves \
         with `common::activate_via_hooks`:\n  {}",
        offenders.join("\n  "));
}

/// Nothing outside the engine puts an ability on the stack.
///
/// `push_ability` needs `behavior_card_id`, which is not always the activated
/// object's own card: Skeletal Grimace grants "{B}: Regenerate this creature"
/// to what it enchants, and Blazing Torch grants its damage ability to what it
/// equips. Only `activate_ability` has done the native → copy-grantor →
/// attached-permanent walk that resolves it, so only it may push.
#[test]
fn only_the_engine_puts_an_ability_on_the_stack() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if rel == "cards/mod.rs" || rel == "engine/actions/abilities.rs" {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with("///") {
                continue;
            }
            // Constructing one, not matching on one: `stack.push(` on the
            // same line, or `push_ability` by name.
            let constructs = l.contains("push_ability")
                || (l.contains("StackEntry::Ability") && l.contains(".push("));
            if constructs {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "putting an activated ability on the stack is \
         `engine::actions::abilities::put_ability_on_stack`'s job — it is the \
         only caller that knows which card's behavior contributes the \
         ability:\n  {}",
        offenders.join("\n  "));
}

/// A card that enumerates a graveyard asks whether each object is a *card*.
///
/// CR 109.1: a token is not a card. It stays in a graveyard until the next
/// state-based-action check (CR 704.5e), so a count or a choice list taken
/// mid-resolution can see one — and "exile a card from their graveyard",
/// "return target creature card", "for each creature card in your graveyard"
/// must not. Nine cards counted tokens before this was swept; Graveyard Shovel
/// offered one as a choice, and Back from the Brink's guard was defeated by
/// `&&` binding tighter than `||`.
///
/// `state.is_card(id)` is the question. This scans card files for a graveyard
/// filter that never asks it.
#[test]
fn a_card_enumerating_a_graveyard_excludes_tokens() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if !rel.starts_with("cards/") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.starts_with("//") || l.starts_with("///") {
                continue;
            }
            if !l.contains("Zone::Graveyard") {
                continue;
            }
            // A filter over many objects, rather than a check on one known
            // object ("am *I* in a graveyard?", "is my target still there?").
            let enumerates = l.contains(".filter(") || l.contains(".any(")
                || l.contains("objects_in_zone(Zone::Graveyard");
            if !enumerates {
                continue;
            }
            // The guard may be on this line or anywhere in the closure below.
            let window: String = text.lines().skip(n).take(12).collect::<Vec<_>>().join(" ");
            if !window.contains("is_card") {
                offenders.push(format!("{rel}:{}: {l}", n + 1));
            }
        }
    }
    assert!(offenders.is_empty(),
        "CR 109.1: a token in a graveyard is not a card, so a card enumerating \
         a graveyard for \"cards\" must ask `state.is_card(id)`:\n  {}",
        offenders.join("\n  "));
}

/// A double-faced card does not restate its back face's P/T.
///
/// CR 712.8: a transformed permanent has its back face's characteristics, P/T
/// included. That is `back_face_data`'s job, and `effective_power` reads the
/// active face — so a `dynamic_pt` that only says "if transformed, (5, 5)" is
/// the same fact written twice, in two places free to disagree.
///
/// Nineteen of the set's DFCs carried exactly that override. `dynamic_pt` is
/// for a *characteristic-defining ability* — Boneyard Wurm, Splinterfright,
/// Geist-Honored Monk — where the P/T is computed from the game state and
/// there is nothing printed to read.
#[test]
fn no_dfc_restates_its_back_faces_power_and_toughness() {
    let mut offenders = Vec::new();
    for (rel, text) in crate_sources() {
        if !rel.starts_with("cards/") || !text.contains("fn dynamic_pt") {
            continue;
        }
        let Some(at) = text.find("fn dynamic_pt") else { continue };
        let open = text[at..].find('{').map(|i| at + i).unwrap_or(at);
        let mut depth = 0usize;
        let mut end = open;
        for (i, c) in text[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 { end = open + i; break; }
                }
                _ => {}
            }
        }
        let body = &text[open..=end];
        // Keys off nothing but the flip, and answers with a literal pair.
        if body.contains("is_transformed") && !body.contains("counters")
            && !body.contains("objects.values") && !body.contains("objects_in_zone") {
            offenders.push(rel);
        }
    }
    assert!(offenders.is_empty(),
        "a transformed permanent's P/T is its back face's (CR 712.8), which \
         `back_face_data` already declares and `effective_power` already \
         reads. Delete the override:\n  {}",
        offenders.join("\n  "));
}
