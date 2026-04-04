## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You gain 2 life for each creature card in your graveyard.
Flashback {2}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Creature card" detection via `power.is_some()`**: In `on_resolve`, the code counts graveyard objects with `o.power.is_some()` as a proxy for "creature card." This is correct because `setup_game` always creates all cards with `card_data.power` set, and only creature cards have non-None power in `CardData`. Non-creature cards (instants, sorceries, lands) are created with `power: None`. No creature card can reach the graveyard with `power: None` through normal gameplay. PASS
- **Spell itself excluded from count**: The filter `o.id != object_id` correctly prevents the resolving Gnaw to the Bone from counting itself, even if it were somehow in the graveyard zone (it's actually on the Stack during resolution, but the filter covers both cases). PASS
- **"Your graveyard" owner filter**: Code uses `o.owner == controller` where `controller` is derived from the spell's controller at resolution time. Per MTG rules, "your graveyard" means the graveyard belonging to the spell's controller (cards they own in the graveyard zone), so this is correct. PASS
- **Flashback cost matches mana cost**: The oracle text specifies `Flashback {2}{G}` and the normal cast cost is also `{2}{G}`. Both `cost` and `flashback_cost` fields in `card_data()` are set to `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])`. PASS
- **`move_spell_after_resolve` used (not `move_object`)**: The card correctly calls `state.move_spell_after_resolve(object_id)` at the end of `on_resolve`, which sends it to exile when `cast_with_flashback == true` and to graveyard otherwise. PASS
- **`cast_with_flashback` flag set by engine**: Engine code at line 1636-1638 sets `obj.cast_with_flashback = true` whenever the spell is cast from the graveyard (`is_flashback = in_graveyard && !is_cast_from_graveyard`). Combined with `move_spell_after_resolve`, flashback exile is correctly handled. PASS
- **Countered flashback spells are exiled**: The ruling "A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way." The `cast_with_flashback` flag is set at cast time, before the spell can be countered, so countering the flashback-cast Gnaw to the Bone would still result in exile. PASS (verified by `flashback_spell_countered_is_exiled` test, though not specifically for Gnaw to the Bone)
- **Life gain of 0 (no creatures in graveyard)**: If `creature_count == 0`, `life_gain == 0` and the `if life_gain > 0` guard skips the life change event. No life is gained and no event is emitted, which is correct. PASS
- **No targeting required**: Gnaw to the Bone has no targets. The `TargetRequirement::None` path in the engine correctly handles this. The `seen_untargeted_flashbacks` deduplication in `legal_actions` ensures only one CastSpell action is offered per card_id, which is correct for UX. PASS
- **Flashback timing restriction**: The ruling "you must still follow any timing restrictions... you can cast a sorcery using flashback only when you could normally cast a sorcery." Gnaw to the Bone is an Instant, so it can be cast at instant speed via flashback — no restriction applies. The engine's timing checks for instants allow this. PASS

### Test coverage
- Normal cast life gain (3 creatures → +6 life): `flashback.rs:324` (gnaw_to_the_bone_gains_life) TESTED
- Cast via flashback with exile after resolution: NOT TESTED specifically for Gnaw to the Bone (generic flashback exile tested in flashback.rs:86 and flashback.rs:471 for other cards)
- Correct count when 0 creature cards in graveyard: NOT TESTED
- Life gain counts only controller's graveyard (not opponent's): NOT TESTED
- Flashback cost is {2}{G} (same as normal cost): NOT TESTED explicitly
- Countered flashback spell is exiled: NOT TESTED specifically for Gnaw to the Bone (generic case covered by flashback.rs:128)
