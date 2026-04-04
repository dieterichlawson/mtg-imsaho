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

## Audit — 2026-04-03 21:31

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Lethal damage timing with type change: PASS - creature becomes Vampire before dying per oracle ruling
- "Another" targeting restriction: PASS - uses `TargetFilter::Another` and additional self-target check
- Permanent type change: PASS - Vampire subtype added permanently, not until Olivia leaves
- Control duration dependency: PASS - stolen creatures return when Olivia leaves battlefield via triggered ability
- Multiple stolen creatures handling: PASS - uses indexed card_state entries to track multiple targets
- Independence of ability components: PASS - type change and +1/+1 counter occur even if damage is prevented

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- First ability deals damage: `olivia_voldaren.rs:23` / `tier14_cards.rs:463`
- Target becomes Vampire in addition to other types: `olivia_voldaren.rs:38-41` / `tier14_cards.rs:489`
- +1/+1 counter on Olivia: `olivia_voldaren.rs:44-46` / `tier14_cards.rs:492-495`
- "Another" targeting (can't target self): `olivia_voldaren.rs:51`
- Control change of Vampires: `olivia_voldaren.rs:68`
- Rejection of non-Vampire targets for ability 1: `olivia_voldaren.rs:86`
- Return of stolen creatures when Olivia leaves: `olivia_voldaren.rs:104`
- Target filter validation for Vampire requirement: `olivia_voldaren.rs:134`
- Lethal damage timing with type change: NOT TESTED
- Control loss before ability resolution: NOT TESTED
