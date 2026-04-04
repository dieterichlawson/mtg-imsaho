## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender, protection from Zombies
**Type line**: Creature — Plant
**Status**: ISSUE

### Code issues

- Protection from Zombies incorrectly prevents Grave Bramble from blocking Zombie attackers (`mtg-engine/src/combat.rs:699`)
  - Oracle text says: `protection from Zombies`
  - Code does: `can_block_attacker` contains the check `if has_protection_from_creature(state, blocker_id, attacker_id, registry) { return false; }` at line 699. When Grave Bramble is `blocker_id` and a Zombie is `attacker_id`, `has_protection_from_creature` returns true, so `can_block_attacker` returns false — preventing Grave Bramble from legally blocking Zombies. Per MTG rule 702.16d, "protection from Zombies" means the protected creature cannot be *blocked by* Zombies (when it is the attacker), not that the protected creature cannot *block* Zombies. The first check at line 696 (`has_protection_from_creature(state, attacker_id, blocker_id)`) already correctly handles the attacker-with-protection case. The second check (line 699) has no rules basis and is wrong. Since Grave Bramble also has Defender and can never attack, blocking is the only way its combat protection interacts with Zombies; the engine bug entirely eliminates that interaction.

- Grimgrin, Corpse-Born's triggered ability can target Grave Bramble despite protection from Zombies (`mtg-engine/src/cards/isd/grimgrin_corpse_born.rs:99–103`)
  - Oracle text says: `protection from Zombies`
  - Code does: `on_attacks` builds the target list as `state.objects_in_zone(Zone::Battlefield, defender).iter().filter(|o| o.power.is_some()).map(|o| Target::Object(o.id)).collect()` with no protection check. Grimgrin is a Zombie (subtype declared in `card_data()`), so its triggered ability "destroy target creature defending player controls" is a Zombie source. Per MTG rule 702.16b, a permanent with protection cannot be targeted by sources with the appropriate quality. Grave Bramble (protection from Zombies) should be excluded from this target list but is not.

### Tricky interactions checked

- Damage from Zombie combat (Walking Corpse blocks/attacks Grave Bramble): PASS — `deal_damage_to_creature` calls `has_protection_from_creature`, which correctly identifies Grave Bramble's `ProtectionFromSubtype { subtype: "Zombie", scope: OnSelf }` and prevents damage.
- Grave Bramble can block Zombies (protection is about being blocked, not blocking): FAIL — engine bug at `combat.rs:699` prevents Grave Bramble from being a valid blocker against Zombie attackers, the opposite of the correct behavior.
- Grimgrin (Zombie) triggered-ability targeting of Grave Bramble: FAIL — `on_attacks` in `grimgrin_corpse_born.rs` generates target list without protection check; Grave Bramble appears as a valid target.
- Zombie tokens (from e.g. Undead Alchemist) correctly identified as Zombies in `get_subtypes`: PASS — `get_subtypes` checks both `obj.subtypes` (object level, covers tokens) and `registry.card_data().subtypes`.
- Defender prevents attacking: PASS — `eligible_attackers` filters out `state.has_keyword(o.id, Keyword::Defender)`.
- Defender does not prevent blocking: PASS — `eligible_blockers` does not exclude creatures with Defender.
- Mana cost `{1}{G}{G}`: PASS — `Generic(1), Colored(Green), Colored(Green)`.
- P/T 3/4: PASS — `power: Some(3), toughness: Some(4)`.
- Subtype Plant (not Zombie, so no self-interaction with protection): PASS.
- `ProtectionFromSubtype` scope `OnSelf` resolves correctly via `effect_applies_to`: PASS — `EffectScope::OnSelf` checks `creature_id == source_id`; when the source object is Grave Bramble itself, the check passes.
- Protection keyword absent from `keywords` vec (implemented as `ContinuousEffect` instead): PASS — correct architecture; `Keyword` enum does not include Protection.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Zombie combat damage prevented: `mtg-engine/tests/card_mechanics.rs:309` (`grave_bramble_protection_prevents_zombie_damage`) — TESTED (uses `declare_blockers` not `declare_blockers_with_registry`, bypassing the block-validation bug)
- Grave Bramble can block Zombies (protection from Zombies does not restrict the protected creature from blocking): NOT TESTED — no test calls `can_block_attacker(state, grave_bramble, zombie, registry)` and asserts it returns true
- Grimgrin (Zombie) triggered ability cannot target Grave Bramble: NOT TESTED
- Defender prevents attacking: `mtg-engine/tests/keywords.rs:107` (`defender_cannot_attack`) — TESTED
- Defender does not prevent blocking: `mtg-engine/tests/keywords.rs:119` (`defender_can_block`) — TESTED
- Zombie token correctly treated as Zombie by protection check: NOT TESTED for Grave Bramble specifically
