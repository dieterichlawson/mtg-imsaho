---
id: evil_twin-05
status: fixed
card: Evil Twin
audit_run_id: 2026-04-19-evil_twin-audit
audit_model: sonnet
audit_tokens: 43910
audit_duration: 1253
fixed_sha: 778ed4738894357d762d118ea082f892dcb0d2c4
fixed_at: 2026-08-23T23:24:11Z
test_file: mtg-engine/tests/copy_effects.rs
fix_note: printed_keywords_of falls back to the object for generic tokens, which have no registry face
---

## Audit Finding

**Oracle text:**
> If the chosen creature is a token, Evil Twin copies the original characteristics of that token as stated by the effect that created the token. Evil Twin is not a token in this case.

**Code:**
> let kw = registry.card_data(o.card_id)
    .map(|d| d.keywords.clone())
    .unwrap_or_default();

**Description:**
Generic tokens (created by `create_token_with_subtypes`) have `card_id = CardId(0)`, a sentinel value not present in the registry. When `CopyCreature` reads keywords for the target, `registry.card_data(CardId(0))` returns `None` and `.unwrap_or_default()` produces an empty `Vec`. Tokens store their keywords directly in `obj.keywords` (e.g., a 1/1 White Spirit token has `obj.keywords = [Flying]`), but the `CopyCreature` handler reads keywords exclusively from the registry. After the copy, Evil Twin has `obj.keywords = []` even if the original token had Flying, Vigilance, or any other keyword. The fix is to fall back to `obj.keywords` when the registry lookup returns `None` (i.e., the target is a generic token). Note: other characteristics (subtypes, card_types, colors) ARE read from the object directly and are correctly preserved.

**Engine path:** mtg-engine/src/engine.rs:3766

**Required check:** 8g

## Tests

### evil_twin_copy_token_preserves_flying
Scenario: Evil Twin copies a 1/1 white Spirit token with Flying; the resulting copy should have Flying but currently has no keywords.

