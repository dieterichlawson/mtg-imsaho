## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues

- Activated ability description in `mtg-engine/src/cards/isd/full_moons_rise.rs` line 57 says "Wolf and Werewolf" when oracle says "Werewolf" only. This description is embedded directly into the log message emitted by the engine at `mtg-engine/src/engine.rs:1806` (`format!("p{} activated ability on {}: {}", player.0, name, ab.description)`), producing an inaccurate log that claims the ability regenerates Wolf creatures when the oracle text limits the effect to Werewolves.
  - Oracle text says: `Sacrifice this enchantment: Regenerate all Werewolf creatures you control.`
  - Code does: `description: "Sacrifice: Regenerate all Wolf and Werewolf creatures you control".into()`

### Tricky interactions checked

- **Sacrifice-before-effect ordering**: The engine pays `SacrificeCost::SacrificeThis` (moves Full Moon's Rise to graveyard) before calling `on_activate_ability`. Inside `on_activate_ability`, the code retrieves the controller via `state.get_object(object_id)` — this succeeds because `move_object` only changes the zone field, it does not remove the object from the HashMap. Regeneration shields are then applied to battlefield Werewolves only. Correct.
- **Continuous effects stop immediately on sacrifice**: `continuous_pt_mods` and `has_continuous_effect` in `state.rs` both iterate `self.objects.values()` and skip sources where `source.zone != Zone::Battlefield`. Once Full Moon's Rise is moved to graveyard as part of the cost payment, all creatures immediately lose their +1/+0 and trample bonuses. This matches the ruling [2011-09-22].
- **Transformed DFC Werewolves — continuous effects**: `matches_filter` for `HasSubtype("Werewolf")` in `state.rs:654` uses back-face subtypes when `creature.is_transformed` is true (`behavior.back_face_data().subtypes`), correctly identifying transformed Werewolves. Pass.
- **Transformed DFC Werewolves — on_activate_ability filter**: The inline filter reads `registry.card_data(o.card_id)` (front-face data). All Innistrad front-face Werewolves have "Human Werewolf" as their subtype, so "Werewolf" is present in the front-face subtype list regardless of transform state. The filter still correctly identifies them in practice. Pass.
- **Token Werewolves**: Both the continuous effects (via `matches_filter` which falls through to `creature.subtypes.iter().any(|s| s == subtype)`) and the `on_activate_ability` filter (which combines `o.subtypes` with `registry.card_data(o.card_id).subtypes`) check object-level subtypes. A Wolf token with subtype "Werewolf" would be covered by both paths. Pass.
- **"Werewolf" only, not "Wolf and Werewolf"**: The `continuous_effects` at lines 28–43 use `CreatureFilter::HasSubtype("Werewolf".into())` only. The regeneration filter at line 84 checks `s == "Werewolf"` only. Game behavior is correct per oracle (Wolf-only creatures are excluded). The *description* string is the only mismatch (flagged above).
- **Instant-speed activation during combat**: `sorcery_speed_only: false` on line 63. The engine only skips an ability if `ab.sorcery_speed_only && !is_sorcery_speed`. Since this is false, the ability can be activated before combat damage, consistent with the [2011-09-22] ruling.
- **Regeneration shields expiry at cleanup**: `engine.rs` Step::Cleanup at line 3031 sets `obj.regeneration_shields = 0` for all battlefield permanents. Shields do not persist across turns. Pass.
- **`behavior_card_id` lookup after sacrifice**: After sacrifice, `activated_abilities` returns `vec![]` (object is no longer on the battlefield), so the engine falls into the "attached aura" branch, finds no aura, and falls back to `card_id` (Full Moon's Rise). `on_activate_ability` is still called on the correct behavior. Pass (works by fallback, not by design).

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Card data (types, cost, continuous_effects present): `innistrad_simple_cards.rs:572`
- Continuous effect grants +1/+0 to Werewolf creatures: NOT TESTED
- Continuous effect grants trample to Werewolf creatures: NOT TESTED
- Sacrifice ability places regeneration shields on Werewolf creatures: NOT TESTED
- Wolf creatures (non-Werewolf) correctly excluded from both effects: NOT TESTED
- Instant-speed activation during combat (ruling [2011-09-22]): NOT TESTED
- Continuous effects cease when Full Moon's Rise leaves battlefield: NOT TESTED
- Regeneration shields properly save Werewolves from destruction: NOT TESTED (regeneration engine tested elsewhere in `card_mechanics.rs`)
- Regeneration shields expire at cleanup: NOT TESTED for this card specifically (general test at `card_mechanics.rs:840`)
- Token Werewolves receive regeneration shield: NOT TESTED
- Transformed DFC Werewolf (is_transformed=true) gets +1/+0 from continuous effect: NOT TESTED
