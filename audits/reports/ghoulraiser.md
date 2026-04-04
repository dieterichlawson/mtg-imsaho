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

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues

The previously identified issue has been fixed:

1. **"Zombie card" filter: FIXED.** Lines 50-54 now filter only by Zombie subtype:
   ```rust
   .filter(|o| {
       registry.card_data(o.card_id)
           .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
           .unwrap_or(false)
   })
   ```
   There is no creature-type check -- the filter matches any card with the Zombie subtype, which correctly implements "a Zombie card" (not "a Zombie creature card").

2. **Oracle text field matches exactly.** Line 28: `"When this creature enters, return a Zombie card at random from your graveyard to your hand."` -- matches Scryfall oracle verbatim.

3. **Card data correct.** Cost `{1}{B}{B}` (lines 18-20), type Creature (line 22), subtype Zombie (line 24), P/T 2/2 (lines 25-26).

4. **Random selection correct.** Uses `shuffle` then picks index 0 (lines 60-61). Uniform random selection.

5. **ETB trigger correct.** `TriggerKind::EntersBattlefield` (line 33), `on_enter_battlefield` handler (line 41).

6. **Zone handling correct.** Checks object is on battlefield (line 43), scans graveyard (line 48), moves chosen card to hand (line 63).

### Tricky interactions checked
- Empty graveyard (no Zombies): the `if !zombies.is_empty()` guard at line 58 correctly handles this as a no-op.
- Ghoulraiser itself is not in the graveyard when ETB triggers (it is on the battlefield), so it cannot return itself.

### Test coverage
- `ghoulraiser_returns_zombie_from_graveyard` -- basic functionality.
- Missing: empty graveyard case, multiple Zombies (randomness verification), non-creature Zombie card.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found. The ETB trigger correctly searches for any card with the Zombie subtype in the graveyard (not restricted to creature cards, matching oracle text which says "Zombie card"). Random selection is properly implemented using rand::thread_rng() and shuffle.

## Audit — 2026-04-02 21:09

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: When this creature enters, return a Zombie card at random from your graveyard to your hand.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues

No issues found. Full verification:

- **Card data**: Name "Ghoulraiser", cost {1}{B}{B} (Generic(1), Black, Black), type Creature, subtype Zombie, P/T 2/2 -- all match oracle exactly.
- **Oracle text field** (line 27): `"When this creature enters, return a Zombie card at random from your graveyard to your hand."` -- verbatim match with Scryfall.
- **ETB trigger**: `TriggerKind::EntersBattlefield` declared in `triggered_abilities` (line 33). `on_enter_battlefield` handler (line 41) invoked by engine trigger resolution after verifying object is still on battlefield.
- **Zombie filter** (lines 48-56): Filters `objects_in_zone(Zone::Graveyard, controller)` by subtype containing "Zombie". No creature-type restriction, correctly implementing "a Zombie card" per current oracle errata.
- **Random selection** (lines 59-61): `rand::thread_rng()` + `shuffle()` + pick index 0. Uniform random. Correct for "at random".
- **Zone move** (line 63): `move_object(chosen, Zone::Hand)`. Correct for "return ... to your hand".
- **Log message** (lines 64-65): Reports the returned card name. Adequate UI feedback.

### Tricky interactions checked (min 3)

1. **Empty graveyard / no Zombies**: Guard at line 58 (`if !zombies.is_empty()`) correctly makes this a no-op. The ability still triggers but does nothing, which matches MTG rules (the ability is not a "may" -- it returns a Zombie if possible, does nothing if not).
2. **Ghoulraiser cannot return itself**: When the ETB trigger resolves, Ghoulraiser is on the battlefield (verified by trigger resolution check at triggers.rs:894-895). The graveyard scan (line 48) only finds objects in Zone::Graveyard, so Ghoulraiser is excluded. Correct.
3. **Non-creature Zombie cards**: The filter checks only for "Zombie" in subtypes, not for CardType::Creature. This correctly handles hypothetical non-creature cards with Zombie subtype (e.g., Tribal -- Zombie spells). Matches oracle which says "a Zombie card" not "a Zombie creature card".
4. **Graveyard ownership**: `objects_in_zone(Zone::Graveyard, controller)` filters by owner for graveyard zone (state.rs:603). "Your graveyard" in oracle means the controller's graveyard. Under normal circumstances controller == owner, so this is correct.

### Test coverage

- `ghoulraiser_returns_zombie_from_graveyard` (tier11_cards.rs:146): Puts Walking Corpse in graveyard, casts Ghoulraiser, verifies Zombie returns to hand and Ghoulraiser is on battlefield. Covers the core happy path.
- **Missing**: No test for empty graveyard (no-op), no test for multiple Zombies (randomness), no test for non-Zombie exclusion.
