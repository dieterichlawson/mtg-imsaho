## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
{U}: Return this creature to its owner's hand.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Controller-only activation (ruling: "Only Lantern Spirit's controller may activate its ability"): pass. `legal_actions` iterates `state.objects_in_zone(Zone::Battlefield, player)`, which for the Battlefield zone filters by `obj.controller == player`. Only the controller of the spirit will ever see the ability in their legal-action list; the opponent cannot activate it.
- "Owner's hand" vs "controller's hand" when control has been stolen: pass. `move_object(object_id, Zone::Hand)` changes the `zone` field but not the `owner` field. `objects_in_zone(Zone::Hand, player)` filters by `obj.owner == player`, so the card correctly lands in the original owner's hand even when a different player controls it.
- Instant-speed activation: pass. `sorcery_speed_only: false` — the ability can be activated any time the player has priority, not just during their main phase with an empty stack.
- Untapped activation: pass. `requires_tap: false` — the creature does not tap as a cost, matching the oracle text which has no tap symbol in the cost.
- No once-per-turn restriction: pass. `once_per_turn: false` — the ability can be activated multiple times per turn if the creature is returned to hand and then replayed (or otherwise returned to the battlefield), matching oracle text.
- Zone guard on activated_abilities: pass. The method returns the ability only when `o.zone == Zone::Battlefield`, preventing the ability from appearing while the spirit is in hand, graveyard, or exile.
- Ability resolves immediately (not queued as a trigger): pass. `on_activate_ability` calls `state.move_object(object_id, Zone::Hand)` directly; no trigger queuing involved, consistent with an activated ability.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Correct stats (cost, P/T, Flying keyword, Spirit subtype): `activated_abilities.rs:237` (`lantern_spirit_has_correct_stats`)
- Activated ability returns creature to hand: `activated_abilities.rs:248` (`lantern_spirit_returns_to_hand`)
- Controller-only activation: NOT TESTED (no test verifying opponent cannot activate the ability)
- "Owner's hand" behaviour when control stolen: NOT TESTED
- Instant-speed activation: NOT TESTED (test uses sorcery-speed main-phase window; no test activating during opponent's turn or in response to a spell)
- Multiple activations per turn: NOT TESTED
