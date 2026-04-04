## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Return target creature to its owner's hand.
Flashback {4}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flashback exile after normal resolution: `move_spell_after_resolve` reads `cast_with_flashback` flag and routes to `Zone::Exile` — pass
- Flashback exile after being countered: `counterspell.rs` calls `move_spell_after_resolve(*target_id)` — pass
- Flashback exile after fizzle (target leaves battlefield): `stack.rs` fizzle path calls `move_spell_after_resolve(object_id)` — pass
- Sorcery timing restriction enforced on flashback cast: graveyard cast loop checks `is_sorcery_speed` for sorcery-type cards, same check as hand cast — pass
- Return to owner's hand (not controller's): `move_object` preserves `owner` field; `objects_in_zone(Zone::Hand, player)` filters by `obj.owner == player` — pass
- Can target any creature (own or opponent's): default `is_valid_target` returns `true`; `valid_targets_for_req` for `Creature` scans all battlefield creatures — pass
- Auras on bounced creature go to graveyard: SBA loop (rule 704.5m) runs after each action and sends unattached auras to graveyard — pass
- `cast_with_flashback` flag set correctly: `is_flashback = in_graveyard && !is_cast_from_graveyard`; set on the spell object before it moves to the stack — pass
- Can be cast via flashback even if put in graveyard without being cast: graveyard scan has no check on how the card arrived — pass
- Normal cast from hand goes to graveyard (not exile): `cast_with_flashback` defaults to `false`; `move_spell_after_resolve` routes to `Zone::Graveyard` — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flashback offered when card in graveyard: `flashback.rs:23` (flashback_offered_from_graveyard)
- Flashback not offered from hand: `flashback.rs:44` (flashback_not_offered_from_hand)
- Flashback not offered without mana: `flashback.rs:64` (flashback_not_offered_without_mana)
- Flashback spell exiled after resolution: `flashback.rs:86` (flashback_spell_is_exiled_after_resolve)
- Normal cast goes to graveyard: `flashback.rs:109` (normal_cast_goes_to_graveyard)
- Flashback spell countered is exiled: `flashback.rs:128` (flashback_spell_countered_is_exiled)
- Flashback fizzle goes to exile: `fizzle.rs:137` (flashback_spell_fizzle_goes_to_exile)
- Flashback fizzle does not emit SpellResolved: `fizzle.rs:176` (flashback_spell_fizzle_no_resolved_event)
- Silent Departure bounces creature to hand: `tier2_spells.rs:220` (silent_departure_bounces_creature)
- Sorcery timing restriction on flashback cast: NOT TESTED (generic sorcery/flashback timing not explicitly tested for Silent Departure; covered implicitly by is_sorcery_speed check)
- Return to owner's hand vs controller's hand: NOT TESTED (no test with a creature controlled by non-owner)
- Can target own creatures: NOT TESTED explicitly for Silent Departure
- Can be cast from graveyard without having been cast first: NOT TESTED explicitly for Silent Departure
