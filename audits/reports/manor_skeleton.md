# Audit: Manor Skeleton

## Oracle (Official)
- **Name:** Manor Skeleton
- **Cost:** {1}{B}
- **Type:** Creature — Skeleton
- **Oracle:** Haste. {1}{B}: Regenerate Manor Skeleton.
- **P/T:** 1/1

## Implementation
- Name: "Manor Skeleton" -- CORRECT
- Cost: {1}{B} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Skeleton"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Haste] -- CORRECT
- Oracle text matches -- CORRECT
- Activated ability: {1}{B} regenerate -- CORRECT
- Uses regeneration_shields mechanism -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Manor Skeleton
- **Cost:** {1}{B}
- **Type:** Creature — Skeleton
- **P/T:** 1/1
- **Oracle Text:** Haste / {1}{B}: Regenerate this creature.

### Card Data Checks
- [x] Name: "Manor Skeleton" — correct
- [x] Cost: {1}{B} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Skeleton — correct
- [x] P/T: 1/1 — correct
- [x] Keywords: Haste — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"{1}{B}: Regenerate this creature."`
  - **Implementation:** `"{1}{B}: Regenerate Manor Skeleton."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Haste keyword present — correct
- [x] Activated ability costs {1}{B} — correct
- [x] Ability only available on battlefield — correct
- [x] Does not require tap — correct
- [x] Adds regeneration shield via `obj.regeneration_shields += 1` — correct

### Result: PASS

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/106/manor-skeleton?utm_source=api)
**Oracle text**: `Haste\n{1}{B}: Regenerate this creature.`
**Type line**: `Creature — Skeleton`
**Mana cost**: `{1}{B}`
**P/T**: 1/1
**Status**: PASS

### Code issues
- **Cosmetic oracle text mismatch** (not a functional issue): The oracle_text field in the implementation says `"{1}{B}: Regenerate Manor Skeleton."` while Scryfall's current oracle text says `"{1}{B}: Regenerate this creature."`. These are functionally identical since the ability is on the creature itself. Modern Oracle templating uses "this creature" but older printings used the card name. No behavioral impact.
- No other issues found.

### Tricky interactions checked (min 3)
1. **Regeneration vs destruction (SBA lethal damage)**: Verified in `sba.rs` lines 100-114 -- lethal damage routes through `try_destroy()` which checks regeneration shields before destroying. When shields > 0, the creature is tapped, damage is cleared, deathtouch flag is cleared, and it is removed from combat. Test `manor_skeleton_regeneration_saves_from_lethal` confirms this works.
2. **Regeneration vs sacrifice**: Verified in `destruction.rs` lines 61-72 -- `sacrifice()` bypasses regeneration entirely and moves the permanent to graveyard. Manor Skeleton's regeneration shield would NOT save it from sacrifice effects, which is correct per MTG rules.
3. **Regeneration vs zero toughness**: Verified in `sba.rs` lines 71-74 -- zero toughness goes directly to graveyard without calling `try_destroy`, so regeneration shields do not apply. Correct per rule 704.5f.
4. **Regeneration shields cleared at cleanup**: Verified in `engine.rs` lines 3028-3033 -- unused regeneration shields are cleared during the cleanup step. Correct per rule 514.2.
5. **Haste allows attacking the turn it enters**: Verified in `combat.rs` lines 576-577 -- `(!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))`. Manor Skeleton has `Keyword::Haste` in its keywords vec.
6. **Activated ability does not require tap**: Implementation correctly sets `requires_tap: false`. The regenerate ability can be activated even when the creature is tapped (e.g., after regeneration taps it).
7. **Multiple activations allowed**: `once_per_turn: false` is correctly set, allowing multiple regeneration shields to be stacked if mana is available.

### Test coverage
- `manor_skeleton_has_correct_stats` -- verifies P/T (1/1), Haste keyword, Skeleton subtype
- `manor_skeleton_regenerate_ability` -- verifies activation adds a regeneration shield
- `manor_skeleton_regeneration_saves_from_lethal` -- verifies regeneration prevents death from lethal damage, damage is cleared afterward
