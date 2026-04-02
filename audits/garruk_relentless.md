# Audit: Garruk Relentless / Garruk, the Veil-Cursed

## Oracle Reference (Scryfall)
**Front Face: Garruk Relentless**
- Cost: {3}{G}
- Type: Legendary Planeswalker -- Garruk
- Loyalty: 3
- Oracle: "When Garruk Relentless has two or fewer loyalty counters on him, transform him.
  0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him.
  0: Create a 2/2 green Wolf creature token."

**Back Face: Garruk, the Veil-Cursed**
- Type: Legendary Planeswalker -- Garruk
- Color: Black, Green
- Oracle: "+1: Create a 1/1 black Wolf creature token with deathtouch.
  -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
  -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard."

## Implementation: garruk_relentless.rs

## Issues Found

1. **ISSUE: Back face not fully implemented** - The comment says "Simplified: Front face only. Back face (Garruk, the Veil-Cursed) is not implemented." The back face has 3 loyalty abilities (+1 wolf with deathtouch, -1 sacrifice+tutor, -3 overrun). After transform, these abilities are unavailable. This is a significant gameplay simplification.

2. **ISSUE: Transform trigger is wrong type** - Oracle says "When Garruk Relentless has two or fewer loyalty counters on him, transform him" - this is a state-triggered ability, not something checked only after loyalty abilities activate. The implementation only checks after on_loyalty_ability, so it won't trigger if Garruk takes damage from combat or other sources.

3. **ISSUE: Missing NonCombatDamageDealt event for creature-to-planeswalker damage** - When the targeted creature deals damage back to Garruk, the implementation just removes loyalty counters directly (line 90-94) without emitting a NonCombatDamageDealt event.

4. **ISSUE: Front face oracle text says "to him" but code oracle says "to him"** - Matches. OK.

5. **MINOR: Wolf tokens from front face** - Front face creates 2/2 green Wolf tokens. This matches oracle. OK.

## Verdict: ISSUES FOUND (3 issues)

## Audit — 2026-04-01 08:20

**Scryfall Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform him. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Scryfall Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. −1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle your library. −3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: PASS (with accepted simplifications)

Previous issue #1 (back face not implemented) is now FIXED. All 3 back face loyalty abilities are implemented:
- +1: Creates 1/1 black Wolf with deathtouch (ability_index 10)
- -1: Sacrifices weakest creature, searches library for creature card (ability_index 11)
- -3: Gives all controlled creatures +X/+X and trample until end of turn (ability_index 12)

The loyalty_abilities trait method now takes state and object_id parameters to support returning different abilities based on is_transformed.

Previous issue #2 (transform trigger type) remains an accepted simplification — transform is checked after loyalty ability activation only.
Previous issue #3 (missing damage event for creature-to-planeswalker) remains an accepted simplification.

Test coverage: 6 tests covering front face wolf creation, transform condition, back face deathtouch wolf, sacrifice-to-tutor, overrun effect, and loyalty abilities list verification.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform him. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Scryfall Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: ISSUE

Findings:
- Mana cost {3}{G}: correct.
- Types: Legendary Planeswalker, subtypes Garruk: correct.
- Starting loyalty 3: correct.
- Front face abilities:
  - 0: Deals 3 damage to target creature, creature deals power back: correct. Uses NonCombatDamageDealt event for the 3 damage: correct.
  - 0: Create 2/2 green Wolf token with subtypes ["Wolf"]: correct.
- Back face abilities:
  - +1: Create 1/1 black Wolf with deathtouch, subtypes ["Wolf"]: correct.
  - -1: Sacrifice creature, search library for creature card: correct.
  - -3: Creatures get +X/+X and trample where X = creature cards in graveyard: correct.
