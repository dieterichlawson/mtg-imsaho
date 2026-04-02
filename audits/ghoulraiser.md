# Audit: Ghoulraiser

## Oracle Reference (Scryfall)
- Cost: {1}{B}{B}
- Type: Creature -- Zombie
- P/T: 2/2
- Oracle: "When Ghoulraiser enters the battlefield, return a Zombie creature card at random from your graveyard to your hand."

NOTE: Current Scryfall oracle errata says "Zombie card" not "Zombie creature card". Original printing said "Zombie creature card".

## Implementation: ghoulraiser.rs

## Issues Found

1. **BUG (from prior audit): Filters for "Zombie creature card" instead of "Zombie card"** - Engine code (line 51-53) filters for is_creature && is_zombie, but updated oracle only requires Zombie subtype. Low severity since all Zombies in the set are creatures.

Otherwise correct: cost ({1}{B}{B}), type (Creature), subtype (Zombie), P/T (2/2), ETB trigger, random selection.

## Verdict: ISSUES FOUND (1 minor issue)

---

## Audit 2 (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
> When this creature enters, return a Zombie card at random from your graveyard to your hand.

### Implementation Review (`mtg-engine/src/cards/isd/ghoulraiser.rs`)

**Card data:** Cost {1}{B}{B}, Creature -- Zombie, 2/2. All correct.

**Triggered ability:** `TriggerKind::EntersBattlefield` defined in `triggered_abilities` vec (line 33-37). `on_enter_battlefield` handler implemented (line 41). Correct.

**Random selection:** Uses `rand::seq::SliceRandom::shuffle` then picks index 0 (lines 63-65). Correct.

**Zone handling:** Controller lookup checks object is on the battlefield (line 43). Graveyard scan uses `objects_in_zone(Zone::Graveyard, controller)` (line 48). Return uses `move_object(chosen, Zone::Hand)` (line 67). All correct.

### Issues Found

1. **BUG (confirmed from prior audit): "Zombie creature card" vs "Zombie card"**
   - Oracle: `"return a Zombie card at random from your graveyard to your hand."`
   - Implementation oracle_text (line 27): `"return a Zombie creature card at random from your graveyard to your hand."`
   - Implementation filter (lines 51-57): requires `is_creature && is_zombie`, excluding non-creature cards with Zombie subtype (e.g., Tribal spells).
   - Severity: Low in Innistrad limited (all Zombie-subtyped cards in ISD are creatures), but technically incorrect per current oracle.

2. **No new issues found.** ETB trigger, random selection, zone handling, and card data are all correct.

### Test Coverage (`mtg-engine/tests/tier11_cards.rs`, line 146)
- `ghoulraiser_returns_zombie_from_graveyard`: Puts a Walking Corpse in graveyard, casts Ghoulraiser, verifies Zombie returns to hand. Passes.
- **Missing tests:** No test for empty graveyard (no-op case), no test for non-Zombie cards being excluded, no test for multiple Zombies verifying randomness.

### Verdict: ISSUES FOUND (1 known bug, no new issues)
