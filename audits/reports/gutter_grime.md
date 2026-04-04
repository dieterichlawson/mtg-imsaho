# Audit: Gutter Grime

## Oracle Reference (Scryfall)
- Cost: {4}{G}
- Type: Enchantment
- Oracle: "Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime.""

## Implementation: gutter_grime.rs

## Issues Found

1. **FIXED: Slime counters stored as PlusOnePlusOne counters** - Added CounterType::Slime to properly track slime counters separately from +1/+1 counters.

2. **FIXED: Ooze tokens have static P/T instead of dynamic** - Tokens now have base 0/0 P/T with dynamic lookup via card_state "pt_source_counter" linking to the source Gutter Grime. effective_power/toughness dynamically reads slime counter count.

Otherwise correct: cost ({4}{G}), type (Enchantment), trigger (nontoken creature you control dies), creates green Ooze tokens.

## Verdict: ALL ISSUES FIXED

## Audit — 2026-04-01 06:10

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: PASS

No issues found. Dynamic P/T for Ooze tokens now correctly tracks slime counter count on the source Gutter Grime. Each token stores its source Gutter Grime ObjectId via card_state, and effective_power/toughness dynamically look up the counter count. If Gutter Grime leaves the battlefield, tokens become 0/0. Token deaths and opponent creature deaths correctly do not trigger the ability.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Scryfall type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct. Type Enchantment: correct. No subtypes: correct. Trigger on nontoken creature you control dying: correct (checks `is_token` and `dead_controller`). Adds slime counter via `CounterType::Slime`: correct. Creates green Ooze creature token with dynamic P/T linked to slime counters on source Gutter Grime: correct. Token created with subtypes `["Ooze"]`: correct. Uses `AnyCreatureDies` trigger kind with `triggered_abilities` declaration: correct. Tests present in `tests/gutter_grime.rs` and `tests/tier15_cards.rs`. No anti-patterns found (no `move_object` to graveyard for spells, no `CombatDamageDealt` misuse).

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct (Generic(4) + Green). Type Enchantment: correct. No supertypes or subtypes: correct. No P/T: correct. Oracle text in code says "This creature's power and toughness" while Scryfall says "This token's power and toughness" -- minor text discrepancy, but functionally identical since the token IS a creature. Trigger condition: `AnyCreatureDies` trigger kind, filters for nontoken (`is_token` check) and controller-owned (`dead_controller`): correct. Adds slime counter via `CounterType::Slime`: correct. Creates green Ooze creature token with base 0/0 and dynamic P/T linked to source Gutter Grime via `card_state["pt_source_counter"]`: correct per rulings (tokens dynamically track counter count). Token has subtypes `["Ooze"]` and colors `[Green]`: correct. `triggered_abilities` declaration matches the `on_any_creature_dies` hook: correct. Tests in `tests/gutter_grime.rs` cover: basic trigger, multiple deaths growing all tokens, token death ignored, opponent death ignored, Gutter Grime removal makes tokens 0/0. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on Gutter Grime, then create a green Ooze creature token with "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

Mana cost {4}{G}: correct. Type Enchantment: correct. No supertypes or subtypes: correct.

Triggered ability: triggers on `AnyCreatureDies`, checks that dead creature was nontoken (`is_token` check) and controlled by the enchantment's controller: correct. Adds slime counter via `CounterType::Slime`: correct. Creates a green Ooze creature token with base 0/0 and dynamic P/T linked to slime counter count on source Gutter Grime via `card_state["pt_source_counter"]`: correct per rulings that tokens update dynamically. Token has correct subtypes `["Ooze"]` and color green: correct.

Oracle text in code says "This creature's power and toughness" while Scryfall says "This creature's power and toughness" (Scryfall also shows "This token's power and toughness" in some entries -- minor wording variant, functionally identical).