- Transform check (loyalty <= 2 triggers transform): implemented after every loyalty ability activation.
- ISSUE 1 (carried forward): Transform is a state-triggered ability per oracle, but implementation only checks after on_loyalty_ability. Won't trigger if Garruk loses loyalty from combat damage or other non-loyalty-ability sources.
- ISSUE 2 (carried forward): When target creature deals damage back to Garruk (ability 0), no NonCombatDamageDealt event is emitted for the creature-to-planeswalker damage. Only loyalty counters are removed directly.
- ISSUE 3: Wolf tokens from both faces have correct subtypes ["Wolf"] passed to create_token_with_subtypes: correct. No missing token subtypes.
- Anti-pattern check: on_resolve uses move_object to battlefield (correct for planeswalker permanent). No spell-to-graveyard anti-pattern.
- No CombatDamageDealt misuse for non-combat damage: correct (uses NonCombatDamageDealt).
- triggered_abilities vec is empty despite having a state-triggered transform ability. This is technically a missing declaration, though the transform check is handled inline in on_loyalty_ability.
- Tests found in tier15_cards.rs.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed) and Gatherer (https://gatherer.wizards.com/ISD/en-us/181/garruk-the-veil-cursed)
**Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform Garruk Relentless. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: ISSUE

Findings:
- Mana cost {3}{G}: correct.
- Types: Legendary Planeswalker, subtypes Garruk: correct.
- Starting loyalty 3: correct.
- Front face ability 0 (fight): Deals 3 damage to target creature (NonCombatDamageDealt emitted, damaged_by tracked): correct. Creature deals power damage back removing loyalty counters: correct.
- Front face ability 0 (wolf): Creates 2/2 green Wolf token with subtypes ["Wolf"]: correct.
- Back face ability +1: Creates 1/1 black Wolf with deathtouch, subtypes ["Wolf"]: correct.
- Back face ability -1: Sacrifices creature, searches library for creature card, moves to hand: correct.
- Back face ability -3: Creatures get +X/+X and trample where X = creature cards in graveyard: correct.
- on_resolve moves to battlefield and adds 3 loyalty counters: correct.
- ISSUE 1 (carried forward): Transform condition ("When Garruk has two or fewer loyalty counters") is a state-triggered ability but is only checked after on_loyalty_ability. If Garruk loses loyalty from combat damage or other sources (e.g., being attacked), the transform will not trigger. The triggered_abilities vec is empty.
- ISSUE 2 (carried forward): When target creature deals damage back to Garruk (front face ability 0, lines 118-123), no NonCombatDamageDealt event is emitted for the creature-to-planeswalker damage direction. Only loyalty counters are decremented directly.
- ISSUE 3: Front face ability 0 (fight) auto-selects the strongest opponent creature (line 99-103) rather than letting the controller choose a target. The oracle says "target creature" which should allow any creature, not just opponent's strongest.
- Anti-pattern check: on_resolve uses move_object to battlefield (correct for planeswalker). No spell-to-graveyard anti-pattern.
- Uses NonCombatDamageDealt for Garruk's 3 damage: correct.
- Tests: 6 tests in tier15_cards.rs covering wolf creation, transform at 2 loyalty, back face deathtouch wolf, sacrifice-to-tutor, overrun, and ability list. Reasonable coverage.

## Audit — 2026-04-01 14:37

**Oracle text source**: Scryfall via WebSearch (https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed)
**Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform him. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: ISSUE

Confirmed issues (all previously identified, still present):

1. **Transform is not a proper state-triggered ability** (`garruk_relentless.rs` lines 246-257).
   - Oracle text says: `When Garruk Relentless has two or fewer loyalty counters on him, transform him.`
   - Code does: Transform check only runs at the end of `on_loyalty_ability`. If Garruk loses loyalty from combat damage or other non-loyalty-ability sources, the transform will not trigger. The `triggered_abilities` vec is empty despite the oracle having this state-triggered ability.

2. **No NonCombatDamageDealt event for creature-to-planeswalker damage** (`garruk_relentless.rs` lines 118-123).
   - Oracle text says: `That creature deals damage equal to its power to him.`
   - Code does: Directly decrements loyalty counters via `loyalty.saturating_sub(remove)` without emitting a `NonCombatDamageDealt` event for the creature's damage to Garruk. Only the 3 damage from Garruk to the creature emits an event (line 112-116).

3. **Front face fight ability auto-selects target** (`garruk_relentless.rs` lines 99-103).
   - Oracle text says: `Garruk Relentless deals 3 damage to target creature.`
   - Code does: Auto-selects the strongest opponent creature via `max_by_key`. The oracle says "target creature" which should allow the controller to choose any creature (including own creatures), not just the opponent's strongest.

No new issues found. Test coverage is adequate (6 tests in tier15_cards.rs).

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text (front)**: When Garruk has two or fewer loyalty counters on him, transform him. 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Starting loyalty**: 3
**Rulings**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability. It triggers once Garruk has two or fewer loyalty counters on him and it can't retrigger while that ability is on the stack.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The -1 ability doesn't target a creature, but you must sacrifice one if you control one.
- [2011-09-22] The -3 bonus is locked in at resolution and doesn't change later.
- [2011-09-22] Only creatures you control when -3 resolves get the bonus.
**Status**: ISSUE

### Code issues

1. **Transform is not a state-triggered ability** (`mtg-engine/src/cards/isd/garruk_relentless.rs`, lines 246-257):
   - Oracle text says: `When Garruk has two or fewer loyalty counters on him, transform him.`
   - Ruling says: "Garruk Relentless's first ability is a state-triggered ability."
   - Code does: Transform check only runs at the end of `on_loyalty_ability()`. If Garruk loses loyalty from combat damage or other non-loyalty-ability sources (e.g., being attacked, Lightning Bolt targeting him), the transform will not trigger. The `triggered_abilities` vec is also empty.

2. **No damage event for creature-to-planeswalker damage** (`mtg-engine/src/cards/isd/garruk_relentless.rs`, lines 118-123):
   - Oracle text says: `That creature deals damage equal to its power to him.`
   - Code does: Directly decrements loyalty counters via `loyalty.saturating_sub(remove)` without emitting a `NonCombatDamageDealt` event. The 3 damage Garruk deals to the creature does emit the event (line 112), but the reverse direction does not.

3. **Front face fight ability auto-selects target** (`mtg-engine/src/cards/isd/garruk_relentless.rs`, lines 99-103):
   - Oracle text says: `Garruk deals 3 damage to target creature.`
   - Code does: Auto-selects the strongest opponent creature via `max_by_key(|(_, p)| *p)`. The word "target" means the controller should choose which creature to target. The code also restricts to opponent's creatures only, but the oracle says "target creature" with no controller restriction.

### Tricky interactions checked
- Starting loyalty 3: PASS (starting_loyalty returns Some(3), on_resolve adds 3 counters)
- Legendary supertype: PASS (line 31)
- Front face wolf token 2/2 green with Wolf subtype: PASS (line 131-139)
- Back face +1 wolf 1/1 black with deathtouch and Wolf subtype: PASS (lines 147-155)
- Back face -1 sacrifice is mandatory if you control a creature (ruling): PASS (line 163-170 finds a creature and sacrifices it; line 202 handles no-creature case)
- Back face -1 does not target (ruling): PASS (no targeting in the implementation, auto-selects)
- Back face -3 counts creatures in graveyard at resolution: PASS (line 210-221 counts at time of ability)
- Back face -3 only affects creatures controlled at resolution: PASS (line 223-227 collects current creatures)
- Loyalty abilities show correct costs (+1, -1, -3): PASS (lines 54-69)
- No loyalty added/removed on transform (ruling): PASS (lines 249-256 only flip is_transformed and name)

### Test coverage
- Front face creates 2/2 Wolf token: `tier15_cards.rs:1046` (garruk_creates_wolf_token)
- Transforms at 2 or fewer loyalty: `tier15_cards.rs:1068` (garruk_transforms_at_two_or_fewer_loyalty)
- Back face creates 1/1 black Wolf with deathtouch: `tier15_cards.rs:1088` (garruk_back_face_creates_deathtouch_wolf)
- Back face sacrifice-to-tutor: `tier15_cards.rs:1116` (garruk_back_face_sacrifice_to_tutor)
- Back face -3 overrun effect: `tier15_cards.rs:1154` (garruk_back_face_overrun)
- Back face loyalty abilities list: `tier15_cards.rs:1194` (garruk_back_face_loyalty_abilities_shown_when_transformed)
- Transform from combat damage (state-triggered): NOT TESTED (bug: not implemented)
- Front face fight targets player-chosen creature: NOT TESTED (bug: auto-selects)
- Creature damage back to Garruk emits event: NOT TESTED (bug: no event emitted)
- Cannot activate loyalty abilities on both faces in same turn (ruling): NOT TESTED
- -1 is mandatory if creature is controlled (ruling): NOT TESTED (implemented correctly but no test)

## Audit — 2026-04-01 13:35

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text (front)**: When Garruk has two or fewer loyalty counters on him, transform him. 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Starting loyalty**: 3
**Rulings**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The -1 ability doesn't target a creature, but you must sacrifice one if you control one.
- [2011-09-22] The -3 bonus is locked in at resolution and doesn't change later.
- [2011-09-22] Only creatures you control when -3 resolves get the bonus.
**Status**: PASS

### Code issues
No issues found.

All three issues from the previous audit have been fixed:

1. **Transform is now a state-triggered ability** (fixed in `mtg-engine/src/sba.rs`, lines 247-266): The `check_state_based_actions_with_registry` function checks for any Garruk Relentless on the battlefield with 2 or fewer loyalty counters and transforms him. This runs after every game action, correctly implementing the state-triggered ability regardless of the source of loyalty loss (combat damage, spells, abilities). The comment at line 252 in `garruk_relentless.rs` confirms: "Transform check is now handled as a state-triggered ability in SBA."

2. **Creature-to-planeswalker damage event now emitted** (fixed at lines 124-128 of `garruk_relentless.rs`): The code now pushes a `NonCombatDamageDealt` event with `source: *target_id` and `target: DamageTarget::Object(self_id)` when the creature deals its power as damage back to Garruk.

3. **Front face ability 0 now uses player-chosen targets** (fixed at line 104 of `garruk_relentless.rs`): The code reads targets from the `targets` parameter (`targets.first()`) rather than auto-selecting. The loyalty ability definition at line 82 specifies `target_requirement: Some(TargetRequirement::Creature)`, and the default `is_valid_target` returns `true` for all creatures, so any creature can be targeted. No controller restriction.

### Tricky interactions checked
- Starting loyalty 3: PASS (starting_loyalty returns Some(3), on_resolve adds 3 counters)
- Legendary supertype: PASS (line 32)
- State-triggered transform at <= 2 loyalty: PASS (SBA check in sba.rs)
- No loyalty change on transform (ruling): PASS (SBA only flips is_transformed and name)
- Front face ability 0 targets any creature: PASS (TargetRequirement::Creature, default is_valid_target)
- Front face ability 0 reads power before dealing damage: PASS (line 105 reads power, then lines 108-115 deal damage)
- Front face wolf token 2/2 green with Wolf subtype: PASS (lines 136-144)
- Back face +1 wolf 1/1 black with deathtouch and Wolf subtype: PASS (lines 152-160)
- Back face -1 sacrifice is mandatory if creature controlled (ruling): PASS (lines 168-172 find a creature; line 207 handles no-creature case)
- Back face -1 does not target (ruling): PASS (no targeting, auto-selects via AI heuristic)
- Back face -1 sacrifice uses weakest-creature heuristic: PASS (documented as AI-driven; line 172 `min_by_key`)
- Back face -3 counts creatures in graveyard at resolution (ruling): PASS (lines 215-226 count at resolution time)
- Back face -3 only affects creatures controlled at resolution (ruling): PASS (lines 228-232 collect current creatures)
- Loyalty abilities show correct costs (0/0 for front, +1/-1/-3 for back): PASS

### Test coverage
- Front face creates 2/2 Wolf token: `tier15_cards.rs:1046` (garruk_creates_wolf_token)
- Transforms at 2 or fewer loyalty (via SBA): `tier15_cards.rs:1068` (garruk_transforms_at_two_or_fewer_loyalty)
- Back face creates 1/1 black Wolf with deathtouch: `tier15_cards.rs:1091` (garruk_back_face_creates_deathtouch_wolf)
- Back face sacrifice-to-tutor: `tier15_cards.rs:1119` (garruk_back_face_sacrifice_to_tutor)
- Back face -3 overrun effect: `tier15_cards.rs:1157` (garruk_back_face_overrun)
- Back face loyalty abilities list: `tier15_cards.rs:1197` (garruk_back_face_loyalty_abilities_shown_when_transformed)
- Transform from non-loyalty-ability damage source (e.g., combat): NOT TESTED (fixed, but test uses loyalty ability path)
- Cannot activate loyalty abilities on both faces in same turn (ruling): NOT TESTED
- -1 is mandatory if creature is controlled (ruling): NOT TESTED (implemented correctly but no test)
- Front face ability 0 creature fights back: NOT TESTED (implemented correctly but no test)

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text (front)**: When Garruk has two or fewer loyalty counters on him, transform him. 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Starting loyalty**: 3
**Rulings**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The -1 ability doesn't target a creature, but you must sacrifice one if you control one.
- [2011-09-22] The -3 bonus is locked in at resolution and doesn't change later.
- [2011-09-22] Only creatures you control when -3 resolves get the bonus.
**Status**: ISSUE

### Code issues

1. **Missing library shuffle after -1 tutor** (`mtg-engine/src/cards/isd/garruk_relentless.rs`, lines 196-206):
   - Oracle text says: `search your library for a creature card, reveal it, put it into your hand, then shuffle.`
   - Code does: Removes the found creature from `library_order` and moves it to hand, but never shuffles the library. Compare with `caravan_vigil.rs` lines 73 and 80 which correctly call `library_order.shuffle(&mut rng)` after searching. This is a concrete bug -- after resolving -1, the library remains in its pre-search order rather than being randomized.

### Tricky interactions checked
- Starting loyalty 3: PASS (starting_loyalty returns Some(3), on_resolve adds 3 counters)
- Legendary supertype: PASS (line 32)
- State-triggered transform at <= 2 loyalty: PASS (SBA check in sba.rs lines 247-266)
- No loyalty change on transform (ruling): PASS (SBA only flips is_transformed and name)
- Front face ability 0 targets any creature via player choice: PASS (reads from `targets` parameter at line 104)
- Front face ability 0 reads power before dealing damage: PASS (line 105)
- Creature damage back to Garruk emits NonCombatDamageDealt: PASS (lines 124-128)
- Front face wolf token 2/2 green with Wolf subtype: PASS (lines 136-144)
- Back face +1 wolf 1/1 black with deathtouch and Wolf subtype: PASS (lines 152-160)
- Back face -1 sacrifice is mandatory if creature controlled (ruling): PASS (lines 168-172)
- Back face -1 does not target (ruling): PASS (no targeting, auto-selects via AI heuristic)
- Back face -1 library search auto-selects: accepted engine pattern (auto-choice heuristic)
- Back face -1 library shuffle after search: FAIL (see issue above)
- Back face -3 counts creatures in graveyard at resolution (ruling): PASS (lines 215-226)
- Back face -3 only affects creatures controlled at resolution (ruling): PASS (lines 228-232)
- Loyalty abilities show correct costs (0/0 for front, +1/-1/-3 for back): PASS

### Test coverage
- Front face creates 2/2 Wolf token: `tier15_cards.rs:1046` (garruk_creates_wolf_token)
- Transforms at 2 or fewer loyalty (via SBA): `tier15_cards.rs:1068` (garruk_transforms_at_two_or_fewer_loyalty)
- Back face creates 1/1 black Wolf with deathtouch: `tier15_cards.rs:1091` (garruk_back_face_creates_deathtouch_wolf)
- Back face sacrifice-to-tutor: `tier15_cards.rs:1119` (garruk_back_face_sacrifice_to_tutor)
- Back face -3 overrun effect: `tier15_cards.rs:1157` (garruk_back_face_overrun)
- Back face loyalty abilities list: `tier15_cards.rs:1197` (garruk_back_face_loyalty_abilities_shown_when_transformed)
- Library shuffle after -1 tutor: NOT TESTED (bug: shuffle not implemented)
- Transform from non-loyalty-ability damage source (e.g., combat): NOT TESTED (fixed but test uses loyalty ability path)
- Cannot activate loyalty abilities on both faces in same turn (ruling): NOT TESTED
- -1 is mandatory if creature is controlled (ruling): NOT TESTED (implemented correctly)
- Front face ability 0 creature fights back: NOT TESTED (implemented correctly)

## Audit — 2026-04-01 20:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text (front)**: When Garruk has two or fewer loyalty counters on him, transform him. 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Starting loyalty**: 3
**Rulings**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The -1 ability doesn't target a creature, but you must sacrifice one if you control one.
- [2011-09-22] The -3 bonus is locked in at resolution and doesn't change later.
- [2011-09-22] Only creatures you control when -3 resolves get the bonus.
**Status**: ISSUE

### Code issues

1. **-1 ability auto-selects sacrifice and tutor targets** (`mtg-engine/src/cards/isd/garruk_relentless.rs`, lines 168-206):
   - Oracle text says: `Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.`
   - Code does: Auto-selects the weakest creature to sacrifice (line 172 `min_by_key`) and the first creature found in library (line 181 `.find()`). Per MTG rules, both the sacrifice choice and the library search choice belong to the player. The player should choose which creature to sacrifice and which creature card to find. These are currently implemented as heuristic auto-choices.

### Tricky interactions checked
- Starting loyalty 3: PASS (starting_loyalty returns Some(3), on_resolve adds 3 counters)
- Legendary supertype: PASS (line 32)
- State-triggered transform at <= 2 loyalty: PASS (SBA check in sba.rs lines 247-266, fires from any loyalty loss source)
- No loyalty change on transform (ruling): PASS (SBA only flips is_transformed and name)
- Front face ability 0 targets any creature via player choice: PASS (reads from `targets` parameter at line 104)
- Front face ability 0 reads power before dealing 3 damage: PASS (line 105 reads effective_power first)
- Creature damage back to Garruk emits NonCombatDamageDealt: PASS (lines 124-128)
- Creature damage back removes loyalty counters: PASS (line 121 `loyalty.saturating_sub(remove)`)
- Front face wolf token 2/2 green with Wolf subtype: PASS (lines 136-144)
- Back face +1 wolf 1/1 black with deathtouch and Wolf subtype: PASS (lines 152-160)
- Back face -1 sacrifice is mandatory if creature controlled (ruling): PASS (lines 168-174)
- Back face -1 does not target (ruling): PASS (no targeting in implementation)
- Back face -1 library shuffle after search: PASS (lines 209-213)
- Back face -3 counts creatures in graveyard at resolution (ruling): PASS (lines 222-233)
- Back face -3 only affects creatures controlled at resolution (ruling): PASS (lines 235-239)
- Back face -3 applies both +X/+X and trample: PASS (lines 241-251)
- Loyalty abilities show correct costs (0/0 for front, +1/-1/-3 for back): PASS

### Test coverage
- Front face creates 2/2 Wolf token: `tier15_cards.rs:1116` (garruk_creates_wolf_token)
- Transforms at 2 or fewer loyalty (via SBA): `tier15_cards.rs:1138` (garruk_transforms_at_two_or_fewer_loyalty)
- Back face creates 1/1 black Wolf with deathtouch: `tier15_cards.rs:1161` (garruk_back_face_creates_deathtouch_wolf)
- Back face sacrifice-to-tutor: `tier15_cards.rs:1189` (garruk_back_face_sacrifice_to_tutor)
- Back face tutor shuffles library: `tier15_cards.rs:1227` (garruk_back_face_tutor_shuffles_library)
- Back face -3 overrun effect: `tier15_cards.rs:1275` (garruk_back_face_overrun)
- Back face loyalty abilities list: `tier15_cards.rs:1315` (garruk_back_face_loyalty_abilities_shown_when_transformed)
- Player choice for sacrifice in -1: NOT TESTED (issue: auto-selects)
- Player choice for library search in -1: NOT TESTED (issue: auto-selects)
- Transform from non-loyalty-ability damage source (e.g., combat): NOT TESTED (implemented correctly via SBA)
- Cannot activate loyalty abilities on both faces in same turn (ruling): NOT TESTED
- Front face ability 0 creature fights back: NOT TESTED (implemented correctly)
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-01 14:49

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text (front)**: When Garruk has two or fewer loyalty counters on him, transform him. 0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Starting loyalty**: 3
**Rulings**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The -1 ability doesn't target a creature, but you must sacrifice one if you control one.
- [2011-09-22] The -3 bonus is locked in at resolution and doesn't change later.
- [2011-09-22] Only creatures you control when -3 resolves get the bonus.
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle text. Mana cost {3}{G} (Generic(3), Green). Legendary Planeswalker - Garruk. Starting loyalty 3 via `starting_loyalty()` returning `Some(3)`, with `on_resolve` adding 3 loyalty counters (line 305). Front face ability 0 (fight, ability_index 0): deals 3 damage to target creature via `NonCombatDamageDealt` event (lines 169-177), creature deals power back as loyalty loss using `state.effective_power()` (line 166) with its own `NonCombatDamageDealt` event (lines 185-189). Front face ability 1 (wolf, ability_index 1): creates 2/2 green Wolf token with `create_token_with_subtypes` and subtype ["Wolf"] (lines 197-205). Transform handled as state-triggered ability in SBA (`sba.rs` lines 247-266), checking for Garruk Relentless on battlefield with <= 2 loyalty counters and transforming. This fires from any source of loyalty loss. No loyalty is added or removed during transform (matching ruling). Back face +1 (ability_index 10): creates 1/1 black Wolf with deathtouch and subtype ["Wolf"] (lines 213-223). Back face -1 (ability_index 11): presents sacrifice choice when multiple creatures available (lines 247-257), auto-sacrifices when only one creature (lines 240-244), searches library for creature card, shuffles library (lines 54-56, 66-67 via `shuffle(&mut rng)`). Back face -3 (ability_index 12): counts creature cards in graveyard at resolution (lines 262-273), applies +X/+X and trample to controlled creatures at resolution (lines 275-293).

Missing `back_face_data()` implementation -- Garruk is the only DFC in the set that doesn't provide back face data. However, since the back face has no keywords, no continuous effects, and no triggered abilities, this has no functional impact.

### Tricky interactions checked
- Starting loyalty 3: PASS (starting_loyalty returns Some(3), on_resolve adds 3 counters)
- Legendary supertype: PASS (line 93)
- State-triggered transform at <= 2 loyalty: PASS (SBA check in sba.rs lines 247-266)
- No loyalty change on transform (ruling): PASS (SBA only flips is_transformed and name)
- Front face ability 0 targets any creature via player choice: PASS (reads from `targets` parameter at line 165)
- Front face ability 0 uses effective_power for creature damage back: PASS (line 166)
- Creature damage back to Garruk emits NonCombatDamageDealt: PASS (lines 185-189)
- Front face wolf token 2/2 green with Wolf subtype: PASS (lines 197-205)
- Back face +1 wolf 1/1 black with deathtouch and Wolf subtype: PASS (lines 213-223)
- Back face -1 sacrifice is mandatory if creature controlled (ruling): PASS (lines 230-244)
- Back face -1 does not target (ruling): PASS (no targeting in implementation)
- Back face -1 presents sacrifice choice for multiple creatures: PASS (lines 246-257)
- Back face -1 library shuffle after search: PASS (lines 54-56, 66-67)
- Back face -3 counts creatures in graveyard at resolution (ruling): PASS (lines 262-273)
- Back face -3 only affects creatures controlled at resolution (ruling): PASS (lines 275-279)
- Back face -3 applies both +X/+X and trample: PASS (lines 281-293)
- Loyalty abilities show correct costs (0/0 for front, +1/-1/-3 for back): PASS

### Test coverage
- Front face creates 2/2 Wolf token: `tier15_cards.rs:1116` (garruk_creates_wolf_token)
- Transforms at 2 or fewer loyalty (via SBA): `tier15_cards.rs:1138` (garruk_transforms_at_two_or_fewer_loyalty)
- Back face creates 1/1 black Wolf with deathtouch: `tier15_cards.rs:1161` (garruk_back_face_creates_deathtouch_wolf)
- Back face sacrifice-to-tutor: `tier15_cards.rs:1189` (garruk_back_face_sacrifice_to_tutor)
- Back face tutor presents sacrifice choice: `tier15_cards.rs:1227` (garruk_back_face_tutor_presents_sacrifice_choice)
- Back face tutor shuffles library: `tier15_cards.rs:1271` (garruk_back_face_tutor_shuffles_library)
- Back face -3 overrun effect: `tier15_cards.rs:1319` (garruk_back_face_overrun)
- Back face loyalty abilities list: `tier15_cards.rs:1359` (garruk_back_face_loyalty_abilities_shown_when_transformed)
- Transform from non-loyalty-ability damage source (e.g., combat): NOT TESTED (implemented correctly via SBA)
- Cannot activate loyalty abilities on both faces in same turn (ruling): NOT TESTED
- Front face ability 0 creature fights back: NOT TESTED (implemented correctly)

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front)**: "When Garruk has two or fewer loyalty counters on him, transform him.\n0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.\n0: Create a 2/2 green Wolf creature token."
**Oracle text (back)**: "+1: Create a 1/1 black Wolf creature token with deathtouch.\n−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.\n−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard."
**Type line**: Legendary Planeswalker — Garruk
**Status**: ISSUE

