## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Mana abilities of artifacts blocked**: The mana-ability loop (`engine.rs:277–298`) iterates `objects_in_zone(Zone::Battlefield, player)` and skips any object where `registry.card_data(obj.card_id).card_types.contains(&CardType::Artifact) || obj.card_types.contains(&CardType::Artifact)` is true. Sol Ring's mana ability is correctly suppressed. Per ruling: "No abilities of artifacts can be activated, including mana abilities." — pass

- **Non-mana activated abilities of artifacts blocked**: The non-mana activated-ability loop (`engine.rs:309–407`) applies the same two-part artifact check and `continue`s for any artifact object. Traveler's Amulet, Graveyard Shovel, Grimoire of the Dead, and similar artifacts would all have their abilities suppressed. — pass

- **Triggered abilities not blocked**: Stony Silence's restriction is enforced only inside `legal_actions()` by excluding actions from the generated list; it does not touch the trigger system in `triggers.rs`. Galvanic Juggernaut's "whenever another creature dies, untap" trigger would still fire. Per ruling: "Triggered abilities (starting with 'when,' 'whenever,' or 'at') are unaffected." — pass

- **Only battlefield artifacts affected**: Both the mana-ability loop and the non-mana-ability loop operate exclusively over `objects_in_zone(Zone::Battlefield, player)`. Cards in hand, graveyard, or library are never iterated for activated-ability generation, so artifact cycling or other non-battlefield abilities (if implemented) would not be suppressed. Per ruling: "Stony Silence's ability affects only artifacts on the battlefield." — pass

- **Artifact detection covers tokens**: The artifact check is `registry.card_data(obj.card_id).map(...).unwrap_or(false) || obj.card_types.contains(&CardType::Artifact)` (`engine.rs:280–283` and `engine.rs:317–320`). `create_object` initializes `card_types: Vec::new()`, so real cards are caught by the registry branch; tokens created via `create_token` have `card_types` set explicitly on the runtime object and are caught by the runtime branch. Both paths correctly handled. — pass

- **Non-artifact permanents unaffected**: The `continue` is conditional on `is_artifact`. Creatures, lands, and enchantments that are not artifacts are not skipped. Forest mana ability is correctly left available. — pass

- **Opponent's Stony Silence applies to priority player**: The detection uses `state.objects.values().any(|o| o.zone == Zone::Battlefield && o.name == "Stony Silence")` (`engine.rs:270–272`), which checks all objects regardless of controller. When P1 controls Stony Silence and P0 has priority, P0's artifact abilities are still suppressed. — pass

- **Stony Silence itself must be on the battlefield**: The check at line 271 requires `o.zone == Zone::Battlefield`. A Stony Silence in graveyard, hand, or exile has no effect. — pass

- **Equipment equip ability blocked**: Blazing Torch is an Artifact (`card_types: vec![CardType::Artifact]`). Its equip ability (ability_index 0) is generated when `blazing_torch.activated_abilities` is called with the equipment's own ID. The outer loop visits the Blazing Torch object; `is_artifact` is true; the object is skipped in full. Equip ability is correctly suppressed. — pass

- **Ability granted by equipment to non-artifact creature**: Blazing Torch's "{T}, Sacrifice Blazing Torch: deal 2 damage" is granted to the equipped creature via the attached-object inner loop (`engine.rs:331–338`), where `activated_abilities` is called with the CREATURE's ID (`obj_id`). The Stony Silence check at `engine.rs:316–322` is applied to `obj` (the creature), not to the attached torch. A non-artifact creature equipped with Blazing Torch is not an artifact, so the check does not block the granted ability — correctly allowing it, since the ability now belongs to the creature, not the artifact. — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:

- Mana abilities of artifacts blocked (Sol Ring): `innistrad_simple_cards.rs:595` (`stony_silence_blocks_artifact_mana_abilities`)
- Non-artifact mana abilities not blocked (Forest): `innistrad_simple_cards.rs:625` (`stony_silence_does_not_block_non_artifact_mana`)
- Card data (mana cost, type): `innistrad_simple_cards.rs:586` (`stony_silence_card_data`)
- Non-mana activated abilities of artifacts blocked (Traveler's Amulet, Graveyard Shovel, etc.): NOT TESTED
- Triggered abilities not blocked by Stony Silence: NOT TESTED
- Opponent's Stony Silence suppresses player's artifacts: NOT TESTED
- Artifacts in non-battlefield zones not affected: NOT TESTED
- Artifact detection via runtime `obj.card_types` (token artifacts): NOT TESTED
- Equipment equip ability blocked: NOT TESTED
