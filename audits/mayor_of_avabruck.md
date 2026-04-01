# Audit: Mayor of Avabruck // Howlpack Alpha

## Official Oracle

### Front Face: Mayor of Avabruck
- **Cost:** {1}{G}
- **Type:** Creature — Human Advisor Werewolf
- **Oracle:** Other Human creatures you control get +1/+1. At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
- **P/T:** 1/1

### Back Face: Howlpack Alpha
- **Type:** Creature — Werewolf
- **Oracle:** Each other creature you control that's a Werewolf or a Wolf gets +1/+1. At the beginning of your end step, create a 2/2 green Wolf creature token. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
- **P/T:** 3/3

## Implementation: `mtg-engine/src/cards/mayor_of_avabruck.rs`

### Front Face
- **Name:** Mayor of Avabruck -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Subtypes:** Human, Advisor, Werewolf -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Continuous effect:** ModifyPT +1/+1, GlobalOther(You AND Human) -- CORRECT
- **Triggered ability:** Upkeep transform -- CORRECT

### Back Face
- **Name:** Howlpack Alpha -- CORRECT
- **Subtypes:** Werewolf -- CORRECT
- **P/T:** 3/3 (via dynamic_pt) -- CORRECT
- **Continuous effect:** ModifyPT +1/+1, GlobalOther(You AND (Werewolf OR Wolf)) -- CORRECT
- **Triggered ability:** EndStep create 2/2 Wolf token -- CORRECT
- **Wolf token:** 2/2 green creature with "Wolf" subtype -- CORRECT

### Transform Logic
- Front->Back: No spells cast last turn AND not first turn -- CORRECT
- Back->Front: Any player cast 2+ spells last turn -- CORRECT

## Issues
1. **Back face missing Upkeep triggered ability:** The back face `triggered_abilities` only includes `TriggerKind::EndStep` but is missing `TriggerKind::Upkeep` for the transform-back trigger. The `on_upkeep` handler does handle both directions, but the triggered_abilities metadata doesn't list it for the back face.

## Verdict
**FAIL** -- 1 issue: Back face metadata missing Upkeep triggered ability entry.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: Other Human creatures you control get +1/+1. / At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Scryfall Oracle text (back)**: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. / At the beginning of your end step, create a 2/2 green Wolf creature token. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Scryfall type line (front)**: Creature — Human Advisor Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: mana cost {1}{G} correct, subtypes Human/Advisor/Werewolf correct, P/T 1/1 correct, ModifyPT +1/+1 for other Humans you control correct, Upkeep triggered ability declared correctly.

Back face: P/T 3/3 correct (via dynamic_pt), subtypes ["Werewolf"] correct. ModifyPT +1/+1 scope GlobalOther(You AND (Werewolf OR Wolf)) correct. EndStep Wolf token creation: 2/2 green Wolf creature token with "Wolf" subtype correct. Only triggers during controller's end step when transformed: correct.

Issues found:
1. **Back face missing Upkeep triggered_abilities declaration** (persists from prior audit): The back face `triggered_abilities` includes `TriggerKind::EndStep` for Wolf token creation but is missing `TriggerKind::Upkeep` for the transform-back trigger. The `on_upkeep` handler correctly handles both directions of transform, but the metadata in `back_face_data` is incomplete. This could cause issues if the engine uses `triggered_abilities` to determine whether to call `on_upkeep` for a card.

Tests present in `tests/werewolf_cards.rs`. No anti-patterns found.