### Code issues

1. **Transform trigger is not a proper state-triggered ability (moderate)**

   The oracle text says: `"When Garruk has two or fewer loyalty counters on him, transform him."` The ruling states: "Garruk Relentless's first ability is a state-triggered ability. It triggers once Garruk has two or fewer loyalty counters on him and it can't retrigger while that ability is on the stack."

   The code in `sba.rs` lines 247-265 implements this as an immediate transformation during SBA processing:
   ```rust
   // State-triggered ability: Garruk Relentless transforms when he has 2 or fewer loyalty.
   // This is an SBA-like check that fires regardless of what caused the loyalty loss.
   ```
   The transformation happens immediately without going on the stack. In correct MTG rules, a state-triggered ability goes on the stack and can be responded to (e.g., an opponent could remove Garruk in response before the transform resolves). This is a known simplification but may produce incorrect gameplay in edge cases.

2. **Oracle text cosmetic mismatch (trivial)**

   Oracle text from Scryfall: `"When Garruk has two or fewer loyalty counters on him, transform him."`
   Code oracle_text field: `"When Garruk Relentless has two or fewer loyalty counters on him, transform Garruk Relentless."`
   Functionally equivalent but does not match Scryfall verbatim.

3. **Card not present in LLM knowledge (`mtg-player/src/llm.rs`)**

   No mention of Garruk Relentless or Garruk, the Veil-Cursed found in the LLM card knowledge section.

