# Audit: Abattoir Ghoul

## Reference (Scryfall/API)
- **Name:** Abattoir Ghoul
- **Mana Cost:** {3}{B}
- **Type:** Creature — Zombie
- **Oracle:** First strike. Whenever a creature dealt damage by Abattoir Ghoul this turn dies, you gain life equal to that creature's toughness.
- **P/T:** 3/2

## Implementation: `abattoir_ghoul.rs`
- **Name:** Abattoir Ghoul -- CORRECT
- **Mana Cost:** {3}{B} -- CORRECT
- **Type:** Creature — Zombie -- CORRECT
- **P/T:** 3/2 -- CORRECT
- **Keywords:** FirstStrike -- CORRECT
- **Triggered ability:** AnyCreatureDies, checks `dead_damaged_by.contains(&self_id)` -- CORRECT
- **Life gain:** Uses `dead_toughness` (last-known information) -- CORRECT
- **NonCombatDamageDealt:** N/A (life gain, not damage)

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike\nWhenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Abattoir Ghoul", cost {3}{B}, 3/2, type Creature — Zombie, keywords [FirstStrike], triggered ability on AnyCreatureDies correctly checks `dead_damaged_by` and gains life equal to `dead_toughness`. Behavior is correct.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01 14:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

### Checklist
- [x] Mana cost: {3}{B} — code: `Generic(3), Colored(Color::Black)` — matches
- [x] Card types: Creature — code: `vec![CardType::Creature]` — matches
- [x] Supertypes: none — code: `vec![]` — matches
- [x] Subtypes: Zombie — code: `vec!["Zombie".into()]` — matches
- [x] Power/toughness: 3/2 — code: `power: Some(3), toughness: Some(2)` — matches
- [x] Keywords: First strike — code: `vec![Keyword::FirstStrike]` — matches
- [x] Oracle text field: matches
- [x] Triggered ability: `TriggerKind::AnyCreatureDies` declared, `on_any_creature_dies` implemented — match
- [x] Trigger checks Ghoul is on battlefield before firing
- [x] Trigger checks `dead_damaged_by.contains(&self_id)` for "dealt damage by this creature this turn"
- [x] Life gain uses `dead_toughness` (last-known information per ruling)
- [x] Life gain emits `LifeChanged` event
- [x] LLM card knowledge present in `mtg-player/src/llm.rs`

### Tricky interactions checked
- Ghoul leaves battlefield before trigger resolves: pass (checked at line 39-41 and also by trigger infrastructure at triggers.rs:907-908)
- "This turn" damage tracking reset at end of turn: pass (engine.rs:3017 clears `damaged_by`)
- Last-known toughness includes counters/modifiers: pass (`destroy()` in destruction.rs uses `effective_toughness` at line 95-97)
- Toughness clamped to 0 minimum: pass (`.max(0)` at line 49 prevents gaining negative life)
- Life gain is mandatory (no "you may"): pass (code always gains if conditions met)

