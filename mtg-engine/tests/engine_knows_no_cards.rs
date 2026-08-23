//! Guard: the engine must not contain card-specific logic.
//!
//! Card rules belong in `src/cards/`. They leaked into the engine twice over,
//! by two different routes:
//!
//! 1. **By name.** `engine.rs` called `registry.get_id_by_name("Evil Twin")`
//!    to re-find a copy's granted ability; `sba.rs` looked up
//!    "Garruk Relentless" to run his state-triggered ability, threshold and
//!    all. Both are now generic mechanisms (`GameObject::copy_grantor`,
//!    `CardBehavior::state_trigger_condition`).
//!
//! 2. **By enum variant.** `PendingEffect` is a closed engine enum, so a card
//!    needing a deferred resolution had to add a variant AND an engine match
//!    arm to execute it. That is how the engine came to contain Ghost
//!    Quarter's library search, Moorland Haunt's Spirit token, Divine
//!    Reckoning's whole choice chain and Elder Cathar's Human bonus — thirteen
//!    variants, each used by exactly one card. `PendingEffect::CardEffect`
//!    replaced them: it routes the chosen target back to the source card's
//!    `resolve_card_effect`.
//!
//! If this test fails, the fix is to add the behaviour to the card, behind a
//! `CardBehavior` hook, not to widen the allowlist.

use std::path::{Path, PathBuf};

/// Engine modules — everything in `src/` that is not `src/cards/`.
fn engine_sources() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).expect("src should be readable").flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// Strip `#[cfg(test)]` modules — test fixtures naturally name cards, and
/// that is not the engine depending on them.
fn without_test_modules(text: &str) -> String {
    let Some(idx) = text.find("#[cfg(test)]") else { return text.to_string() };
    text[..idx].to_string()
}

#[test]
fn engine_does_not_look_up_cards_by_name() {
    let mut violations = Vec::new();
    for path in engine_sources() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = without_test_modules(&std::fs::read_to_string(&path).unwrap());
        for (i, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            // A literal card name handed to the registry is the engine
            // hard-coding one card. `get_id_by_name(card_name)` with a
            // variable — the decklist path — is fine.
            if let Some(rest) = line.split_once("get_id_by_name(").map(|(_, r)| r) {
                if rest.trim_start().starts_with('"') {
                    violations.push(format!("{name}:{}: {}", i + 1, line.trim()));
                }
            }
        }
    }
    assert!(violations.is_empty(),
        "the engine looks up {} card(s) by name:\n{}\n\nGive the behaviour a \
         CardBehavior hook and let the card implement it.",
        violations.len(), violations.join("\n"));
}

/// A `PendingEffect` variant used by exactly one card is *suspicious*: it may
/// be that card's logic sitting in an engine enum. It is not proof — a general
/// primitive can simply have one user today. So each such variant must be
/// listed here with the reason it is general, which forces the judgement to be
/// made and recorded rather than skipped.
///
/// The thirteen variants removed in this pass all failed that test: their
/// engine arms spelled out card text (Graveyard Shovel's 2 life, Elder
/// Cathar's Human bonus, Divine Reckoning's whole choice chain), rather than
/// applying a general rule to parameters the card supplied.
const REVIEWED_SINGLE_USER: &[(&str, &str)] = &[
    ("DebuffUntilEOT", "general: 'target creature gets -X/-X until end of turn', card supplies X"),
    ("CantBlockThisTurn", "general: 'target creature can't block this turn'"),
    ("Mill", "general: 'target player mills N cards', card supplies N"),
    ("DestroyCreature", "general: destroy a creature, card supplies only the log name"),
    ("ReturnToHand", "general: return a permanent to its owner's hand"),
    ("CopyCreature", "general: CR 706 copy effect; the copy grantor is generic"),
];

#[test]
fn single_card_pending_effect_variants_are_justified() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let state = std::fs::read_to_string(src.join("state.rs")).unwrap();

    let start = state.find("pub enum PendingEffect").expect("PendingEffect should exist");
    let body = &state[start..];
    let end = body.find("\n}").expect("enum should close");
    let variants: Vec<String> = body[..end].lines()
        .filter_map(|l| {
            let t = l.trim();
            let first = t.split(|c: char| !c.is_alphanumeric()).next()?;
            if l.starts_with("    ") && !t.starts_with("//")
                && first.chars().next().is_some_and(char::is_uppercase) {
                Some(first.to_string())
            } else {
                None
            }
        })
        .collect();
    assert!(!variants.is_empty(), "failed to parse PendingEffect variants");

    let mut card_files = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out); }
            else if p.extension().is_some_and(|x| x == "rs") { out.push(p); }
        }
    }
    walk(&src.join("cards"), &mut card_files);
    // `cards/helpers.rs` is shared infrastructure for cards, not a card, so a
    // variant used only from there is not "one card's logic".
    card_files.retain(|p| p.file_name().is_some_and(|n| n != "helpers.rs"));

    let single_user = |v: &str| -> Option<String> {
        let needle = format!("PendingEffect::{v}");
        let users: Vec<String> = card_files.iter()
            .filter(|p| std::fs::read_to_string(p).map(|t| t.contains(&needle)).unwrap_or(false))
            .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
            .collect();
        (users.len() == 1).then(|| users[0].clone())
    };

    let mut unjustified = Vec::new();
    for v in &variants {
        if v == "CardEffect" || REVIEWED_SINGLE_USER.iter().any(|(n, _)| n == v) {
            continue;
        }
        if let Some(user) = single_user(v) {
            unjustified.push(format!("  {v} — used only by {user}"));
        }
    }
    assert!(unjustified.is_empty(),
        "{} PendingEffect variant(s) are used by exactly one card and are not \
         justified as general primitives:\n{}\n\nEither move the resolution \
         into that card's `resolve_card_effect` and queue \
         `PendingEffect::CardEffect {{ source_id, key }}`, or — if the variant \
         really is a general rule the card only parameterises — add it to \
         REVIEWED_SINGLE_USER with the reason.",
        unjustified.len(), unjustified.join("\n"));

    // Keep the justification list honest: an entry that gained more users, or
    // whose variant is gone, is stale.
    let stale: Vec<&str> = REVIEWED_SINGLE_USER.iter()
        .filter(|(n, _)| !variants.iter().any(|v| v == n) || single_user(n).is_none())
        .map(|(n, _)| *n)
        .collect();
    assert!(stale.is_empty(),
        "REVIEWED_SINGLE_USER entries no longer apply (variant removed, or it \
         has other users now) — drop them: {stale:?}");
}
