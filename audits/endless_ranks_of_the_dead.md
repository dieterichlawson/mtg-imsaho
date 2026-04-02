# Audit: Endless Ranks of the Dead

## Reference (Scryfall)
- **Name:** Endless Ranks of the Dead
- **Cost:** {2}{B}{B}
- **Type:** Enchantment
- **Oracle:** At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{B}{B})
- Type: CORRECT (Enchantment)
- Oracle text: CORRECT
- Triggered ability: CORRECT (TriggerKind::Upkeep)
- Only triggers on controller's upkeep: CORRECT (checks state.active_player == controller)
- Counts Zombies you control: CORRECT
- X = half rounded down: CORRECT (zombie_count / 2)
- Creates 2/2 black Zombie tokens: CORRECT
- Token subtypes: CORRECT (Zombie)
- P/T: CORRECT (N/A)

## Issues
None found.

---

# Re-Audit: Endless Ranks of the Dead (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)
> At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.

- **Mana Cost:** {2}{B}{B}
- **Type:** Enchantment

## Official Rulings
1. If you control fewer than two Zombies, you won't get any tokens. (2011-09-22)
2. The number of Zombies you control is counted when the ability resolves. (2011-09-22)
3. If you control multiple Endless Ranks of the Dead, the tokens you get when the first ability resolves will count for the subsequent abilities. (2011-09-22)

## Implementation File
`mtg-engine/src/cards/isd/endless_ranks_of_the_dead.rs`

## Audit Checklist

### Card Data
- [x] **Name:** "Endless Ranks of the Dead" -- correct.
- [x] **Mana cost:** `Generic(2), Black, Black` -- matches {2}{B}{B}.
- [x] **Card types:** `[Enchantment]` -- correct.
- [x] **Supertypes / subtypes:** empty -- correct (not legendary, no subtypes).
- [x] **Oracle text:** matches Scryfall verbatim.
- [x] **Triggered abilities:** declares `TriggerKind::Upkeep` -- correct.

### Trigger Logic (`on_upkeep`)
- [x] **Zone check:** only fires if the enchantment is on the battlefield (`o.zone == Zone::Battlefield`).
- [x] **Active player check:** only triggers on controller's upkeep (`state.active_player != controller`).
- [x] **Zombie count at resolution:** counts Zombies at the time `on_upkeep` is called, which is resolution time. Correct per ruling #2.
- [x] **Counts from registry subtypes AND object subtypes:** checks both `registry.card_data(o.card_id)` subtypes and `o.subtypes` on the object itself. This correctly catches both card-defined Zombies and token Zombies whose subtype is set on the object.
- [x] **Creature filter:** uses `o.power.is_some()` as a proxy for "is a creature." Adequate for the current card pool.
- [x] **Integer division:** `zombie_count / 2` -- Rust integer division truncates toward zero, equivalent to rounding down for positive numbers. Correct.
- [x] **Token creation:** creates tokens via `create_token_with_subtypes` with name "Zombie", P/T 2/2, color Black, type Creature, subtypes ["Zombie"]. Correct.
- [x] **No keywords on tokens:** tokens have no keywords (`vec![]`). Correct -- Zombie tokens are vanilla 2/2s.

### Edge Cases
- [x] **0 or 1 Zombies:** `0 / 2 = 0` and `1 / 2 = 0` -- no tokens created. Matches ruling #1.
- [x] **Multiple copies:** each Endless Ranks triggers independently; Zombies created by the first resolution are counted by subsequent resolutions. Matches ruling #3.
- [x] **Parallel Lives interaction:** `create_token_with_subtypes` handles Parallel Lives doubling internally. Correct.

### Test Coverage
- File: `mtg-engine/tests/tier7_cards.rs`, test `endless_ranks_creates_zombie_tokens`
- Tests 5 Zombies -> 2 tokens (5/2 = 2 rounded down), verifying final count of 7. Correct.
- Missing test: 0 or 1 Zombie edge cases.
- Missing test: opponent's upkeep should not trigger.

## Potential Issues

### Minor
1. **Creature detection heuristic:** The filter `o.power.is_some()` is used as a proxy for "is a creature" when counting Zombies. A non-creature permanent with power/toughness could be miscounted, but no such case exists in the Innistrad set.

## Verdict

**PASS** -- The implementation correctly matches the oracle text. Card data, trigger kind, Zombie counting logic, integer division rounding, and token creation are all faithful to the rules. No mismatches found.