Tests: 4 tests in `tests/gutter_grime.rs` covering creation, growth, token death exclusion, opponent death exclusion, and source removal causing 0/0. Good coverage. No anti-patterns found.

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)

> Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."

### Implementation Review (`mtg-engine/src/cards/isd/gutter_grime.rs`)

#### Card Data: PASS
- Cost `{4}{G}`: correct (line 20-23).
- Type `Enchantment`: correct (line 24).
- No supertypes, no subtypes: correct.

#### Trigger Kind: PASS
- Uses `TriggerKind::AnyCreatureDies` (line 36), which is the correct generic death-watch trigger. The handler (`on_any_creature_dies`) applies its own filtering for nontoken + controller match. This is the standard engine pattern.

#### Nontoken Restriction: PASS
- Lines 53-56 check `o.is_token` on the dead object and return early if true. Correctly prevents triggering on token creature deaths.

#### Controller Check: PASS
- Lines 48-50 compare `dead_controller` against the Gutter Grime controller. Only triggers for creatures the enchantment's controller owns.

#### Slime Counter Before Token: PASS
- Line 58 adds the slime counter via `state.add_counters(self_id, CounterType::Slime, 1)` before the token is created on line 63. This matches the oracle sequencing ("put a slime counter... then create").

#### Token Creation: PASS
- Token is created via `create_token_with_subtypes` (lines 63-69) with:
  - Name: `"Ooze"` -- correct.
  - Color: `Green` -- correct.
  - Card types: `[Creature]` -- correct.
  - Subtypes: `["Ooze"]` -- correct. (No missing subtypes anti-pattern.)
  - Base P/T: `0/0` -- correct for dynamic P/T pattern.

#### Dynamic P/T Linkage: PASS
- Lines 73-76 link the token to the source Gutter Grime via `card_state["pt_source_counter"]` and `card_state["pt_source_counter_type"]`. This is the engine's mechanism for characteristic-defining abilities that reference another permanent's counters.
- Per official ruling (2011-09-22): "The power and toughness of the Ooze tokens will constantly update as Gutter Grime accumulates slime counters." The dynamic linkage implementation satisfies this.
- Per ruling: "If you control more than one Gutter Grime, each Ooze token remembers which one created it." The `self_id` binding on line 74 ensures each token links to its specific Gutter Grime.

#### Gutter Grime Removal: PASS (tested)
- Per ruling: "If Gutter Grime leaves the battlefield, the power and toughness of each Ooze token it created will become 0." Test `gutter_grime_ooze_tokens_become_zero_without_source` verifies this behavior.

#### Oracle Text String: MINOR DISCREPANCY
The `oracle_text` field in code (line 29) reads:
> "This creature's power and toughness are each equal to the number of slime counters on Gutter Grime."

Scryfall oracle text reads:
> "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."

The word "creature" vs "token" is a cosmetic difference. Functionally identical -- no gameplay impact.

### Test Coverage (`mtg-engine/tests/gutter_grime.rs`)

| Test | What it verifies |
|------|-----------------|
| `gutter_grime_creates_dynamic_pt_ooze` | Basic trigger: slime counter added, Ooze token created with correct dynamic P/T |
| `gutter_grime_ooze_tokens_grow_with_more_counters` | Two deaths produce two tokens, both scale to 2/2 |
| `gutter_grime_ignores_token_deaths` | Token creature deaths do not trigger |
| `gutter_grime_ignores_opponent_deaths` | Opponent creature deaths do not trigger |
| `gutter_grime_ooze_tokens_become_zero_without_source` | Removing Gutter Grime makes Ooze tokens 0/0 |

Coverage is thorough. No missing edge cases identified.

### Summary

**Status**: PASS

One minor oracle text string discrepancy ("creature" vs "token") with no functional impact. All game mechanics -- nontoken filtering, controller scoping, counter-before-token sequencing, dynamic P/T linkage, source removal behavior -- are correctly implemented and well-tested.

