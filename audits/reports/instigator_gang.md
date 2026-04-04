# Audit: Instigator Gang // Wildblood Pack

## Oracle (Official)
### Front: Instigator Gang
- **Cost:** {3}{R}
- **Type:** Creature — Human Werewolf
- **Oracle:** Attacking creatures you control get +1/+0. At the beginning of each upkeep, if no spells were cast last turn, transform Instigator Gang.
- **P/T:** 2/3

### Back: Wildblood Pack
- **Type:** Creature — Werewolf
- **Oracle:** Trample. Attacking creatures you control get +3/+0. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.
- **P/T:** 5/5

## Implementation
- Front name: "Instigator Gang" -- CORRECT
- Front cost: {3}{R} -- CORRECT
- Front subtypes: ["Human", "Werewolf"] -- CORRECT
- Front P/T: 2/3 -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Wildblood Pack" -- CORRECT
- Back subtypes: ["Werewolf"] -- CORRECT
- Back P/T: 5/5 (via dynamic_pt) -- CORRECT
- Back keywords: [Trample] -- CORRECT
- Back oracle text matches -- CORRECT
- Transform logic: no spells -> transform to back; any player 2+ spells -> transform to front -- CORRECT
- Attacking creatures buff: +1/+0 (front) or +3/+0 (back) via on_any_creature_attacks -- CORRECT
- Uses UntilEndOfTurnEffect for power bonus -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit — 2026-04-01 15:12

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform Instigator Gang.
**Oracle text (back)**: Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.
**Type line (front)**: Creature — Human Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

### Code issues

1. **Instigator Gang / Wildblood Pack does not buff itself when attacking**
   - Oracle text says: `Attacking creatures you control get +1/+0.` (front) / `Attacking creatures you control get +3/+0.` (back) — no "other" qualifier, so the buff applies to ALL attacking creatures you control, including Instigator Gang/Wildblood Pack itself.
   - Code does: The `AnyCreatureAttacks` watcher in `mtg-engine/src/triggers.rs:708` filters with `o.id != *attacker_id`, which means when Instigator Gang itself attacks, it is excluded from the watchers list and `on_any_creature_attacks` is never called with itself as both watcher and attacker. As a result, Instigator Gang never gives itself the +1/+0 (or +3/+0) bonus when it attacks.
   - Note: This is an engine-level limitation in how `AnyCreatureAttacks` watchers are dispatched. The fix would need to either include self in the watcher list or add a separate `on_attacks` hook for the self-buff case.

### Tricky interactions checked
- Werewolf transform timing (upkeep, no spells / 2+ spells): PASS
- First turn protection (no transform on first turn): PASS
- Buff only applies to creatures the controller owns: PASS
- Trample on back face only: PASS
- Back face missing Upkeep in triggered_abilities but trigger still fires because front face has it: PASS (works due to `trigger_description` checking front face first)

### Test coverage
- Transform test: `werewolf_cards.rs:instigator_gang_transforms_and_gains_trample` — tests transform and trample
- Buff on attack (other creatures): NOT TESTED
- Buff on attack (self): NOT TESTED (this is the bug — would fail)
- Transform back when 2+ spells cast: NOT DIRECTLY TESTED (covered by generic werewolf tests)
- First turn no-transform: NOT DIRECTLY TESTED (covered by generic werewolf tests)

## Audit — 2026-04-01 21:42

**Oracle text source**: Oracle cache (Scryfall API) via `python3 scripts/oracle_lookup.py lookup "Instigator Gang"`
**Oracle text (front)**: Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Werewolf
**Type line (back)**: Creature — Werewolf
**Mana cost**: {3}{R}
**P/T (front)**: 2/3
**P/T (back)**: 5/5
**Rulings**: 1 ruling (2016-07-13) — general DFC reference, no mechanical implications.
**Status**: PASS

### Code issues
No issues found.

Note on previous audit (2026-04-01 15:12): That audit incorrectly claimed Instigator Gang does not buff itself when attacking. This is false. The `AnyCreatureAttacks` watcher loop at `mtg-engine/src/triggers.rs:709-712` iterates ALL battlefield permanents with no exclusion of the attacker — the comment at line 707-708 explicitly states "the attacker is NOT excluded." The test `instigator_gang_buffs_itself_when_attacking` in `mtg-engine/tests/werewolf_cards.rs:405` confirms the self-buff works correctly (2 base + 1 buff = 3 effective power).

