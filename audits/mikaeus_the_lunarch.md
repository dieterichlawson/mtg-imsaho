# Audit: Mikaeus, the Lunarch

## Reference (Scryfall/API)
- **Name:** Mikaeus, the Lunarch
- **Mana Cost:** {X}{W}
- **Type:** Legendary Creature — Human Cleric
- **Oracle:** Mikaeus enters with X +1/+1 counters on it. / {T}: Put a +1/+1 counter on Mikaeus. / {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
- **P/T:** 0/0

## Implementation: `mikaeus_the_lunarch.rs`
- **Name:** Mikaeus, the Lunarch -- CORRECT
- **Mana Cost:** {X}{W} (ManaSymbol::X + Colored White) -- CORRECT
- **Type:** Legendary Creature — Human Cleric -- CORRECT (Supertype::Legendary, subtypes Human/Cleric)
- **P/T:** 0/0 -- CORRECT
- **ETB with X counters:** Reads `x_value`, adds PlusOnePlusOne counters -- CORRECT
- **Ability 0: {T} add counter:** requires_tap=true, cost=free, adds 1 +1/+1 counter -- CORRECT
- **Ability 1: {T} remove counter, distribute:** requires_tap=true, cost=free, checks has_counter > 0, removes 1 counter, adds 1 +1/+1 to each other creature you control -- CORRECT
- **Other creatures filter:** `o.id != object_id && o.power.is_some()` scoped to controller's battlefield -- CORRECT

### Minor: Oracle text wording

- **Code:** "Mikaeus, the Lunarch enters the battlefield with X +1/+1 counters on it."
- **Oracle:** "Mikaeus enters with X +1/+1 counters on it."

This is a cosmetic/template difference (WotC updated oracle templates); functionally identical.

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Mikaeus enters with X +1/+1 counters on it.\n{T}: Put a +1/+1 counter on Mikaeus.\n{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
**Type line**: Legendary Creature — Human Cleric
**Status**: PASS
### Code issues
None. Card data and behavior match oracle. Minor cosmetic difference: code uses "enters the battlefield with" vs oracle's "enters with" -- functionally equivalent. All three abilities (ETB counters, tap to add counter, tap+remove to distribute) are correctly implemented.