### Card data verification

- Mana cost: `{3}{G}` — CORRECT (Generic(3), Colored(Green))
- Card types: `[Planeswalker]` — CORRECT
- Supertypes: `[Legendary]` — CORRECT
- Subtypes: `["Garruk"]` — CORRECT
- Starting loyalty: `3` — CORRECT
- Keywords: `[]` — ACCEPTABLE (no Transform variant in Keyword enum)
- Both faces: front face via `card_data()`, back face via `loyalty_abilities()` with `is_transformed` check — CORRECT

### Behavior verification

- **Front [0] fight ability**: Deals 3 damage to target creature, creature deals power back to Garruk as loyalty counter removal. Emits NonCombatDamageDealt events for both directions. Target requirement is `Creature`. — CORRECT
- **Front [0] wolf token**: Creates 2/2 green Wolf creature token with subtype "Wolf". — CORRECT
- **Back [+1] deathtouch wolf**: Creates 1/1 black Wolf creature token with deathtouch. — CORRECT
- **Back [-1] sacrifice and tutor**: Does not target (per ruling). Presents sacrifice choice if multiple creatures. Searches library for creature card, puts in hand, shuffles. — CORRECT
- **Back [-3] overrun**: Counts creature cards in graveyard at resolution. Grants +X/+X and trample until end of turn only to creatures controlled at resolution. — CORRECT (matches ruling about snapshot behavior)
- **Loyalty costs**: 0, 0 (front); +1, -1, -3 (back) — CORRECT

