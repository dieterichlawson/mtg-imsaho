---
id: garruk_relentless-01
status: fixed
card: Garruk Relentless
audit_run_id: 2026-04-19-garruk_relentless-audit
audit_model: sonnet
audit_tokens: 43867
audit_duration: 733
fixed_sha: fc41ee775c2558a71e0743f1f9af70a119e52574
fixed_at: 2026-08-23T17:28:19Z
test_file: mtg-engine/tests/characteristics_card_sweep.rs
fix_note: cluster fix: card code now reads characteristics through the GameState accessors (has_card_type / is_creature / has_subtype)
---

## Audit Finding

**Oracle text:**
> −3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.

**Code:**
> let creatures: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller)
    .iter()
    .filter(|o| o.card_types.contains(&CardType::Creature))
    .map(|o| o.id)
    .collect();

**Description:**
The −3 ability identifies battlefield creatures using `o.card_types.contains(&CardType::Creature)`, but the engine's object `card_types` field is empty for most non-token permanents — the default `on_resolve` (and its override in this card) does not populate `card_types` on the object. Only tokens (created by `create_token_with_subtypes` with an explicit `card_types` argument) and the rare card that manually sets `obj.card_types` in `on_resolve` will pass this filter. Regular non-token creatures like Reckless Waif or any vanilla beater will be silently excluded from the buff. The rest of the engine uses `o.power.is_some()` to identify battlefield creatures (see `sba.rs:54`), which works because `power` is copied from card data at object creation. The immediate sibling code in ability 11 already uses `o.card_types.contains(&CardType::Creature) || o.power.is_some()`, but ability 12 omits the fallback. The fix is to add `|| o.power.is_some()` or to adopt the registry-fallback pattern used in the graveyard count just four lines above.

**Engine path:** mtg-engine/src/cards/isd/garruk_relentless.rs:270

**Required check:** 8d

## Tests

### veil_cursed_minus3_buffs_nonttoken_creature
Scenario: Garruk, the Veil-Cursed's −3 ability grants +X/+X and trample to a non-token creature (e.g., a 2/2 with empty card_types), not just to creature tokens.

### veil_cursed_minus3_buffs_token_creature
Scenario: Garruk, the Veil-Cursed's −3 ability also buffs creature tokens (Wolf tokens with explicit card_types=[Creature]) in the same resolution.

