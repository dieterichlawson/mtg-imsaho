# Audit: Moonmist

## Official Oracle
- **Name:** Moonmist
- **Cost:** {1}{G}
- **Type:** Instant
- **Oracle:** Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.

## Implementation: `mtg-engine/src/cards/moonmist.rs`
- **Name:** Moonmist -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Instant -- CORRECT
- **on_resolve:** Transforms Human DFCs, updates characteristics from back face -- CORRECT

## Issues
1. **Combat damage prevention not implemented:** The oracle says "Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves." This is noted in comments as not implemented. This is a significant part of the card's effect.

## Verdict
**FAIL** -- 1 issue: Combat damage prevention for non-Wolf/non-Werewolf creatures is not implemented.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Scryfall type line**: Instant
**Status**: PASS

Previous combat damage prevention issue has been fixed. The implementation now sets `state.prevent_non_wolf_werewolf_combat_damage = true` on resolve, which flags the engine to prevent combat damage from non-Wolf/non-Werewolf creatures this turn.

Verified correct:
- Mana cost: {1}{G} -- matches
- Type: Instant -- matches
- Transform logic: transforms all Humans that have a back face (DFCs), updates name/P/T/keywords/subtypes from back face -- correct per reminder text "(Only double-faced cards can be transformed.)"
- Combat damage prevention: sets engine flag for non-Wolf/non-Werewolf prevention -- correct
- `on_resolve` calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/moonmist.rs` and `mtg-engine/tests/innistrad_simple_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Type line**: Instant
**Status**: ISSUE

Card data correct: name, mana cost ({1}{G}), type (Instant).

Transform logic: transforms all Humans with a back face (DFCs only), updates name/P/T/keywords/subtypes from back face. Correct per rulings ("Moonmist causes any double-faced Human to transform, not just Werewolves") and reminder text.

Combat damage prevention: sets state.prevent_non_wolf_werewolf_combat_damage = true. Correct.

on_resolve calls move_spell_after_resolve(object_id). Correct.

Minor issue:
1. The code filters on `!o.is_transformed` which means it only transforms front-face Humans to their back face. The oracle says "Transform all Humans" which should also transform any currently-transformed creature whose back face has the Human subtype back to its front face. In practice this is unlikely to matter in Innistrad (Humans are typically front-face), but it is technically incomplete.

Tests in moonmist.rs cover prevention flag, damage prevention to player/creature, and wolf exception. No test for the transform functionality itself, but the damage prevention tests are thorough.

## Audit — 2026-04-01 14:37

