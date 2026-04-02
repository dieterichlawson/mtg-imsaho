# ISD Audit Progress

## Status
ALL 89 ISD CARDS RE-AUDITED. 72 PASS, 16 ISSUE, 1 ENGINE LIMITATION.
Now fixing ISSUE cards.

## Cards with ISSUE status (17 cards to fix)

### High priority (gameplay bugs)
1. **Evil Twin**: 6 issues — mandatory copy (should be optional "you may"), auto-selected target, card_types not copied, subtypes merged, destroy ability target filter wrong, is_evil_twin not copiable
2. **Burning Vengeance**: Only triggers on flashback (cast_with_flashback), not all graveyard casts. Also triggers.rs SpellCast filter blocks non-instant/sorcery
3. **Rooftop Storm**: Alternative cost implemented as unconditional free — should be optional ("you may"), bypasses additional costs
4. **Essence of the Wild**: Replacement effect modeled as triggered ability, incomplete copy
5. **Heretic's Punishment**: Wrong order (damage before mill), missing damaged_by, outdated oracle text
6. **Divine Reckoning**: Auto-selects highest toughness creature instead of player choice
7. **Memory's Journey**: Player not explicitly targeted (hexproof bypass), 0-card opponent mode shuffles wrong library
8. **Corpse Lunge**: Missing damaged_by tracking, engine auto-selects exile target

### Medium priority (missing player choice / incorrect behavior)
9. ~~**Garruk Relentless**: State-triggered transform as immediate SBA instead of stack~~ — RECLASSIFIED as engine limitation (SBA approach is functionally correct; engine lacks state-triggered ability stack support)
10. **Caravan Vigil**: Morbid "you may" auto-selects instead of presenting choice
11. **Curiosity**: "You may" draw is forced, not optional
12. **Claustrophobia**: ETB tap during resolution instead of as triggered ability on stack

### Low priority (text-only / minor)
13. **Civilized Scholar**: Oracle text field wording (text-only fix)
14. **Ludevic's Test Subject**: Manual transform instead of helpers::apply_transform()
15. **Fiend Hunter**: LTB doesn't reset controller to owner
16. **Elder Cathar**: Human subtype check only uses registry, misses tokens
17. **Ghoulraiser**: Filters "Zombie creature card" but oracle says "Zombie card"

## Systemic Issues Found
- `crate::combat::fight` emits CombatDamageDealt for fight damage (should be NonCombat)
- `triggers.rs` SpellCast filter blocks non-instant/sorcery from SpellCast events
- Registry-only Human subtype check pattern (Butcher's Cleaver, Bonds of Faith, Elder Cathar)
- Engine auto-selects sacrifice targets instead of player choice (multiple cards affected)

## Completed Audits (new PASS) — 72 cards
All other ISD cards passed their re-audit.
