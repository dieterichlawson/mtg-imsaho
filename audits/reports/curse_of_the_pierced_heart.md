## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:50
**Oracle text source**: Scryfall API (via scripts/oracle_lookup.py, cached 2026-04-01)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
**Type line**: Enchantment — Aura Curse
**Status**: ISSUE

### Code issues

1. **Planeswalker damage uses `damage_marked` instead of removing loyalty counters (engine-level bug).**
   When the cursed player controls a planeswalker and the controller chooses to redirect damage to it, the card uses `PendingEffect::DealDamage` with `Target::Object`. The handler in `engine.rs:2179-2191` adds to `obj.damage_marked`, which is creature-style damage tracking. Planeswalkers take damage by having loyalty counters removed (see Stensia Bloodhall at `cards/isd/stensia_bloodhall.rs:90-94` for the correct approach). The SBA check for planeswalker death at `sba.rs:215-220` checks for 0 loyalty counters, so damage dealt this way would never kill a planeswalker. This is a systemic issue in `apply_pending_effect` affecting any card that uses `PendingEffect::DealDamage` against a planeswalker target.

2. **No test coverage for the planeswalker targeting branch.** The only dedicated test (`curse_of_pierced_heart_deals_damage_on_upkeep` in `tier7_cards.rs:176`) covers the no-planeswalker path. There is no test verifying that when the cursed player controls a planeswalker, the choice is presented and damage is correctly applied.

### Card data verification
- **Name**: "Curse of the Pierced Heart" -- matches oracle
- **Mana cost**: {1}{R} (Generic(1), Colored(Red)) -- matches oracle
- **Type line**: Enchantment with subtypes Aura, Curse -- matches oracle "Enchantment - Aura Curse"
- **Oracle text in code** (line 26): `"Enchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls."` -- matches oracle exactly
- **Target requirement**: PlayerOnly -- correct for "Enchant player"
- **on_resolve**: Uses `helpers::resolve_curse` which attaches to target player -- correct

### Tricky interactions checked (min 3)
1. **Trigger only on enchanted player's upkeep**: Line 58 checks `state.active_player != cursed_player` and returns early -- correct. The curse does not trigger on the controller's upkeep if a different player is enchanted.
2. **Controller chooses damage target when planeswalkers present**: Lines 82-99 correctly present a choice to the curse's controller (not the enchanted player) when the cursed player controls planeswalkers. Options include the player and all their planeswalkers.
3. **Multiple curses on same player**: Each curse is a separate object with its own `on_upkeep`, so multiple copies trigger independently -- correct by design.
4. **Curse removed mid-turn**: Line 50 checks `o.zone == Zone::Battlefield` before proceeding -- if the curse is removed before the upkeep trigger resolves, it will not fire.
5. **No planeswalkers shortcut**: Lines 69-81 skip the choice UI and deal damage directly to the player when no planeswalkers are present -- correct optimization.

### Test coverage
- `curse_of_pierced_heart_deals_damage_on_upkeep` (tier7_cards.rs:176): Tests basic damage to enchanted player on upkeep. PASS.
- Curse is also used as a fixture in 3 Bitterheart Witch tests (tier15_cards.rs) testing search-and-attach behavior. These test the resolve/attach path indirectly.
- **Missing**: No test for the planeswalker targeting branch.
