## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/76/skaab-goliath?utm_source=api
**Type line**: `Creature — Zombie Giant` — {5}{U}, 6/9
**Oracle text**:
```
As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve` as Makeshift Mauler; removed. Exiles two creature cards as an additional cost, paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/76/skaab-goliath?utm_source=api
**Type line**: `Creature — Zombie Giant` — {5}{U}, 6/9
**Oracle text**:
```
As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
```

**Status**: PASS

### Code issues
No issues found.

`ExileCreaturesFromGraveyard(2)` — the same fixed-count path, with two. 6/9
Zombie Giant with trample, both subtypes present.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — two cards leave the graveyard.

## Audit — 2026-08-28 20:15

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile two creature cards from your graveyard.
Trample
**Type line**: Creature — Zombie Giant
**P/T**: 6/9
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/skaab_goliath.rs` matches: {5}{U}, Zombie Giant, 6/9, Trample, `AdditionalCost::ExileCreaturesFromGraveyard(2)`. Data-only card on the shared exile-cost machinery (audited with Skaab Ruinator 217/249, Stitched Drake 227/249).

### Tricky interactions checked
- Exactly two, from your own graveyard, at cast time (fuel in exile before resolution): shared machinery — offer gate, prompt min=max=2, submit validation, `o.id != spell` guard. PASS
- Trample: engine-generic combat behavior; the card's declaration is now behavior-checked on the battlefield. PASS
- It's a Zombie: Rooftop Storm's {0} applies (filter tested with other Zombies), counts for Zombie-watching triggers. PASS

### Test coverage
- Cast pays (both fuel cards exiled, 6/9 arrives, has Trample): `mtg-engine/tests/cards_graveyard_interaction.rs` `exiling_creature_cards_pays_for_the_skaab` (keyword column added this audit)
- Choice prompt machinery: `auto_pick.rs` `bug_f_stitched_drake_enumerates_exile_choices` (shared)
- No rulings on Scryfall for this card.

Mutation checks:
- `ExileCreaturesFromGraveyard(2)` -> `(1)`: `exiling_creature_cards_pays_for_the_skaab` FAILS (second fuel card not exiled). Bites.
- Emptying `keywords` (Trample): same test FAILS on the new keyword assertion. Bites.
