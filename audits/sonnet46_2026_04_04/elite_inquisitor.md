## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: First strike, vigilance
Protection from Vampires, from Werewolves, and from Zombies
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues

- Protection's targeting restriction is not enforced in the engine's ability-targeting path (`mtg-engine/src/engine.rs:758-768` and `mtg-engine/src/engine.rs:1305-1312`)
  - Oracle text says: `Protection from Vampires, from Werewolves, and from Zombies`
  - Per MTG rule 702.16c, protection means (among other things) the protected permanent "can't be the target of spells or abilities from sources" with the stated quality.
  - Code does: `can_be_targeted` in `engine.rs` at lines 758–768 only checks hexproof; it does not check whether the source of the ability is a Vampire, Werewolf, or Zombie:
    ```rust
    fn can_be_targeted(state: &GameState, target_id: ObjectId, caster: PlayerId, registry: &CardRegistry) -> bool {
        if state.has_keyword(target_id, Keyword::Hexproof, registry) {
            ...
        }
        true
    }
    ```
    `generate_ability_targets` at lines 1305–1312 likewise calls `can_be_targeted` and `matches_ability_target_filter` with no check against the source object's subtypes:
    ```rust
    TargetRequirement::CreatureWithFilter(filter) => {
        state.all_objects_in_zone(Zone::Battlefield).iter()
            .filter(|o| o.power.is_some())
            .filter(|o| can_be_targeted(state, o.id, controller, registry))
            .filter(|o| matches_ability_target_filter(state, o, filter, controller, source_id, registry))
            ...
    }
    ```
  - Concrete cases broken by this:
    1. **Olivia Voldaren** (`mtg-engine/src/cards/isd/olivia_voldaren.rs`) is a Vampire with activated ability `{1}{R}: Deal 1 damage to another target creature`. Olivia can incorrectly target the Elite Inquisitor; protection from Vampires should make it an illegal target.
    2. **Nightfall Predator** (back face of `mtg-engine/src/cards/isd/daybreak_ranger.rs`) is a Werewolf with activated ability `{R}, {T}: This creature fights target creature`. Nightfall Predator can incorrectly target the Elite Inquisitor.
    3. **Grimgrin, Corpse-Born** (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs`) is a Zombie whose `on_attacks` triggered ability builds its target list with no protection check (`state.objects_in_zone(Zone::Battlefield, defender).iter().filter(|o| o.power.is_some())`), and passes it directly to `present_target_choice`. When the Elite Inquisitor is the only defending creature and the target auto-applies (line 129–133 of `helpers.rs`), Grimgrin will destroy it — a direct protection violation.

### Tricky interactions checked

- **Protection's Damage restriction (D)**: PASS — `deal_damage_to_creature` in `combat.rs:440` calls `has_protection_from_creature`, which checks all `ProtectionFromSubtype` effects; Vampire/Werewolf/Zombie combat damage to the Inquisitor is correctly prevented.
- **Protection's Blocking restriction (B)**: PASS — `can_block_attacker` in `combat.rs:696-701` calls `has_protection_from_creature` for both attacker and blocker; Vampires/Werewolves/Zombies cannot legally block or be blocked against the Inquisitor.
- **Protection's Targeting restriction (T)**: FAIL — see issue above. `can_be_targeted` checks only hexproof; protection from subtype is not checked when generating ability targets or constructing trigger target lists.
- **Protection's Enchanting restriction (E)**: Not directly testable in this card set (no Vampire/Werewolf/Zombie Aura cards exist), but the same underlying gap in `can_be_targeted` would apply if such a card existed.
- **First strike keyword**: PASS — `Keyword::FirstStrike` is in the `keywords` vec; `deal_damage_step` in `combat.rs:186-190` gates first-strike dealers correctly.
- **Vigilance keyword**: PASS — `Keyword::Vigilance` is in the `keywords` vec; `declare_attackers` in `combat.rs:19-20` skips tapping when vigilance is present.
- **"back_face_data" for transformed attackers checking protection subtypes**: PASS for this card — `get_subtypes` in `combat.rs` reads `registry.card_data().subtypes` (front face) for the attacker. Daybreak Ranger's front-face data already includes `"Werewolf"` as a subtype, so protection from Werewolves fires correctly in combat even when transformed. Not a problem for the Inquisitor.
- **`has_protection_from` using `card_data()` vs `back_face_data()` for the Inquisitor itself**: PASS — Elite Inquisitor is not a DFC; its `ProtectionFromSubtype` effects are in `card_data().continuous_effects` and are found correctly by `has_protection_from`.
- **Token attackers (e.g., a Vampire token) vs. protection**: PASS — `get_subtypes` in `combat.rs:356-369` checks both `obj.subtypes` (where token subtypes are stored) and `registry.card_data().subtypes`, so tokens with Vampire/Zombie/Werewolf subtypes are correctly identified when checking combat protection.
- **Mana cost {W}{W}, card types, subtypes, P/T, oracle text field**: PASS — all match Scryfall data exactly.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- First strike keyword present: `mtg-engine/tests/tier12_cards.rs:115` (`elite_inquisitor_keywords`)
- Vigilance keyword present: `mtg-engine/tests/tier12_cards.rs:115` (`elite_inquisitor_keywords`)
- Protection prevents Vampire combat damage: `mtg-engine/tests/tier12_cards.rs:128` (`elite_inquisitor_protection_prevents_damage`)
- Protection prevents Zombie blocking: `mtg-engine/tests/tier12_cards.rs:153` (`elite_inquisitor_cant_be_blocked_by_zombies`)
- Protection prevents Werewolf combat damage: NOT TESTED
- Protection prevents Werewolf blocking: NOT TESTED
- Protection prevents Vampire ability targeting (Olivia): NOT TESTED
- Protection prevents Werewolf ability targeting (Nightfall Predator): NOT TESTED
- Protection prevents Zombie triggered-ability targeting (Grimgrin): NOT TESTED