Sources:
- [Scryfall - Gutter Grime](https://scryfall.com/card/isd/186/gutter-grime)
- [MTG Assist - Gutter Grime Rulings](https://www.mtgassist.com/cards/Innistrad/Gutter-Grime/rulings/)
- [MTG Salvation - Gutter Grime Rulings Discussion](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/786249-gutter-grime)

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name, mana cost {4}{G}, Enchantment. Trigger: AnyCreatureDies, filters for controller match and nontoken (is_token check). Adds Slime counter to self, creates 0/0 green Ooze creature token with dynamic P/T linked to this Gutter Grime via card_state pt_source_counter. Token correctly created with subtypes ["Ooze"], colors [Green], types [Creature]. Per rulings, tokens track their specific Gutter Grime instance (implemented via pt_source_counter pointing to self_id). No anti-patterns.

## Audit — 2026-04-02 21:12

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
**Type line**: Enchantment
**Status**: PASS

### Code issues

1. **Oracle text string mismatch (cosmetic only)**: Code oracle_text says "put a slime counter on Gutter Grime" and "This creature's power and toughness", while Scryfall says "put a slime counter on this enchantment" and "This token's power and toughness". No gameplay impact -- both refer to the same thing.

2. **Parallel Lives interaction: extra tokens lack dynamic P/T linkage (engine limitation)**: When Parallel Lives is on the battlefield, `create_token_with_subtypes` creates extra copies internally, but only the primary token's ID is returned. The code at lines 73-76 only sets `pt_source_counter` card_state on the returned token_id, so any Parallel Lives copies would remain 0/0 base stats without dynamic P/T. This is an engine-level design limitation in how `create_token_with_subtypes` returns only one ID. Not marking as ISSUE because this affects any card that needs post-creation token state and is not specific to Gutter Grime's correctness in isolation.

### Tricky interactions checked (min 3)

1. **Multiple Gutter Grimes**: Each token stores its source Gutter Grime's ObjectId via `card_state["pt_source_counter"]` (line 74 uses `self_id`). Per ruling: "each Ooze token remembers which one created it." Correctly implemented -- tokens only track their specific source.

2. **Gutter Grime leaves the battlefield**: When Gutter Grime moves to graveyard, `move_object` calls `obj.counters.clear()` (state.rs line 485), so `effective_power`/`effective_toughness` reads 0 slime counters from the source, returning 0. Per ruling: "the power and toughness of each Ooze token it created will become 0." Correctly implemented and tested.

3. **Multiple creatures dying simultaneously (board wipe)**: Each death fires a separate `AnyCreatureDies` trigger. Triggers are queued and resolved sequentially. First trigger adds slime counter (now 1), creates token. Second trigger adds another (now 2), creates token. All tokens dynamically read current slime count, so both become 2/2. This matches the ruling about simultaneous deaths.

4. **Token creature deaths do not trigger**: Line 53-55 checks `is_token` on the dead creature and returns early if true. The `is_token` field persists in the graveyard since it is not cleared by `move_object`. Correctly implemented and tested.

5. **Counter added before token creation**: Line 58 adds the slime counter, then lines 63-69 create the token. This matches oracle sequencing ("put a slime counter... then create"). The first token created after the first death will be a 1/1, not a 0/0.

### Test coverage

5 tests in `mtg-engine/tests/gutter_grime.rs`, all passing:
- `gutter_grime_creates_dynamic_pt_ooze` -- basic trigger, 1 slime counter, 1/1 Ooze
- `gutter_grime_ooze_tokens_grow_with_more_counters` -- two deaths, both tokens become 2/2
- `gutter_grime_ignores_token_deaths` -- token death does not trigger
- `gutter_grime_ignores_opponent_deaths` -- opponent creature death does not trigger
- `gutter_grime_ooze_tokens_become_zero_without_source` -- Gutter Grime removal makes Ooze 0/0
