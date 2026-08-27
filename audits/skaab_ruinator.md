## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/77/skaab-ruinator?utm_source=api
**Type line**: `Creature — Zombie Horror` — {1}{U}{U}, 5/6
**Oracle text**:
```
As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve`; removed. Two further clauses checked:
- "You may cast this card from your graveyard" is a *permission*, correctly
  expressed by the cast being offered from the graveyard rather than by a
  yes/no prompt.
- The additional cost exiles three creature cards, paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/77/skaab-ruinator?utm_source=api
**Type line**: `Creature — Zombie Horror` — {1}{U}{U}, 5/6
**Oracle text**:
```
As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
```

**Status**: PASS

### Code issues
No issues found.

Ruling 1: "Skaab Ruinator is on the stack when you pay its costs. It can't be
exiled to pay for itself." Every site that enumerates the graveyard for
`ExileCreaturesFromGraveyard` — the payability check in `additional_cost_plan`,
the player prompt in `exile_prompt`, and the payment in `pay_exile_creatures` —
carries `o.id != spell`. So casting from the graveyard with the Ruinator plus
two other creature cards is not a legal cast.

Ruling 2: "You must exile three creature cards from your graveyard no matter
what zone you're casting Skaab Ruinator from." The cost is keyed on `card_id`,
not on zone, and `legal/casting.rs:350` runs `additional_cost_plan` on the
graveyard-cast path too — that check used to be inline and only covered casts
from hand.

`can_cast_from_graveyard` gives the permission; the cost paid is the printed
{1}{U}{U}, not a flashback cost, and the card is not exiled on resolution.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_complex_creatures.rs::skaab_ruinator_cannot_be_exiled_to_pay_for_itself` (new — two other creature cards is not enough, three is), `::skaab_ruinator_cast_from_graveyard`, `::skaab_ruinator_exiles_creatures_from_graveyard`, `spell_costs.rs` for the cost-reduction paths.
