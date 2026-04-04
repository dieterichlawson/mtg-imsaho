## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets -2/-2.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **-2/-2 kills a creature with exactly 2 toughness**: `effective_toughness` drops to 0, SBA 704.5f (zero toughness) moves the creature to graveyard. pass
- **Aura goes to graveyard when enchanted creature dies**: SBA 704.5m (`sba.rs:149-193`) detects `attached_to.is_some()` where target's `zone != Battlefield`, and moves the aura to graveyard on the next SBA iteration. pass
- **Aura fizzles if target leaves battlefield before resolution**: `resolve_aura` in `helpers.rs:20` checks `target.zone == Battlefield` before attaching; if false, calls `move_spell_after_resolve`. pass
- **Continuous effect only applies to attached creature**: `EffectScope::Attached` in `state.rs:700-705` reads `source.attached_to` and checks `target == creature_id`, so the -2/-2 is limited to the single enchanted creature. pass
- **Multiple Dead Weights stack independently**: `continuous_pt_mods` in `state.rs:726-769` iterates all battlefield sources and accumulates each ModifyPT, so each Dead Weight contributes -2/-2 separately. pass
- **Hexproof creature cannot be targeted at cast time**: `valid_targets_for_req` at `engine.rs:1050-1056` calls `can_be_targeted` before allowing a target, blocking hexproof creatures. pass
- **`Enchant` listed as keyword by Scryfall not added to `keywords` vec**: `Enchant` is not in the engine's `Keyword` enum (which contains keyword abilities only); the Aura subtype and `TargetRequirement::Creature` together encode the enchant mechanic. `keywords: vec![]` is correct. pass
- **SBA detects aura-with-no-attached-target**: The filter at `sba.rs:155` requires `attached_to.is_some()`. `resolve_aura` always sets `attached_to` before entering the battlefield, so this state is unreachable in normal play. pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Dead Weight gives -2/-2 and kills a 2/2 via SBA: `innistrad_cards.rs:238` (`dead_weight_kills_small_creature`)
- Aura goes to graveyard when enchanted creature dies (Dead Weight specifically): NOT TESTED (covered indirectly for Holy Strength in `enchantments.rs:36` and `edge_cases.rs:167`)
- Aura fizzles if target dies before resolution: NOT TESTED
- Multiple Dead Weights on same creature stack: NOT TESTED
- Targeting hexproof creature disallowed: NOT TESTED