**Oracle text source**: Scryfall via WebSearch (https://scryfall.com/card/isd/195/moonmist)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Type line**: Instant
**Status**: ISSUE

Card data verified correct: name, mana cost ({1}{G}), card_types (Instant), oracle_text matches.

on_resolve correctly:
- Transforms Human DFCs by checking for Human subtype and back face existence
- Updates characteristics (name, P/T, keywords, subtypes) from back face data
- Sets `state.prevent_non_wolf_werewolf_combat_damage = true`
- Calls `move_spell_after_resolve(object_id)` (correct for instant)

Issue:

1. **Transform only applies to non-transformed Humans** (`moonmist.rs` line 34).
   - Oracle text says: `Transform all Humans.`
   - Code does: `.filter(|o| o.zone == Zone::Battlefield && !o.is_transformed)` -- the `!o.is_transformed` filter means only front-face (non-transformed) Humans are transformed. If a DFC's back face has the Human subtype and is currently transformed (showing the back face), it would not be transformed back to the front face. The oracle says "Transform all Humans" which means any creature currently with the Human subtype should be transformed, regardless of which face is showing.

No other issues found. Tests in moonmist.rs (4 tests) cover prevention flag, damage prevention to player, wolf exception, and damage prevention to creature. No test for the transform functionality itself.

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

Card data verified correct: name "Moonmist", mana cost {1}{G} (Generic(1), Green), type Instant, oracle text matches.

Transform logic (lines 33-70): correctly identifies creatures that are (a) on the battlefield, (b) not already transformed, (c) have the Human subtype, and (d) have a back face (are DFCs). Updates name, P/T, keywords, and subtypes from the back face. The `!o.is_transformed` filter is correct because a creature that is already transformed would have its back-face subtypes (not Human), so the Human subtype check alone would filter it out. The extra `!o.is_transformed` check is redundant but harmless.

Per ruling: "Moonmist causes any double-faced Human to transform, not just Werewolves." The code checks `has_human_subtype && has_back_face` without restricting to Werewolves -- correct.

Per ruling: "Whether or not a creature is a Werewolf or a Wolf is checked only as combat damage is dealt." The code uses `state.prevent_non_wolf_werewolf_combat_damage = true` flag checked at damage time via `is_non_wolf_damage_prevented` in `combat.rs` (line 301-307), which checks subtypes at the time of damage dealing -- correct.

Per ruling: "Moonmist will prevent combat damage dealt by a creature that isn't a Werewolf or a Wolf even if that creature wasn't on the battlefield... when Moonmist resolved." The flag applies to all creatures at damage time, not just those present at resolution -- correct.

Uses `move_spell_after_resolve(object_id)` -- correct. Flag cleared at end of turn (engine.rs line 2259) -- correct.

### Tricky interactions checked
- Non-Wolf/Werewolf DFC Human transform: pass
- Combat damage prevention checked at damage time, not resolution time: pass
- Flag cleared at end of turn: pass
- Only DFCs transform (reminder text): pass

### Test coverage
- Prevention flag set after resolve: `tests/moonmist.rs` (line 19)
- Non-wolf combat damage to player prevented: `tests/moonmist.rs` (line 32)
- Wolf still deals combat damage: `tests/moonmist.rs` (line 50)
- Non-wolf combat damage to creature prevented: `tests/moonmist.rs` (line 68)
- Card data verification: `tests/innistrad_simple_cards.rs` (line 530)
- Transform functionality (actual Human DFC transforms): NOT TESTED
- Werewolf creature deals combat damage after Moonmist: NOT TESTED (only Wolf tested)

## Audit — 2026-04-01 (independent)

**Oracle text source**: Scryfall API (cached via oracle_lookup.py)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Type line**: Instant
**Mana cost**: {1}{G}
**Rulings**:
- Moonmist causes any double-faced Human to transform, not just Werewolves.
- Whether or not a creature is a Werewolf or a Wolf is checked only as combat damage is dealt.
- Moonmist will prevent combat damage dealt by a creature that isn't a Werewolf or a Wolf even if that creature wasn't on the battlefield (or was a Werewolf or a Wolf) when Moonmist resolved.
**Status**: ISSUE

### Code issues

1. **Transform filter incorrectly excludes already-transformed Humans** (`mtg-engine/src/cards/isd/moonmist.rs` line 34)
   - Oracle text says: `Transform all Humans.`
   - Code does: `.filter(|o| o.zone == Zone::Battlefield && !o.is_transformed)` — the `!o.is_transformed` guard skips any creature already showing its back face. This is incorrect when a DFC's back face also has the Human subtype. Concrete example: Thraben Sentry's back face Thraben Militia is a Human Soldier (subtypes: `["Human", "Soldier"]` in `thraben_sentry.rs` line 43). A transformed Thraben Militia on the battlefield is a Human but would not be transformed back to Thraben Sentry by Moonmist because the `!o.is_transformed` filter excludes it. The fix should remove `!o.is_transformed` from the filter and add bidirectional transform logic: if the creature is not transformed, transform to back face; if already transformed, transform to front face.
   - NOTE: A previous audit (18:00) incorrectly marked this PASS, reasoning that "a creature that is already transformed would have its back-face subtypes (not Human)". This is factually wrong for Thraben Militia, which IS a Human on its back face.

2. **Not in LLM card knowledge** (`mtg-player/src/llm.rs`)
   - AI players have no awareness of Moonmist. For a card that functions as both a one-sided combat trick (preventing opponent's non-Wolf damage) and a mass transform enabler, this is a notable gap.

### Tricky interactions checked
- Non-Werewolf Human DFCs (e.g., Cloistered Youth) transform correctly when on front face: pass
- Already-transformed back-face Humans (e.g., Thraben Militia) should transform back: FAIL (issue #1)
- Combat damage prevention checked at damage-dealing time (not resolution): pass (per ruling #2)
- Prevention applies to creatures entering after Moonmist resolves: pass (per ruling #3, flag is global)
- Flag cleared at end of turn (`engine.rs` line 2464): pass
- Only DFCs can transform (non-DFC Humans are skipped via `has_back_face` check): pass
- Wolf AND Werewolf creatures still deal combat damage (`combat.rs` line 306 checks both subtypes): pass
- `move_spell_after_resolve(object_id)` used correctly: pass

### Test coverage
- Prevention flag set after resolve: `tests/moonmist.rs:19`
- Non-Wolf combat damage to player prevented: `tests/moonmist.rs:32`
- Wolf still deals combat damage: `tests/moonmist.rs:50`
- Non-Wolf combat damage to creature prevented: `tests/moonmist.rs:68`
- Card data verification: `tests/innistrad_simple_cards.rs:530`
- Transform of front-face Human DFC: NOT TESTED
- Transform of back-face Human DFC (e.g., Thraben Militia): NOT TESTED
- Werewolf creature still deals combat damage: NOT TESTED (only Wolf tested)
- Non-DFC Human not affected: NOT TESTED

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

Card data is correct: {1}{G} Instant, no keywords beyond Transform (not stored as keyword), oracle text matches. The on_resolve correctly: (1) finds all DFC Humans on the battlefield and transforms them, checking both front and back face subtypes, (2) sets the state-level prevent_non_wolf_werewolf_combat_damage flag. The combat damage prevention is enforced in combat.rs via is_non_wolf_damage_prevented which checks subtypes at damage-deal time (correct per ruling). The flag is cleared at end of turn (engine.rs:2543). Uses move_spell_after_resolve correctly.

### Tricky interactions checked
- Only DFC Humans transform (non-DFC Humans unaffected): PASS - code checks has_back_face at line 58-61
- Back-face Humans also transform (e.g., Thraben Militia): PASS - code checks is_transformed and back_face_data at line 46-49
- Wolf/Werewolf type checked at damage time, not Moonmist resolution: PASS - is_non_wolf_damage_prevented called in deal_damage_to_creature and deal_damage_to_player
- Creatures entering after Moonmist still prevented: PASS - flag is state-level, applies to all creatures
- Prevention clears at end of turn: PASS - engine.rs:2543
- move_spell_after_resolve used: PASS

### Test coverage
- Prevention flag set: `moonmist.rs:20` sets_prevention_flag
- Non-Wolf combat damage to player prevented: `moonmist.rs:33` prevents_non_wolf_combat_damage_to_player
- Wolf still deals damage: `moonmist.rs:51` wolf_still_deals_damage
- Non-Wolf combat damage to creature prevented: `moonmist.rs:69` prevents_non_wolf_combat_damage_to_creature
- Front-face Human transforms: `moonmist.rs:91` transforms_front_face_human
- Back-face Human transforms: `moonmist.rs:110` transforms_back_face_human
- Non-DFC Human not transformed: `moonmist.rs:135` does_not_transform_non_dfc_human
- Card data: `innistrad_simple_cards.rs:530` moonmist_card_data
- Werewolf creature still deals combat damage: NOT TESTED (only Wolf tested)
- Ruling: creature entering after Moonmist still prevented: NOT TESTED
