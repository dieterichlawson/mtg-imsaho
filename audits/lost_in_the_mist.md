# Audit: Lost in the Mist

## Oracle (Official)
- **Name:** Lost in the Mist
- **Cost:** {3}{U}{U}
- **Type:** Instant
- **Oracle:** Counter target spell. Return target permanent to its owner's hand.
- **P/T:** N/A

## Implementation
- Name: "Lost in the Mist" -- CORRECT
- Cost: {3}{U}{U} -- CORRECT
- Type: Instant -- CORRECT
- Oracle text matches -- CORRECT
- Two targets: spell + permanent via TwoTargets -- CORRECT
- Counters spell (removes from stack, moves to graveyard) -- CORRECT
- Bounces permanent to hand -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lost in the Mist
- **Cost:** {3}{U}{U}
- **Type:** Instant
- **Oracle Text:** Counter target spell. Return target permanent to its owner's hand.

### Card Data Checks
- [x] Name: "Lost in the Mist" — correct
- [x] Cost: {3}{U}{U} — correct
- [x] Types: Instant — correct
- [x] Oracle text matches — correct

### Behavior Checks
- [x] Requires two targets (spell + permanent) via `TwoTargets` — correct
- [x] Target validation: checks for Stack or Battlefield zone — correct
- [x] On resolve: counters the spell (first target on stack) — correct
- [x] On resolve: bounces the permanent (second target on battlefield) — correct
- [x] Each target checked independently on resolution (if one is illegal, the other still resolves) — correct
- [x] Spell moves to graveyard after resolve — correct

### Result: PASS

## Audit — 2026-04-03 07:08
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/63/lost-in-the-mist?utm_source=api), cached 2026-04-01
**Oracle text**: Counter target spell. Return target permanent to its owner's hand.
**Type line**: Instant
**Status**: PASS

### Code issues
None. Implementation correctly matches oracle text in all respects:
- Card data: name, mana cost ({3}{U}{U}), type (Instant), oracle text all match exactly.
- Target requirement: `TwoTargets(Spell, PermanentWithFilter(Any))` correctly requires both a spell on the stack and any permanent on the battlefield.
- `is_valid_target` accepts objects on the Stack or Battlefield, covering both target types.
- `on_resolve` counter logic: checks spell is still on Stack, removes from stack via `stack.retain`, moves countered spell to graveyard via `move_spell_after_resolve`. Matches Counterspell/Dissipate pattern.
- `on_resolve` bounce logic: checks permanent is still on Battlefield, moves to Hand via `move_object`. Correct.
- Each half of resolution is independently gated by zone checks, so if one target becomes illegal the other still resolves (per ruling 2011-09-22).
- Lost in the Mist itself is moved to graveyard after resolution via `move_spell_after_resolve(object_id)` on line 71. The stack.rs post-resolution cleanup (lines 107-111) checks zone before moving, so no double-move.

### Tricky interactions checked (min 3)
1. **Partial resolution (one target illegal)**: If the spell target leaves the stack before resolution (e.g., the spell resolves first or is countered by another effect), the counter portion does nothing but the bounce still happens. If the permanent leaves the battlefield, the bounce does nothing but the counter still happens. Both halves check zones independently. Correct per ruling: "If one of Lost in the Mist's targets is illegal by the time it resolves, Lost in the Mist will still affect the remaining legal target."
2. **Fizzle (both targets illegal)**: The engine's `resolve_spell` in stack.rs checks `targets.iter().any(|t| is_target_legal(...))`. If both targets are illegal, the spell fizzles. Correct per ruling: "If both targets are illegal at this time, Lost in the Mist won't resolve."
3. **Countered spell with flashback**: If the countered spell was cast with flashback, `move_spell_after_resolve` exiles it instead of sending to graveyard. This is correct behavior for countered flashback spells.
4. **Self-bounce**: The engine generates a Cartesian product of valid targets. If the caster controls a permanent, they could target their own permanent. This is legal per the rules (the card says "target permanent" not "target permanent you don't control").

### Test coverage
- `lost_in_the_mist_counters_and_bounces` in `mtg-engine/tests/tier2_spells.rs`: Verifies the basic case -- counters a Grizzly Bears spell (moves to graveyard) and bounces a creature (moves to hand). Test passes.
- No test for partial resolution (one target becoming illegal) or fizzle (both targets illegal). These are handled by engine-level logic in stack.rs rather than card-specific code.
