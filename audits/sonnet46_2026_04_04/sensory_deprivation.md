## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets -3/-0.
**Type line**: Enchantment — Aura
**Status**: ISSUE

### Code issues

- Engine does not check hexproof when evaluating target legality at resolution time
  - Oracle text says: `Enchant creature` (targeting rules apply; per CR 608.2b, a target that gains hexproof after being chosen is an illegal target at resolution, and the spell is countered by game rules)
  - Code does: `is_target_legal` in `mtg-engine/src/stack.rs:8-41` only checks zone legality (`obj.zone == Zone::Battlefield`), not whether the target has gained hexproof since the spell was cast. `resolve_aura` in `mtg-engine/src/cards/helpers.rs:18-31` similarly checks only `o.zone == Zone::Battlefield`. As a result, if a creature gains hexproof (e.g., via Ranger's Guile) after Sensory Deprivation is cast but before it resolves, the aura still attaches instead of fizzling. This is a general engine bug documented in `mtg-engine/tests/spell_fizzle.rs:185-225` (`bolt_target_gains_hexproof_before_resolution`), which explicitly notes "The current engine does NOT check target legality on resolution."

### Tricky interactions checked

- **-3/-0 power vs toughness**: `ModifyPT { power: -3, toughness: 0, scope: EffectScope::Attached }` correctly applies -3 to power and 0 (no change) to toughness. Matches "gets -3/-0." — PASS
- **Continuous re-evaluation ("as long as" not needed but confirmed dynamic)**: `effective_power` and `effective_toughness` in `state.rs:851-935` call `continuous_pt_mods` on every evaluation, so the debuff is always current and doesn't need a snapshot. — PASS
- **Aura falling off when host creature leaves battlefield**: SBA rule 704.5m is implemented in `sba.rs:149-193`. The code scans for auras with `attached_to.is_some()` whose target is no longer on the battlefield and moves them to graveyard. When the enchanted creature moves to another zone, the aura's `attached_to` pointer (to the creature's old ID) correctly triggers this SBA. — PASS
- **Hexproof at cast time**: `can_be_targeted` in `engine.rs:758-768` checks hexproof and excludes hexproof creatures from valid targets when the caster is an opponent. Sensory Deprivation uses `TargetRequirement::Creature` which is filtered through `can_be_targeted`. — PASS
- **Hexproof gained after cast (resolution fizzle)**: As noted above, hexproof gained between cast and resolution is not rechecked. The aura attaches when it should fizzle. — FAIL (engine bug)
- **Target leaves battlefield between cast and resolution**: `is_target_legal` returns false if the creature's zone is no longer `Battlefield`, causing the spell to fizzle via `move_spell_after_resolve`. `resolve_aura` also has a zone check as a secondary guard. — PASS
- **"Enchant creature" targeting restriction (no subtype restriction)**: `TargetRequirement::Creature` correctly allows any creature; no additional filter needed and none is applied. — PASS
- **Multiple auras on the same creature**: `continuous_pt_mods` iterates all battlefield sources independently, so two copies of Sensory Deprivation would each contribute -3/-0 for a total of -6/-0. This is correct. — PASS
- **Creature with 0 power from other effects + Sensory Deprivation**: Power can go negative (code uses `i32`). The engine allows negative effective power, which is correct per MTG rules. — PASS

### Test coverage

- **Basic -3/-0 effect**: `mtg-engine/tests/innistrad_cards.rs:258` (`sensory_deprivation_reduces_power`) — TESTED
- **Toughness unchanged**: `mtg-engine/tests/innistrad_cards.rs:268` (asserts `effective_toughness == Some(3)`) — TESTED
- **Aura falling off when host dies**: NOT TESTED
- **Target leaves battlefield before resolution (fizzle)**: Covered generically at `mtg-engine/tests/spell_fizzle.rs:160-182` for auras, not Sensory Deprivation specifically — NOT TESTED (for this card)
- **Hexproof gained after cast / resolution fizzle**: Documented as known engine limitation at `mtg-engine/tests/spell_fizzle.rs:185-225` — NOT TESTED for auras specifically
- **Multiple Sensory Deprivations on same creature**: NOT TESTED
