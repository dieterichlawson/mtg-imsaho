## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Targeting already-untapped creature: pass — `TargetRequirement::Creature` places no tap-state filter, so an untapped creature is a valid target. Assigning `target.tapped = false` to an already-untapped creature is a no-op; the +2/+4 and reach still apply. Matches the Scryfall ruling: "Spidery Grasp can target a creature that's already untapped. It will still get +2/+4 and gain reach."
- "Until end of turn" expiry: pass — both `until_end_of_turn_effects` and `until_end_of_turn_keywords` are cleared at the cleanup step in `engine.rs` line 3021–3022, which is the correct point per CR 514.
- Target validity at resolution: pass — the code checks `o.zone == Zone::Battlefield` before applying the untap and buff (line 35), so if the creature leaves the battlefield between cast and resolution the effects are safely skipped (fizzle-like behavior consistent with targeting rules).
- Targeting restriction (any creature, no owner/controller filter): pass — `TargetRequirement::Creature` allows any creature on the battlefield regardless of controller; no erroneous controller filter is applied.
- Hexproof interaction: pass — `can_be_targeted` (engine.rs line 758) skips hexproof creatures not controlled by the caster, so a hexproof opponent's creature is correctly un-targetable while your own hexproof creature remains a valid target.
- +2/+4 values: pass — `power_mod: 2, toughness_mod: 4` matches oracle "+2/+4".
- Reach keyword: pass — `Keyword::Reach` pushed to `until_end_of_turn_keywords`; `has_keyword` checks that vec (state.rs line 1036–1039).
- `move_spell_after_resolve` usage: pass — called at line 55, correctly moves the instant to the graveyard after resolution instead of leaving it on the stack or using a raw zone move.

### Test coverage
- Untap a tapped creature, +2/+4, reach: `innistrad_cards.rs:217` (test `spidery_grasp_untaps_and_buffs`) — TESTED
- Target an already-untapped creature (Scryfall ruling 2011-09-22): NOT TESTED
- Effects expire at end of turn: NOT TESTED
- Targeting hexproof creature you control vs. opponent's hexproof creature: NOT TESTED
