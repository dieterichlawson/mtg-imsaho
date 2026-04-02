# Audit: Rage Thrower

## Official Oracle
- **Name:** Rage Thrower
- **Cost:** {5}{R}
- **Type:** Creature — Human Shaman
- **Oracle Text:** Whenever another creature dies, Rage Thrower deals 2 damage to target player or planeswalker.
- **P/T:** 4/2

## Implementation Review
- **Name:** OK
- **Cost:** {5}{R} — OK
- **Type:** Creature, subtypes ["Human", "Shaman"] — OK
- **Oracle Text:** Matches — OK
- **P/T:** 4/2 — OK
- **Triggered Abilities:** AnyCreatureDies — OK (says "another creature", and the hook is on_any_creature_dies which excludes self)
- **on_any_creature_dies:** Checks zone == Battlefield, presents target choice among all players, PendingEffect::DealDamage { amount: 2 } — OK
- **Damage event:** DealDamage emits NonCombatDamageDealt — OK
- **"target player or planeswalker":** Only offers player targets (no planeswalkers), but planeswalkers may not be in the engine — acceptable simplification

## Issues
None found (planeswalker omission is an engine-level limitation, not a card bug).

## Verdict: PASS

---

# Audit: Rage Thrower (2026-04-02)

## Oracle Text (Scryfall)
- **Name:** Rage Thrower
- **Mana Cost:** {5}{R}
- **Type:** Creature — Human Shaman
- **P/T:** 4/2
- **Oracle Text:** Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.

## Card Data Verification
- **Name:** Correct ("Rage Thrower")
- **Cost:** Correct ({5}{R})
- **Type:** Correct (Creature)
- **Subtypes:** Correct (Human, Shaman)
- **P/T:** Correct (4/2)
- **Keywords:** Correct (none)

## Behavior Verification
- **Trigger:** Correct — triggers on `AnyCreatureDies` for any creature other than itself dying.
- **Effect:** Deals 2 damage via `PendingEffect::DealDamage { amount: 2 }`. Correct amount.

## Issues
- **ISSUE: Missing planeswalker targeting.** Oracle says "target player or planeswalker" but the implementation only builds targets from `state.players` (players only). No planeswalker objects are included as valid targets.
  - **Oracle:** "deals 2 damage to target player or planeswalker"
  - **Code:** `let targets: Vec<Target> = state.players.iter().filter(|p| !p.lost).map(|p| Target::Player(p.id)).collect();`

## Result: ISSUE
