## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- `is_valid_target` accepts creatures for the land target slot, allowing the card to be cast with no legal land target
  - Oracle text says: `"Destroy target land. Into the Maw of Hell deals 13 damage to target creature."`
  - Code does: `fn is_valid_target` returns `is_land || is_creature` for any object, with no slot distinction. The engine's `valid_targets_for_req` for `TargetRequirement::PermanentWithFilter(_)` ignores the embedded `HasCardType([Land])` filter entirely — it iterates all battlefield permanents and applies only `can_be_targeted` and `behavior.is_valid_target` as filters (engine.rs line 1067–1072). Since `is_valid_target` returns true for any land or creature, `targets1` (the land slot) is populated with both lands AND creatures. The Cartesian product then generates (creature_A, creature_B) cast actions with no land involved.

  Concrete consequence: if there are no lands on the battlefield but 2+ creatures, the engine will offer Into the Maw of Hell as a legal cast action, placing a creature's ObjectId in the land slot. `on_resolve` then calls `try_destroy` on that creature (destroying it instead of a land), which is wrong per the oracle text.

  - Card file: `mtg-engine/src/cards/isd/into_the_maw_of_hell.rs` lines 40–56 (`is_valid_target`)
  - Engine file: `mtg-engine/src/engine.rs` lines 1067–1072 (`valid_targets_for_req` for `PermanentWithFilter`)

### Tricky interactions checked

- **Partial resolution — land becomes illegal before resolution**: `on_resolve` checks `state.get_object(*land_id).map(|o| o.zone == Zone::Battlefield)` before calling `try_destroy`. If the land left the battlefield, it skips that half and still deals 13 damage to the creature. Correct per ruling [2011-09-22]. PASS
- **Partial resolution — creature becomes illegal before resolution**: `on_resolve` checks `obj.zone == Zone::Battlefield` before dealing damage. If the creature left the battlefield, 13-damage is skipped but the land is still destroyed. Correct per ruling [2011-09-22]. PASS
- **Both targets illegal at resolution**: `stack.rs` `is_target_legal` checks each target; `any_legal = targets.iter().any(...)`. If both targets left the battlefield, `any_legal = false` → spell fizzles. Correct per ruling [2011-09-22]. PASS
- **Creature used as land target (slot confusion)**: `targets1` (land slot) includes creatures due to `is_valid_target` returning true for creatures. The cast is offered when no lands are present. FAIL — see Code issues above.
- **Indestructible land**: `try_destroy` checks `has_keyword(id, Keyword::Indestructible, registry)` and returns `DestroyResult::Indestructible` without moving the land. This is correct behavior. PASS
- **Damage amount — 13 not 12 or any other value**: `obj.damage_marked += 13` — matches oracle text exactly. PASS
- **NonCombatDamageDealt event used (not CombatDamageDealt)**: `GameEvent::NonCombatDamageDealt` is pushed — correct for a sorcery dealing non-combat damage. PASS
- **move_spell_after_resolve called**: Called at end of `on_resolve` — correctly moves spell to graveyard (or exile if flashback, though this card has no flashback). PASS
- **Mana cost {4}{R}{R}**: `ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])` — mana value 6, correct. PASS
- **Card type Sorcery**: `card_types: vec![CardType::Sorcery]` — correct. PASS
- **No supertypes or subtypes**: `supertypes: vec![], subtypes: vec![]` — correct for a plain Sorcery. PASS
- **is_target_legal zone check for TwoTargets**: `stack.rs` unwraps `TwoTargets(inner, _)` extracting only the first inner requirement for zone checking. Both `PermanentWithFilter(HasCardType([Land]))` and `Creature` fall to the wildcard `_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack` in the match, so the zone check is identical and correct for both slots. PASS

### Test coverage

- **Card data (type, mana value)**: `innistrad_simple_cards.rs:450` (`into_the_maw_of_hell_card_data`) — TESTED
- **Land is destroyed on resolution**: NOT TESTED
- **13 damage dealt to creature on resolution**: NOT TESTED
- **Partial resolution (land illegal, creature legal)**: NOT TESTED
- **Partial resolution (creature illegal, land legal)**: NOT TESTED
- **Both targets illegal → fizzle**: NOT TESTED
- **Indestructible land survives try_destroy**: NOT TESTED
- **Creature incorrectly usable as land target**: NOT TESTED
