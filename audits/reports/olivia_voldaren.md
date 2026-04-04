# Audit: Olivia Voldaren

## Reference (Scryfall/API)
- **Name:** Olivia Voldaren
- **Mana Cost:** {2}{B}{R}
- **Type:** Legendary Creature -- Vampire
- **Oracle:** Flying / {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. / {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
- **P/T:** 3/3

## Implementation: `olivia_voldaren.rs`
- **Name:** Olivia Voldaren -- CORRECT
- **Mana Cost:** {2}{B}{R} -- CORRECT
- **Type:** Legendary Creature -- Vampire -- CORRECT (supertypes: [Legendary], subtypes: ["Vampire"])
- **P/T:** 3/3 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Ability 0:** {1}{R}, targets another creature, deals 1 damage, adds Vampire subtype, adds +1/+1 counter -- CORRECT
- **Ability 1:** {3}{B}{B}, targets Vampire, gains control for as long as you control Olivia -- CORRECT
- **on_leave_battlefield:** Returns stolen creatures to original controllers -- CORRECT
- **Triggered ability:** LeavesBattlefield registered for stolen creature cleanup -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature -- Vampire
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {2}{B}{R}, 3/3, Legendary Creature -- Vampire, Flying keyword, two activated abilities with correct costs and targeting, control-change effect tracked via card_state with proper cleanup on leave. Behavior is correct.