### Card data verification
- Name: "Instigator Gang" — matches oracle. CORRECT.
- Mana cost: `Generic(3), Colored(Red)` — matches `{3}{R}`. CORRECT.
- Card types: `[Creature]` — matches oracle. CORRECT.
- Supertypes: `[]` — none needed. CORRECT.
- Subtypes (front): `["Human", "Werewolf"]` — matches "Creature — Human Werewolf". CORRECT.
- Subtypes (back): `["Werewolf"]` — matches "Creature — Werewolf". CORRECT.
- P/T (front): `power: Some(2), toughness: Some(3)` — matches 2/3. CORRECT.
- P/T (back): `dynamic_pt` returns `(5, 5)` when transformed; `back_face_data` also declares `power: Some(5), toughness: Some(5)`. CORRECT.
- Keywords (front): `vec![]` — front face has no game keywords (Transform is a DFC mechanic, not a keyword ability). CORRECT.
- Keywords (back): `vec![Keyword::Trample]` — matches "Trample". CORRECT.
- Oracle text fields: Both faces match verbatim oracle text. CORRECT.

### Behavior verification
- **Attacking creatures buff (front)**: `on_any_creature_attacks` applies `power_mod: 1` via `UntilEndOfTurnEffect` when `!is_transformed`. CORRECT.
- **Attacking creatures buff (back)**: Same function applies `power_mod: 3` when `is_transformed`. CORRECT.
- **Controller restriction**: Code checks `attacker_controller != controller` and returns early, preventing buffs to opponent's attackers. CORRECT.
- **Self-buff**: The watcher loop does NOT exclude the attacker, so Instigator Gang/Wildblood Pack buffs itself when it attacks. CORRECT.
- **Transform (front to back)**: `werewolf_should_transform` checks `total_spells_last_turn == 0 && !state.is_first_turn`. Oracle says "if no spells were cast last turn." CORRECT.
- **Transform (back to front)**: Checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)`. Oracle says "if a player cast two or more spells last turn." CORRECT.
- **First turn protection**: Front face won't transform on first turn (`!state.is_first_turn`). CORRECT per standard werewolf rules.
- **Transform applies name/keywords/subtypes**: Uses `helpers::apply_transform` which correctly updates `name`, `keywords`, and `subtypes` from `back_face_data`. CORRECT.

### Tricky interactions checked
- Werewolf transform timing (upkeep, spell count check): PASS
- First turn protection: PASS
- Self-buff on attack: PASS
- Buff restricted to controller's creatures only: PASS
- Trample on back face only (not front): PASS
- Back face missing `TriggerKind::Upkeep` in `triggered_abilities` — works because `trigger_description` finds the Upkeep entry on the front face first: PASS
- Static ability implemented as triggered `AnyCreatureAttacks` — functionally equivalent for all cards in the pool (no cards create creatures already attacking): PASS (architectural note, not a bug)

### Minor UI note (not flagged as issue)
When transformed to Wildblood Pack, the trigger description on the stack reads "attacking creatures you control get +1/+0" instead of "+3/+0" because `trigger_description` matches the front face's `AnyCreatureAttacks` entry first. The actual buff applied is correct (+3). This is cosmetic only.

### LLM card knowledge
Instigator Gang / Wildblood Pack is NOT in the LLM card knowledge section (`mtg-player/src/llm.rs`). This means AI players have no strategic guidance for this card.

### Test coverage
- Transform and gains trample: `werewolf_cards.rs:instigator_gang_transforms_and_gains_trample` (line 389)
- Self-buff when attacking (+1/+0): `werewolf_cards.rs:instigator_gang_buffs_itself_when_attacking` (line 405)
- Buffs other attackers you control: `werewolf_cards.rs:instigator_gang_buffs_other_attackers_you_control` (line 425)
- Does not buff opponent's attackers: `werewolf_cards.rs:instigator_gang_does_not_buff_opponent_attackers` (line 446)
- Wildblood Pack self-buff (+3/+0): `werewolf_cards.rs:wildblood_pack_buffs_itself_plus_3` (line 464)
- Transform back when 2+ spells: NOT DIRECTLY TESTED (covered by generic werewolf tests for the same transform logic)
- Wildblood Pack buffs other attackers +3/+0: NOT TESTED (only self-buff tested for back face)
- First turn no-transform: NOT DIRECTLY TESTED (covered by generic werewolf tests)

## Audit: Instigator Gang // Wildblood Pack
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Front -- Type:** Creature -- Human Werewolf | **Cost:** {3}{R} | **P/T:** 2/3
- **Front -- Oracle:** Attacking creatures you control get +1/+0. / At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **Back -- Type:** Creature -- Werewolf | **P/T:** 5/5
- **Back -- Oracle:** Trample / Attacking creatures you control get +3/+0. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

### Card Data
- **Name:** Instigator Gang -- PASS
- **Cost:** {3}{R} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Human, Werewolf -- PASS
- **P/T:** 2/3 (front), 5/5 (back via dynamic_pt) -- PASS
- **Back face name:** Wildblood Pack -- PASS
- **Back face subtypes:** Werewolf -- PASS
- **Back face keywords:** Trample -- PASS

### Oracle Text Match
- Front and back oracle text match. -- PASS

### Behavior Audit
- **Attacking buff (front):** +1/+0 to attacking creatures you control via on_any_creature_attacks. -- PASS
- **Attacking buff (back):** +3/+0 to attacking creatures you control. -- PASS
- **Transform (front->back):** Checks no spells cast last turn and not first turn. -- PASS
- **Transform (back->front):** Checks if any player cast 2+ spells last turn. -- PASS
- **Static vs triggered implementation:** Oracle describes a static continuous effect ("Attacking creatures you control get +1/+0") but code implements it as a triggered effect via on_any_creature_attacks with UntilEndOfTurnEffect. This is a reasonable approximation but technically differs from a true static ability (e.g., a creature that gains control mid-combat would not get the buff in code but would under oracle rules). Minor behavioral difference.
- **Back face triggered_abilities:** Missing Upkeep TriggerKind in back_face_data's triggered_abilities vec. The on_upkeep handler still runs because it dispatches from the front face implementation, so this is a metadata omission rather than a functional bug.

### Result: PASS

## Audit — 2026-04-03 07:04
**Oracle text source**: Scryfall API (via `python3 scripts/oracle_lookup.py lookup "Instigator Gang"`)
**Oracle text**:
Front (Instigator Gang): Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
Back (Wildblood Pack): Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**:
Front: Creature — Human Werewolf
Back: Creature — Werewolf
**Status**: PASS

### Code issues
None found. All card data and behavior match oracle text.

- Name: "Instigator Gang" / "Wildblood Pack" — matches oracle.
- Cost: Generic(3) + Red = {3}{R} — matches oracle.
- Types/Subtypes: Front ["Human", "Werewolf"], Back ["Werewolf"] — match oracle.
- P/T: Front 2/3, Back 5/5 (via dynamic_pt) — match oracle.
- Keywords: Front none, Back [Trample] — correct (Trample is back-face only).
- Buff: +1/+0 (front) / +3/+0 (back) applied to attacking creatures you control including self — matches oracle (no "other" qualifier).
- Controller check: attacker_controller != controller early return prevents buffing opponent's creatures — correct.
- Transform front->back: no spells cast last turn AND not first turn — correct per werewolf rules.
- Transform back->front: any player cast 2+ spells last turn — correct.
- Cosmetic note: when transformed, the trigger stack description says "+1/+0" instead of "+3/+0" because trigger_description finds the front face's AnyCreatureAttacks entry first. The actual buff applied is correct (+3/+0). Not a functional issue.

### Tricky interactions checked (min 3)
1. Self-buff on attack: Instigator Gang buffs itself when it attacks (+1/+0 front, +3/+0 back). The AnyCreatureAttacks watcher loop does NOT exclude the attacker (confirmed by comment at triggers.rs:724-725 and test instigator_gang_buffs_itself_when_attacking). PASS.
2. Buff restricted to controller's creatures: Code checks attacker_controller != controller and returns early. Test instigator_gang_does_not_buff_opponent_attackers confirms this. PASS.
3. Trample only on back face: Front face has keywords: vec![], back face has keywords: vec![Keyword::Trample]. Test instigator_gang_transforms_and_gains_trample confirms trample appears only after transform. PASS.
4. Back face missing Upkeep TriggerKind in triggered_abilities: The back_face_data only declares AnyCreatureAttacks, not Upkeep. However, trigger_description checks front face triggers first, finds the Upkeep entry, and returns it — so the upkeep trigger fires when transformed. on_upkeep then correctly handles transform-back logic. PASS.
5. Static ability implemented as triggered AnyCreatureAttacks + UntilEndOfTurnEffect: This is the standard engine pattern for "attacking creatures get +N/+0" effects. No cards in the Innistrad pool create creatures already attacking, so the approximation is functionally equivalent. PASS (engine limitation, not card bug).

### Test coverage
- instigator_gang_transforms_and_gains_trample (line 389): transform + trample keyword check
- instigator_gang_buffs_itself_when_attacking (line 405): self-buff +1/+0
- instigator_gang_buffs_other_attackers_you_control (line 425): ally gets +1/+0
- instigator_gang_does_not_buff_opponent_attackers (line 446): opponent's creature not buffed
- wildblood_pack_buffs_itself_plus_3 (line 464): back face self-buff +3/+0
- NOT TESTED: Wildblood Pack buffs other attackers +3/+0 (only self-buff tested for back face)
- NOT TESTED: Transform back when 2+ spells (covered by generic werewolf tests using same logic)
