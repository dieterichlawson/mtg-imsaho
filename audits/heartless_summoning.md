# Audit: Heartless Summoning

## Oracle Reference (Scryfall)
- Cost: {1}{B}
- Type: Enchantment
- Oracle: "Creature spells you cast cost {2} less to cast.
  Creatures you control get -1/-1."

## Implementation: heartless_summoning.rs

## Issues Found

No issues found. Name, cost ({1}{B}), type (Enchantment), oracle text, and both continuous effects (ReduceCost for creature spells by 2, ModifyPT -1/-1 for creatures you control) all match correctly.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
```

### Findings
- Name, cost ({1}{B}), type (Enchantment) all match.
- Oracle text in code matches Scryfall oracle.
- ContinuousEffect::ReduceCost with reduction 2 for CreatureSpells -- correct.
- ContinuousEffect::ModifyPT with -1/-1 for Global(CreatureFilter::You) -- correct.

### Verdict: PASS

---

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Creature spells you cast cost {2} less to cast.\nCreatures you control get -1/-1.
**Type line**: Enchantment
**Status**: PASS

### Code issues
None found. All card data fields match oracle exactly:
- Name: "Heartless Summoning"
- Cost: {1}{B} (Generic(1) + Colored(Black))
- Type: Enchantment (no supertypes, subtypes, P/T, keywords)
- Oracle text matches Scryfall verbatim
- Two continuous effects correctly implement both abilities:
  - `ReduceCost { reduction: 2, filter: SpellFilter::CreatureSpells }` — reduces creature spell costs by {2}
  - `ModifyPT { power: -1, toughness: -1, scope: EffectScope::Global(CreatureFilter::You) }` — gives -1/-1 to your creatures

Engine handling verified:
- `effective_spell_cost()` only applies ReduceCost from permanents the caster controls (line 81 check: `obj.controller != caster`)
- Reduction only applies to `ManaSymbol::Generic`, leaving colored requirements intact (matches ruling)
- `effect_applies_to` correctly scopes `CreatureFilter::You` to `creature.controller == source_controller`

### Tricky interactions checked (min 3)
1. **Only reduces generic mana**: Engine iterates mana symbols and only reduces Generic portion, colored mana is preserved. A creature costing {3}{G}{G}{G} becomes {1}{G}{G}{G}, not {G}{G}{G}. Matches Scryfall ruling: "can only reduce the generic mana portion."
2. **Does not reduce non-creature spells**: `SpellFilter::CreatureSpells` checks `card_types.contains(&CardType::Creature)`. Test `heartless_summoning_no_reduce_noncreature` confirms Lightning Bolt is unaffected.
3. **-1/-1 is a continuous effect, not counters**: Uses `ModifyPT` (a static continuous effect), not counters. This correctly interacts with undying — a creature with undying returns with a +1/+1 counter, and the continuous -1/-1 does not remove that counter.
4. **Applies only to controller's creatures**: `EffectScope::Global(CreatureFilter::You)` resolved through `matches_filter` checks `creature.controller == source_controller`. Opponent creatures are unaffected.

### Test coverage
- `heartless_summoning_gives_minus_one` — verifies 6/6 Kindercatch becomes 5/5
- `heartless_summoning_reduces_creature_cost` — verifies Kindercatch ({3}{G}{G}{G}) castable with 1 colorless + 3 green
- `heartless_summoning_no_reduce_noncreature` — verifies Lightning Bolt cost is not reduced
- Gap: no test for 1-toughness creature dying to -1/-1 via state-based actions (minor, SBA handling is tested elsewhere)
