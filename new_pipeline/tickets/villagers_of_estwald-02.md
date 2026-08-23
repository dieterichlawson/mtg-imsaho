---
id: villagers_of_estwald-02
status: new
card: Villagers of Estwald
audit_run_id: 2026-04-19-villagers_of_estwald-audit
audit_model: sonnet
audit_tokens: 29158
audit_duration: 548
---

## Audit Finding

**Oracle text:**
> if no spells were cast last turn

**Code:**
> total_spells_last_turn == 0 && !state.is_first_turn

**Description:**
The werewolf_should_transform helper adds `&& !state.is_first_turn` as an additional guard that is not present in the oracle text. At the start of the game, num_spells_cast_last_turn is initialized to an empty HashMap; its values().sum() is therefore 0. On the very first upkeep of the game the oracle-text condition 'if no spells were cast last turn' is vacuously satisfied (zero spells were cast in the non-existent prior turn), but !state.is_first_turn evaluates to false and suppresses the transformation. The oracle text contains no such restriction. This guard is applied identically across every Innistrad front-face werewolf that uses this shared helper pattern.

**Engine path:** mtg-engine/src/cards/isd/villagers_of_estwald.rs:18

**Affected cards:**
- Daybreak Ranger
- Reckless Waif
- Tormented Pariah
- Gatstaf Shepherd
- Village Ironsmith
- Kruin Outlaw
- Mayor of Avabruck
- Grizzled Outcasts
- Ulvenwald Mystics
- Instigator Gang
- Hanweir Watchkeep

## Tests

### front_face_transforms_on_first_upkeep_if_no_prior_turn
Scenario: A Villagers of Estwald enters the battlefield before the very first upkeep of the game (e.g., via an effect that puts it directly onto the battlefield); at that upkeep the front-face condition 'if no spells were cast last turn' is met, so the creature should transform, but the !state.is_first_turn guard prevents it.

