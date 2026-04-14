//! Failing tests for draft-related bugs documented in
//! `audits/AUDIT_BUGS.md`.
//!
//! Bugs covered in this file:
//! - Bug 17-004: `validate_deck` has a no-op DFC fallback that
//!   silently lets the model include more copies of a card than
//!   it drafted.
//! - Bug H9: the deck-builder validator's error messages are
//!   structured for humans, not for a retry loop that wants
//!   actionable diffs.

use std::collections::HashMap;

use mtg_draft::deckbuilding::validate_deck;

/// Bug 17-004 (`audits/AUDIT_BUGS.md)`: `validate_deck` counts
/// `pool_counts[name]` but then, if `used_counts[name] > available`,
/// checks a no-op DFC fallback:
///
/// ```
/// let front_match = pool_counts.keys().find(|&&k| k == name);
/// if front_match.is_none() {
///     return Err(...);
/// }
/// ```
///
/// Since `pool_counts` was populated from the same front-face
/// names, `pool_counts.keys().find(|&&k| k == name)` IS `Some(_)`
/// whenever `pool_counts.get(name)` is `Some(_)`. So once any copy
/// of a card is in the pool, the validator silently accepts an
/// arbitrary number of copies in the maindeck.
///
/// We build a 40-card deck where Dearly Departed appears twice
/// but the drafted pool only has one. Current behavior: accepted.
/// Correct behavior: rejected.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug 17-004 is fixed.
#[test]
fn bug_17_004_validate_deck_rejects_more_copies_than_drafted() {
    // Pool has exactly ONE Dearly Departed alongside 40 fillers.
    let mut pool: Vec<String> = (0..40)
        .map(|i| format!("Filler Card {i}"))
        .collect();
    pool.push("Dearly Departed".into());

    // Maindeck tries to use TWO Dearly Departed + 38 fillers.
    let mut maindeck: Vec<String> = (0..38)
        .map(|i| format!("Filler Card {i}"))
        .collect();
    maindeck.push("Dearly Departed".into());
    maindeck.push("Dearly Departed".into());

    let mut lands = HashMap::new();
    lands.insert("Plains".to_string(), 0);

    // Add enough basics to reach a valid total. validate_deck
    // requires total >= 40 (maindeck.len() + land_count). The
    // maindeck has 40 entries already, but the second Dearly
    // Departed must be rejected before the total check runs.
    let result = validate_deck(&pool, &maindeck, &lands);

    assert!(
        result.is_err(),
        "validate_deck should reject a maindeck that uses more copies \
         of a card than were drafted. Bug 17-004: the no-op DFC \
         fallback accepts any number of copies once the card is in \
         the pool. Got: {result:?}",
    );
}

/// Bug H9 (`audits/AUDIT_BUGS.md)`: The deck-builder validator's error
/// messages don't help the retry loop converge. The model repeats
/// the same invalid deck because the error text doesn't say "here's
/// the specific delta to fix".
///
/// The actionable-diff requirement is soft and partly subjective,
/// but we can assert a concrete invariant: the error message for an
/// "over-quota card" case should contain both the card name AND
/// the expected count (or a delta), not just a generic "not in
/// pool" string. Today the error is literally
/// `'{name}' is not in your drafted pool.` — which is misleading
/// (the card IS in the pool, just not enough times).
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug H9 is fixed.
#[test]
fn bug_h9_deckbuilder_error_names_the_actual_problem() {
    let mut pool: Vec<String> = (0..40)
        .map(|i| format!("Filler Card {i}"))
        .collect();
    pool.push("Dearly Departed".into());

    let mut maindeck: Vec<String> = (0..38)
        .map(|i| format!("Filler Card {i}"))
        .collect();
    // 4 Dearly Departed → 40 cards total.
    for _ in 0..4 {
        maindeck.push("Dearly Departed".into());
    }

    let lands = HashMap::new();

    let result = validate_deck(&pool, &maindeck, &lands);
    let msg = result.err().unwrap_or_default();

    // The message should mention the count (4) or the available
    // number (1) — something more actionable than "not in pool".
    let has_count_info = msg.contains('4') || msg.contains("1 copy") || msg.contains("only");
    assert!(
        has_count_info,
        "validate_deck's error message for over-quota cards should \
         tell the retry-loop LLM exactly how many copies were \
         allowed vs requested, so it can fix the delta. Bug H9: \
         today the error says 'not in your drafted pool' — \
         misleading because the card IS in the pool, just not \
         enough times. msg = {msg:?}",
    );
}