### Tricky interactions checked

- **Transform at 0 loyalty**: SBA checks loyalty=0 death before the transform check (line 242 death, line 247 transform), so Garruk at 0 loyalty will die before transforming — CORRECT per rules.
- **No double activation after transform**: The engine enforces one loyalty ability per turn per planeswalker — handled at the engine level, not card level. CORRECT.
- **Sacrifice is mandatory**: The -1 ability uses `optional: false` in the ChooseTarget — CORRECT per ruling "you must sacrifice a creature if you control one."

### Test coverage

Found 7 tests in `mtg-engine/tests/tier15_cards.rs`:
- `garruk_creates_wolf_token` — front face wolf creation
- `garruk_transforms_at_two_or_fewer_loyalty` — transform via SBA
- `garruk_back_face_creates_deathtouch_wolf` — back face +1
- `garruk_back_face_sacrifice_to_tutor` — back face -1 with single creature
- `garruk_back_face_tutor_presents_sacrifice_choice` — back face -1 with multiple creatures
- `garruk_back_face_tutor_shuffles_library` — library shuffle after tutor
- `garruk_back_face_overrun` — back face -3
- `garruk_back_face_loyalty_abilities_shown_when_transformed` — correct abilities per face

Coverage is good. Missing test for front face fight ability (ability_index 0).
