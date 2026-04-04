## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
**Type line**: Creature — Spirit Bird
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Targeting yourself vs. opponent**: Oracle says "target player" with no restriction; `TargetRequirement::PlayerOnly` in `generate_ability_targets` (engine.rs:1314) iterates over all non-lost players including the controller — both self and opponent are valid targets. pass
- **Empty library**: If `library_order.is_empty()`, the card returns early without milling or applying any effect. Since milling an empty library yields X=0, the +0/+0 that would result has no effect; early return is correct. pass
- **Mana value 0 (land milled)**: `mana_value()` in `types.rs:67` returns the sum of generic + colored symbols; lands have `cost: None`, so `and_then(|b| b.card_data().cost.as_ref().map(...))` returns `None`, and `unwrap_or(0)` gives 0. Code then skips `if mana_value > 0`, producing +0/+0 — correct. pass
- **X-cost spells milled from library**: `mana_value()` maps `ManaSymbol::X` to 0 (types.rs:72); per rules, X is 0 outside the stack. pass
- **Until-end-of-turn expiry**: `until_end_of_turn_effects.clear()` is called at `Step::Cleanup` (engine.rs:3021). `effective_power` and `effective_toughness` both sum over `until_end_of_turn_effects`. The +X/+X expires correctly. pass
- **Instant vs. sorcery speed**: Activated ability has `sorcery_speed_only: false` and `requires_tap: false`; can be activated anytime the controller has priority. Oracle imposes no timing restriction. pass
- **Source leaving battlefield between activation and resolution**: Irrelevant in this engine — `on_activate_ability` is called synchronously (no stack for activated abilities), so Mindshrieker cannot leave between activation and resolution. The defensive `zone == Battlefield` check before applying the effect is belt-and-suspenders. pass
- **Mana cost lookup path consistency**: Library_order is maintained with `remove(0)` (matching how `draw_top_card` works at engine.rs:2707-2716). After removal, `move_object(milled_card_id, Zone::Graveyard)` updates the zone in `state.objects`. The subsequent `get_object(milled_card_id)` still finds the card (now in Zone::Graveyard) and retrieves its `card_id` for the registry lookup. pass
- **Targeting hexproof player**: `can_target_player` (engine.rs:772) blocks targeting a hexproof opponent; self-targeting is never blocked. Correct per rules. pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card data (cost 2MV, P/T 1/1, keywords Flying, subtypes Spirit/Bird): `mtg-engine/tests/tier10_cards.rs:66` (`mindshrieker_card_data`)
- Mills card to graveyard and creature gets +X/+X: `mtg-engine/tests/tier10_cards.rs:78` (`mindshrieker_mills_and_pumps`)
- Mana value 0 (land milled) gives no pump: `mtg-engine/tests/tier10_cards.rs:112` (`mindshrieker_mills_land_no_pump`)
- +X/+X expires at end of turn: NOT TESTED
- Targeting controller (self-mill): NOT TESTED
- Targeting hexproof player is blocked: NOT TESTED
- Ability available at instant speed (not just sorcery speed): NOT TESTED
- X-cost spell milled from library has X=0: NOT TESTED
