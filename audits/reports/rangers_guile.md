# Audit: Ranger's Guile

## Official Oracle
- **Name:** Ranger's Guile
- **Cost:** {G}
- **Type:** Instant
- **Oracle Text:** Target creature you control gets +1/+1 and gains hexproof until end of turn.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {G} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Target:** CreatureWithFilter(YouControl) — OK
- **is_valid_target:** Checks battlefield, is creature, controller matches — OK
- **on_resolve:** Applies +1/+1 UntilEndOfTurnEffect and Hexproof UntilEndOfTurnKeyword — OK

## Issues
None found.

## Verdict: PASS

---

# Audit: Ranger's Guile (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Ranger's Guile
- **Mana Cost:** {G}
- **Type:** Instant
- **Oracle Text:** Target creature you control gets +1/+1 and gains hexproof until end of turn.

## Card Data Verification
- **Name:** Correct ("Ranger's Guile")
- **Cost:** Correct ({G})
- **Type:** Correct (Instant)
- **Keywords:** Correct (none on the card itself)

## Behavior Verification
- **Targeting:** Correct — `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` targets a creature you control. `is_valid_target` verifies battlefield, is creature, and controller match.
- **Effect:** Correct — applies `power_mod: 1, toughness_mod: 1` and grants `Keyword::Hexproof` until end of turn.
- **Cleanup:** Correct — calls `move_spell_after_resolve`.

## Result: PASS
