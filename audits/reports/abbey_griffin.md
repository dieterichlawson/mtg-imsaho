# Audit: Abbey Griffin

## Reference (Scryfall/API)
- **Name:** Abbey Griffin
- **Mana Cost:** {3}{W}
- **Type:** Creature — Griffin
- **Oracle:** Flying, vigilance
- **P/T:** 2/2

## Implementation: `abbey_griffin.rs`
- **Name:** Abbey Griffin -- CORRECT
- **Mana Cost:** {3}{W} -- CORRECT
- **Type:** Creature — Griffin -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** Flying, Vigilance -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Abbey Griffin", cost {3}{W}, 2/2, type Creature — Griffin, keywords [Flying, Vigilance]. Vanilla creature with keywords only, no behavior needed beyond card_data.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01 14:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- No complex interactions apply (keyword-only creature): PASS

### Test coverage
- Flying blocks ground creatures: `keywords.rs` (flying test - ground creature cannot block)
- Flying allows flier-on-flier blocking: `keywords.rs` (flier can block another flier)
- Vigilance does not tap when attacking: `keywords.rs` (vigilance test)
- Card has correct keywords: `innistrad_cards.rs:abbey_griffin_has_flying_and_vigilance`
- LLM card knowledge entry: present in `mtg-player/src/llm.rs`

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying keyword blocks correctly (ground creatures cannot block, flyers can block flyers): PASS
- Vigilance keyword prevents tapping on attack: PASS
- Summoning sickness applies normally (no haste): PASS
- No triggered/activated abilities to interact with the stack: PASS
- Keywords handled by engine (not card code) — no anti-patterns possible: PASS

### Test coverage
- Flying blocks ground creatures: `mtg-engine/tests/keywords.rs:flying_creature_cannot_be_blocked_by_ground_creature`
- Flyer-on-flyer blocking: `mtg-engine/tests/keywords.rs:flyer_can_block_flyer`
- Vigilance does not tap on attack: `mtg-engine/tests/keywords.rs:vigilance_does_not_tap_on_attack`
- Card has correct keywords: `mtg-engine/tests/innistrad_cards.rs:abbey_griffin_has_flying_and_vigilance`
- LLM card knowledge entry: present in `mtg-player/src/llm.rs:51`

## Audit — 2026-04-02 20:03

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying keyword: engine handles blocking restrictions (ground creatures cannot block flyers) — verified in code and tests: PASS
- Vigilance keyword: engine prevents tapping on attack declaration — verified in code and tests: PASS
- Summoning sickness: no haste keyword, so creature cannot attack or use tap abilities the turn it enters — handled by engine: PASS
- No triggered/activated abilities: no stack interactions, no responses possible beyond the spell itself on the stack: PASS
- No anti-patterns possible: card is keyword-only, no `on_resolve`, no `move_object`, no targeting, no token creation — all behavior is engine-level: PASS

### Test coverage
- Flying blocks ground creatures: `mtg-engine/tests/keywords.rs:flying_creature_cannot_be_blocked_by_ground_creature` (line 22)
- Flyer-on-flyer blocking: `mtg-engine/tests/keywords.rs:flyer_can_block_flyer` (line 38)
- Vigilance does not tap on attack: `mtg-engine/tests/keywords.rs:vigilance_does_not_tap_on_attack` (line 78)
- Card has correct keywords: `mtg-engine/tests/innistrad_cards.rs:abbey_griffin_has_flying_and_vigilance` (line 69)
- LLM card knowledge entry: present in `mtg-player/src/llm.rs:51`

## Audit — 2026-04-02 20:28

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying keyword enforced in combat (ground creatures cannot block): verified in `combat.rs:can_block_attacker` — PASS
- Vigilance keyword prevents tapping on attack: verified in `combat.rs:declare_attackers` and `engine.rs` forced attacker handling — PASS
- Summoning sickness: no haste keyword, creature cannot attack the turn it enters — handled by engine — PASS
- No extra behavior methods implemented (only `card_data()`) — correct for keyword-only creature — PASS

### Test coverage
- Flying blocks ground creatures: `mtg-engine/tests/keywords.rs:flying_creature_cannot_be_blocked_by_ground_creature` (line 22)
- Flyer-on-flyer blocking: `mtg-engine/tests/keywords.rs:flyer_can_block_flyer` (line 38)
- Vigilance does not tap on attack: `mtg-engine/tests/keywords.rs:vigilance_does_not_tap_on_attack` (line 78)
- Card has correct keywords: `mtg-engine/tests/innistrad_cards.rs:abbey_griffin_has_flying_and_vigilance` (line 69)
