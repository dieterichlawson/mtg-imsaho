# Audit: Rebuke

## Official Oracle
- **Name:** Rebuke
- **Cost:** {2}{W}
- **Type:** Instant
- **Oracle Text:** Destroy target attacking creature.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{W} — OK
- **Type:** Instant — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Target:** CreatureWithFilter(Attacking) — OK
- **is_valid_target:** Checks battlefield, is creature, is in combat.attackers — OK
- **on_resolve:** Uses resolve_destroy helper — OK

## Issues
None found.

## Verdict: PASS

---

# Audit: Rebuke (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rebuke
- **Mana Cost:** {2}{W}
- **Type:** Instant
- **Oracle Text:** Destroy target attacking creature.

## Card Data Verification
- **Name:** Correct ("Rebuke")
- **Cost:** Correct ({2}{W})
- **Type:** Correct (Instant)
- **Keywords:** Correct (none)

## Behavior Verification
- **Targeting:** Correct — `TargetRequirement::CreatureWithFilter(TargetFilter::Attacking)` targets attacking creatures. `is_valid_target` verifies the creature is on the battlefield and is in `combat.attackers`.
- **Effect:** Correct — `resolve_destroy` handles the destroy effect on resolution.
- **Player target rejection:** Correct — returns false for `Target::Player`.

## Result: PASS
