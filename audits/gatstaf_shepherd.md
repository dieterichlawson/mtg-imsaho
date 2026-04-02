# Audit: Gatstaf Shepherd // Gatstaf Howler

## Oracle Reference
- **Name:** Gatstaf Shepherd
- **Mana Cost:** {1}{G}
- **Type:** Creature — Human Werewolf
- **P/T:** 2/2
- **Front Oracle Text:** At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
- **Back Name:** Gatstaf Howler
- **Back Type:** Creature — Werewolf
- **Back P/T:** 3/3
- **Back Oracle Text:** Intimidate / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

## Card Data Audit
- **Name:** Correct ("Gatstaf Shepherd")
- **Mana Cost:** Correct (Generic(1), Green)
- **Type:** Correct (Creature)
- **Subtypes:** Correct ("Human", "Werewolf")
- **P/T:** Correct (2/2)
- **Back Face Name:** Correct ("Gatstaf Howler")
- **Back Face Subtypes:** Correct ("Werewolf")
- **Back Face P/T:** Correct (3/3 via back_face_data and dynamic_pt)
- **Back Face Keywords:** Correct (Keyword::Intimidate)

## Behavior Audit
- **Transform (front to back):** `werewolf_should_transform` checks `total_spells_last_turn == 0 && !state.is_first_turn`. Correct.
- **Transform (back to front):** Checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)`. Correct -- "a player cast two or more spells."
- **Intimidate on back face:** Defined in `back_face_data()` keywords vec. The engine's `has_keyword` checks back_face_data when `is_transformed == true`. Correct.
- **on_upkeep:** Flips `is_transformed` and updates name. Correct.

## Result: PASS
