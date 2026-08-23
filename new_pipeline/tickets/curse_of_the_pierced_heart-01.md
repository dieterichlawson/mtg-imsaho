---
id: curse_of_the_pierced_heart-01
status: fixed
card: Curse of the Pierced Heart
audit_run_id: 2026-04-19-curse_of_the_pierced_heart-audit
audit_model: sonnet
audit_tokens: 30078
audit_duration: 606
fixed_sha: fc41ee775c2558a71e0743f1f9af70a119e52574
fixed_at: 2026-08-23T17:28:19Z
test_file: mtg-engine/tests/characteristics_card_sweep.rs
fix_note: cluster fix: card code now reads characteristics through the GameState accessors (has_card_type / is_creature / has_subtype)
---

## Audit Finding

**Oracle text:**
> this Aura deals 1 damage to that player or a planeswalker that player controls

**Code:**
> .filter(|o| o.zone == Zone::Battlefield && o.controller == cursed_player
                && o.card_types.contains(&CardType::Planeswalker))

**Description:**
In `on_upkeep`, the planeswalker search filters on `o.card_types.contains(&CardType::Planeswalker)`. Because `create_object` initializes `card_types: Vec::new()` for every non-token permanent, this predicate always returns `false` for non-token planeswalkers on the battlefield. Whenever the cursed player controls a non-token planeswalker, the `planeswalkers` vector is always empty, the `if planeswalkers.is_empty()` branch is always taken, and 1 damage is silently dealt to the player without ever presenting the Curse controller with the choice to redirect it to the planeswalker. The 'or a planeswalker that player controls' option in the oracle text is therefore dead code for every non-token planeswalker. The engine's own `generate_ability_targets` for `PlayerOrPlaneswalker` (engine.rs:2043-2045) already uses the correct two-step pattern: `obj.card_types.contains(&CardType::Planeswalker) || registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Planeswalker))`. The fix is to apply the same registry fallback in the Curse's in-handler planeswalker scan.

**Engine path:** mtg-engine/src/cards/isd/curse_of_the_pierced_heart.rs:61-65

**Required check:** 8d

## Tests

### curse_pierced_heart_offers_planeswalker_choice
Scenario: Curse of the Pierced Heart enchants player B, who controls a non-token planeswalker; at the beginning of player B's upkeep the Curse trigger resolves and player A (the Curse controller) should be presented a choice between dealing 1 damage to player B or to the planeswalker — currently no choice is offered and the damage always goes to player B.

### curse_pierced_heart_redirected_damage_hits_planeswalker
Scenario: Same setup as above; when player A chooses the planeswalker as the damage target, the planeswalker should receive 1 damage (loyalty reduced) and player B's life total should be unchanged.

