## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +1/+1 and has "{B}: Regenerate this creature."
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- `EffectScope::Attached` for +1/+1: state.rs `creature_applies_to` checks `source_id.attached_to == creature_id`, correctly granting the bonus only to the enchanted creature — pass
- Guard preventing ability from appearing on the aura itself: `power.is_some()` in `activated_abilities` returns `[]` when `object_id` is the aura (which has `power: None`), preventing duplicate exposure — pass
- Engine passes creature ID (not aura ID) to `on_activate_ability`: engine.rs lines 1782–1803 confirm `*object_id` (the creature) is passed, so `regeneration_shields += 1` applies to the creature — pass
- Regeneration shield consumed by `try_destroy`: `destruction.rs` checks `regeneration_shields > 0`, calls `regenerate()` (taps, clears damage, decrements shield, removes from combat) — pass
- Unused shields cleared at end of turn: engine.rs cleanup step sets `obj.regeneration_shields = 0` for all battlefield objects — pass
- Instant-speed activation: `sorcery_speed_only: false` is correct; MTG regeneration can be activated any time — pass
- Aura falls off when creature leaves battlefield: SBA rule 704.5m in sba.rs checks `attached_to` target zone != Battlefield and moves the unattached aura to graveyard — pass
- Target validation on resolution: `resolve_aura` helper checks target is still on battlefield; if not, calls `move_spell_after_resolve` — pass
- No once-per-turn restriction: `once_per_turn: false` is correct; multiple shields may be stacked — pass
- "Enchant creature" prefix in oracle_text field: missing from `oracle_text` string (`"Enchanted creature gets..."` instead of `"Enchant creature\nEnchanted creature gets..."`); consistent with other auras in the engine (e.g. Spectral Flight); targeting is correctly enforced by `TargetRequirement::Creature`; display-only omission, no gameplay impact — pass
- Aura controller vs. creature controller activation rights: engine iterates `objects_in_zone(player)` for each player; aura ability is exposed via the creature iteration (creature's controller), not via the aura iteration (which returns `[]` due to `power.is_some()` guard); correct per MTG rules — pass
- Regeneration shield from deathtouch damage: SBA correctly routes through `try_destroy` which checks shields before destroying — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- +1/+1 continuous effect while attached: `innistrad_cards.rs:343` (`skeletal_grimace_gives_plus_one_plus_one`)
- {B}: Regenerate ability is available and adds a shield: `card_mechanics.rs:1028` (`skeletal_grimace_grants_regenerate_ability`)
- Regeneration shield saves creature from lethal damage (SBA path): `card_mechanics.rs:1055` (`skeletal_grimace_regeneration_saves_from_lethal`)
- Regeneration shield saves creature from Doom Blade (destroy spell path): `card_mechanics.rs:1093` (`skeletal_grimace_regeneration_vs_doom_blade`)
- Regeneration shield saves creature from deathtouch damage: `card_mechanics.rs:1129` (`skeletal_grimace_regeneration_vs_deathtouch`)
- Aura falls off when enchanted creature dies: NOT TESTED (for Skeletal Grimace specifically; general SBA behavior is tested elsewhere)
- Multiple shields from multiple {B} activations: NOT TESTED
- Ability not exposed on the aura object itself: NOT TESTED