### Test coverage
- Basic life gain from damaged creature dying: `tier6_cards.rs:20` (abattoir_ghoul_gains_life_from_damaged_creature_death)
- No life gain if ghoul didn't damage the creature: `tier6_cards.rs:43` (abattoir_ghoul_no_life_if_not_damaged_by_ghoul)
- Last-known toughness with +1/+1 counters: `tier6_cards.rs:61` (abattoir_ghoul_uses_last_known_toughness_with_counters)
- Ruling (last-known toughness before death): covered by counter test above
- Ghoul leaves battlefield before trigger resolves: NOT TESTED (handled by engine infrastructure)
- "This turn" boundary (damage from previous turn doesn't count): NOT TESTED (handled by engine infrastructure)

## Audit — 2026-04-01 20:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

Card data verification (all match oracle text):
- Mana cost: Oracle `{3}{B}` vs code `Generic(3), Colored(Color::Black)` -- correct
- Card types: Oracle `Creature` vs code `vec![CardType::Creature]` -- correct
- Supertypes: Oracle has none vs code `vec![]` -- correct
- Subtypes: Oracle `Zombie` vs code `vec!["Zombie".into()]` -- correct
- Power/toughness: Oracle `3/2` vs code `power: Some(3), toughness: Some(2)` -- correct
- Keywords: Oracle `First strike` vs code `vec![Keyword::FirstStrike]` -- correct
- Oracle text field: matches verbatim
- Triggered ability: `TriggerKind::AnyCreatureDies` declared in `triggered_abilities`, `on_any_creature_dies` hook implemented -- match
- Life gain emits `LifeChanged` event (line 54) -- correct
- No targeting (trigger condition checks "a creature", not "target creature") -- correct

### Tricky interactions checked
- Ghoul removed before trigger resolves: pass -- trigger infrastructure verifies watcher is on battlefield at `triggers.rs:908` before calling `on_any_creature_dies`, and the card also checks at line 39-41
- Last-known toughness calculation: pass -- SBA captures `effective_toughness` (includes counters, effects) at `sba.rs:90-93` before moving to graveyard, matching the ruling about last-known toughness
- "This turn" damage tracking: pass -- `damaged_by` is cleared during cleanup step at `engine.rs:3017`, so damage from previous turns does not carry over
- Non-combat damage also tracked: pass -- `damaged_by.push()` called in both combat (`combat.rs:464`) and non-combat (`engine.rs:2182`, `helpers.rs:56`) damage paths, matching oracle text "dealt damage by" (not limited to combat damage)
- Life gain is mandatory (no "you may"): pass -- code always gains life when conditions are met, which matches the oracle text having no "you may" clause
- Toughness clamped to zero: pass -- `.max(0)` at line 49 prevents gaining negative life from a creature with negative toughness
- Trigger fires for any creature (not just opponent's): pass -- no controller check on the dead creature, matching oracle text "a creature" without ownership restriction

### Test coverage
- Basic life gain from damaged creature dying: `tier6_cards.rs:20` (abattoir_ghoul_gains_life_from_damaged_creature_death)
- No life gain if ghoul didn't damage the creature: `tier6_cards.rs:43` (abattoir_ghoul_no_life_if_not_damaged_by_ghoul)
- Last-known toughness with +1/+1 counters (ruling): `tier6_cards.rs:61` (abattoir_ghoul_uses_last_known_toughness_with_counters)
- Ghoul leaves battlefield before trigger resolves: NOT TESTED (handled by engine infrastructure)
- "This turn" boundary reset: NOT TESTED (handled by engine infrastructure)
- Non-combat damage sources triggering life gain: NOT TESTED
- Life gain from own creature dying (damaged by ghoul): NOT TESTED

## Audit — 2026-04-02 20:03

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

Card data verification (all compared against Scryfall oracle text fetched via `scripts/oracle_lookup.py`):
- Mana cost: Oracle `{3}{B}` vs code `Generic(3), Colored(Color::Black)` -- correct
- Card types: Oracle `Creature` vs code `vec![CardType::Creature]` -- correct
- Supertypes: Oracle has none vs code `vec![]` -- correct
- Subtypes: Oracle `Zombie` vs code `vec!["Zombie".into()]` -- correct
- Power/toughness: Oracle `3/2` vs code `power: Some(3), toughness: Some(2)` -- correct
- Keywords: Oracle `First strike` vs code `vec![Keyword::FirstStrike]` -- correct
- Oracle text field: matches verbatim
- Triggered ability: `TriggerKind::AnyCreatureDies` declared in `triggered_abilities`, `on_any_creature_dies` hook implemented -- match
- Life gain emits `LifeChanged` event (line 54) -- correct
- No targeting (trigger says "a creature", not "target creature") -- correct

### Tricky interactions checked
- Ghoul removed before trigger resolves: pass -- trigger infrastructure verifies watcher is on battlefield at `triggers.rs:908` before calling `on_any_creature_dies`; card also checks zone at line 40. Matches community ruling that Ghoul must be on the battlefield.
- Last-known toughness includes counters/modifiers: pass -- SBA captures `effective_toughness` (includes counters, continuous effects) at `sba.rs:90-93` before moving to graveyard. Matches Scryfall ruling (2011-09-22) about using last-known toughness.
- Non-combat damage also tracked: pass -- `damaged_by.push()` called in combat path (`combat.rs:464`) and non-combat damage path (`engine.rs:2182`). Oracle says "dealt damage by" without combat restriction.
- "This turn" damage tracking reset at cleanup: pass -- `damaged_by.clear()` at `engine.rs:3017` during cleanup step, so previous-turn damage does not carry over.
- Life gain is mandatory (no "you may"): pass -- code always gains life when conditions met, matching oracle text with no "you may" clause.
- Toughness clamped to zero: pass -- `.max(0)` at line 49 prevents gaining negative life from a creature with modified negative toughness.
- Trigger fires for any creature (not just opponent's): pass -- no controller restriction on the dead creature, matching oracle text "a creature" without ownership restriction.
- LLM card knowledge: pass -- present in `mtg-player/src/llm.rs` at line 137.

### Test coverage
- Basic life gain from damaged creature dying: `tier6_cards.rs:20` (abattoir_ghoul_gains_life_from_damaged_creature_death)
- No life gain if ghoul didn't damage the creature: `tier6_cards.rs:43` (abattoir_ghoul_no_life_if_not_damaged_by_ghoul)
- Last-known toughness with +1/+1 counters (ruling): `tier6_cards.rs:61` (abattoir_ghoul_uses_last_known_toughness_with_counters)
- Ghoul leaves battlefield before trigger resolves: NOT TESTED (handled by engine infrastructure at triggers.rs:908)
- "This turn" boundary reset: NOT TESTED (handled by engine infrastructure at engine.rs:3017)
- Non-combat damage sources triggering life gain: NOT TESTED
- Life gain from own creature dying (damaged by ghoul): NOT TESTED

## Audit — 2026-04-02 20:28

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01)
**Oracle text**: First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
**Type line**: Creature — Zombie
**Status**: PASS

### Code issues
No issues found.

Card data verification (all compared against Scryfall oracle text fetched via `scripts/oracle_lookup.py`):
- Mana cost: Oracle `{3}{B}` vs code `Generic(3), Colored(Color::Black)` -- correct
- Card types: Oracle `Creature` vs code `vec![CardType::Creature]` -- correct
- Supertypes: Oracle has none vs code `vec![]` -- correct
- Subtypes: Oracle `Zombie` vs code `vec!["Zombie".into()]` -- correct
- Power/toughness: Oracle `3/2` vs code `power: Some(3), toughness: Some(2)` -- correct
- Keywords: Oracle `First strike` vs code `vec![Keyword::FirstStrike]` -- correct
- Oracle text field: matches verbatim
- Triggered ability: `TriggerKind::AnyCreatureDies` declared in `triggered_abilities`, `on_any_creature_dies` hook implemented -- match
- Life gain emits `LifeChanged` event (abattoir_ghoul.rs:54) -- correct
- No targeting (trigger says "a creature", not "target creature") -- correct

### Tricky interactions checked
- Ghoul removed before trigger resolves: pass -- trigger infrastructure verifies watcher is on battlefield at `triggers.rs:908`; card also checks zone at abattoir_ghoul.rs:40. Matches community ruling that Ghoul must be on the battlefield.
- Last-known toughness includes counters/modifiers: pass -- SBA captures `effective_toughness` at `sba.rs:90-93` before zone change. Matches Scryfall ruling (2011-09-22): "You'll gain life equal to the creature's last known toughness before it died."
- Non-combat damage also tracked: pass -- `damaged_by.push()` called in combat path (`combat.rs:464`) and non-combat damage path (`engine.rs:2182`). Oracle says "dealt damage by" (not "dealt combat damage by"), so any damage source counts.
- "This turn" damage tracking reset at cleanup: pass -- `damaged_by.clear()` at `engine.rs:3017` during cleanup step.
- Life gain is mandatory (no "you may"): pass -- code always gains life when conditions met. Oracle text has no "you may" clause.
- Toughness clamped to zero minimum: pass -- `.max(0)` at abattoir_ghoul.rs:49 prevents gaining negative life.
- Trigger fires for any creature (not just opponent's): pass -- no controller restriction on dead creature in code, matching oracle text "a creature" without ownership restriction.

### Test coverage
- Basic life gain from damaged creature dying: `tier6_cards.rs:20` (abattoir_ghoul_gains_life_from_damaged_creature_death)
- No life gain if ghoul didn't damage the creature: `tier6_cards.rs:43` (abattoir_ghoul_no_life_if_not_damaged_by_ghoul)
- Last-known toughness with +1/+1 counters (ruling): `tier6_cards.rs:61` (abattoir_ghoul_uses_last_known_toughness_with_counters)
- Ghoul leaves battlefield before trigger resolves: NOT TESTED (handled by engine infrastructure at triggers.rs:908)
- "This turn" boundary reset: NOT TESTED (handled by engine infrastructure at engine.rs:3017)
- Non-combat damage sources triggering life gain: NOT TESTED
- Simultaneous death (ghoul and victim die at same time): NOT TESTED
