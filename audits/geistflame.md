# Audit: Geistflame

## Oracle Reference (Scryfall)
- Cost: {R}
- Type: Instant
- Oracle: "Geistflame deals 1 damage to any target.
  Flashback {3}{R}"

## Implementation: geistflame.rs

## Issues Found

No issues found. Name, cost ({R}), type (Instant), oracle text, flashback cost ({3}{R}), target requirement (AnyTarget), and damage amount (1) all match. Uses resolve_damage helper which correctly handles damaged_by tracking and NonCombatDamageDealt events.

## Verdict: PASS
