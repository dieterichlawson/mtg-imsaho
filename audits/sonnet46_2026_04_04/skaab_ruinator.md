## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

### Code issues

- Engine auto-selects which creature cards to exile as the additional cost, rather than presenting the player with a choice (`mtg-engine/src/engine.rs` lines 1574–1600)
  - Oracle text says: `"As an additional cost to cast this spell, exile three creature cards from your graveyard."`
  - Code does: `exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` then `let exile_candidates: Vec<_> = exile_candidates.into_iter().take(n).collect();` — auto-selects the three highest-power creatures with no player input. There is no field in `Action::CastSpell` to record which specific creature cards the player chose to exile, so player choice is structurally impossible for this cost.

### Tricky interactions checked

- Skaab Ruinator cannot exile itself to pay its own cost (ruling 2011-09-22): PASS — both the eligibility check (engine.rs ~line 547) and the cost-payment code (engine.rs ~line 1577) filter by `o.id != obj.id` / `o.id != *object_id`, excluding the card being cast regardless of its zone at the time.
- Must exile 3 creatures no matter what zone Skaab Ruinator is cast from (ruling 2011-09-22): PASS — both the hand-casting eligibility block (engine.rs ~line 543–554) and the graveyard-casting eligibility block (engine.rs ~line 713–722) enforce the `ExileCreaturesFromGraveyard(3)` check.
- Card returns to graveyard (not exile) when it dies, enabling repeated casting: PASS — `is_cast_from_graveyard = true` prevents `cast_with_flashback` from being set (engine.rs lines 1491–1492, 1636–1638), so `move_spell_after_resolve` sends it to graveyard, not exile.
- Cast-from-graveyard uses normal mana cost {1}{U}{U}, not a flashback cost: PASS — engine.rs lines 680–684 use `data.cost` (the normal cost) when `cast_from_gy` is true and no flashback cost is declared.
- Not castable when fewer than 3 creature cards in graveyard (excluding itself): PASS — eligibility count correctly excludes self by object ID (engine.rs lines 543–553).
- on_resolve correctly moves to battlefield (not graveyard): PASS — `state.move_object(object_id, Zone::Battlefield)` in `on_resolve` (skaab_ruinator.rs line 40) moves it to Battlefield; stack.rs lines 107–110 then skip `move_spell_after_resolve` because the object is no longer in the Stack zone.
- Flying keyword declared: PASS — `keywords: vec![Keyword::Flying]` in `card_data()`.
- Mana cost {1}{U}{U}, P/T 5/6, subtypes Zombie Horror: PASS — all match oracle text.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic cast from hand: exiles 3 creatures and lands on battlefield: `tier15_cards.rs:636` (`skaab_ruinator_exiles_creatures_from_graveyard`)
- Cast from graveyard (can_cast_from_graveyard path), not marked as flashback: `tier15_cards.rs:662` (`skaab_ruinator_cast_from_graveyard`)
- Not castable without 3 creature cards in graveyard: `tier15_cards.rs:706` (`skaab_ruinator_not_castable_without_enough_creatures`)
- Cannot exile itself to pay its own cost: NOT TESTED
- Must exile 3 regardless of casting zone (ruling): NOT TESTED explicitly (covered implicitly by the existing tests but no dedicated test)
- Card returns to graveyard (not exile) after dying and can be cast again: NOT TESTED
- Player chooses which 3 creatures to exile (the auto-select issue): NOT TESTED — and the existing tests do not catch the bug because they always set up exactly 3 creatures in the graveyard, so there is no meaningful choice to be made.
