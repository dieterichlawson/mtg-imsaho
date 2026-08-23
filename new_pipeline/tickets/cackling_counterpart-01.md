---
id: cackling_counterpart-01
status: fixed
card: Cackling Counterpart
audit_run_id: 2026-04-18-cackling_counterpart-audit
audit_model: sonnet
audit_tokens: 15636
audit_duration: 272
test_run_id: 2026-04-18-cackling_counterpart-01-test
test_model: sonnet
test_tokens: 3861
test_duration: 91
test_file: mtg-engine/tests/pipeline_bugs_cackling_counterpart_01.rs
tested_sha: 2d2298190537190f2888b805b1e1d449d1ee6b90
tested_at: 2026-04-19T02:57:57Z
fix_run_id: 2026-04-18-cackling_counterpart-01-fix
fix_model: sonnet
fix_tokens: 2826
fix_duration: 65
fixed_sha: f2df4300795af85f60ca425ba14cf1db4a64908c
fixed_at: 2026-04-19T03:28:50Z
---

## Audit Finding

**Oracle text:**
> If the copied creature is a token, the token that's created copies the original characteristics of that token as stated by the effect that created the token.

**Code:**
> let (colors, keywords, card_types, subtypes) = registry.card_data(card_id)
            .map(|d| {
                // Derive colors from mana cost.
                let mut cols = Vec::new();
                if let Some(ref cost) = d.cost {
                    for sym in &cost.symbols {
                        if let crate::types::ManaSymbol::Colored(c) = sym {
                            if !cols.contains(c) {
                                cols.push(*c);
                            }
                        }
                    }
                }
                (cols, d.keywords.clone(), d.card_types.clone(), d.subtypes.clone())
            })
            .unwrap_or_default();

**Description:**
`create_token_copy` (state.rs:504) reads colors, keywords, card_types, and subtypes exclusively from `registry.card_data(card_id)`. Generic tokens — those created by `create_token_with_subtypes`, such as a 2/2 black Zombie or a 1/1 white Human — are assigned `card_id = CardId(0)` as a sentinel value not present in the registry. When Cackling Counterpart copies such a token, `registry.card_data(CardId(0))` returns `None` and `.unwrap_or_default()` silently supplies empty vectors for all four fields. The resulting copy token retains the correct name and P/T (read from the source object at lines 500–502) but enters the battlefield with no creature type, no colors, no subtypes, and no keywords. A 2/2 black Zombie token copied by Cackling Counterpart produces a colorless 2/2 with no types that isn't even classified as a creature — it would immediately die to SBA 704.5f (creature with 0 toughness) if toughness were 0, or just be a non-creature permanent. The fix must read colors, card_types, keywords, and subtypes from the source *object's* fields (`obj.colors`, `obj.card_types`, etc.) when the registry lookup fails.

**Engine path:** mtg-engine/src/state.rs:504

**Required check:** 8g

**Affected cards:**
- Cackling Counterpart

## Tests

### cackling_counterpart_copy_of_zombie_token_preserves_card_types
Scenario: Cackling Counterpart copies a 2/2 black Zombie token; the resulting token should have card_types=[Creature] and subtypes=["Zombie"].

### cackling_counterpart_copy_of_zombie_token_preserves_color
Scenario: Cackling Counterpart copies a 2/2 black Zombie token; the resulting token should be black.

## Test Run Results

- **cackling_counterpart_copy_of_zombie_token_preserves_card_types** — confirmed
  - assertion: A token copy of a 2/2 black Zombie should have card_types=[Creature], but got []. Bug: create_token_copy reads card_types from registry.card_data(CardId(0)) which returns None, so unwrap_or_default() yields an empty vec.
- **cackling_counterpart_copy_of_zombie_token_preserves_color** — confirmed
  - assertion: A token copy of a 2/2 black Zombie should be black, but got colors=[]. Bug: create_token_copy derives colors from registry.card_data(CardId(0)) which returns None, so unwrap_or_default() yields an empty vec, making the copy colorless.

## Fix Result

**Status:** fixed

In mtg-engine/src/state.rs create_token_copy, changed .unwrap_or_default() to .unwrap_or_else(|| (obj_colors, obj_keywords, obj_card_types, obj_subtypes)) on the registry.card_data lookup. Generic tokens (2/2 black Zombie, etc.) use CardId(0) as a sentinel not present in the registry, so the lookup returns None. Previously the default fallback yielded empty vecs, making copies colorless and typeless. Now when the registry lookup fails, colors/keywords/card_types/subtypes are read directly from the source object's fields. Both failing tests pass: the Zombie token copy correctly has card_types=[Creature], subtypes=[Zombie], and colors=[Black]. All 16 llm_conversation tests and the full cargo test suite pass with no failures.

