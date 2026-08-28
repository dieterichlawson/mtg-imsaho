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

## Audit — 2026-08-28 19:46

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile three creature cards from your graveyard.
Flying
You may cast this card from your graveyard.
**Type line**: Creature — Zombie Horror
**P/T**: 5/6
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/skaab_ruinator.rs` matches: {1}{U}{U}, Creature, subtypes ["Zombie", "Horror"], 5/6, Flying, `AdditionalCost::ExileCreaturesFromGraveyard(3)`, `can_cast_from_graveyard() -> true`.

### Tricky interactions checked
- Ruling: "It can't be exiled to pay for itself": every path excludes the spell object — `additional_cost_plan` (offer), `exile_prompt` (choice options), `pay_exile_creatures` (auto-pick), `additional_cost_is_payable` (submit validation) all filter `o.id != spell`. PASS
- Ruling: "You must exile three creature cards no matter what zone you're casting from": the graveyard-cast path in `legal/casting.rs` calls `additional_cost_plan` and refuses when unpayable (comment there records this used to be Skaab-Ruinator-specific and is now generic per CR 601.2b). PASS
- Cast from graveyard uses the normal mana cost, not a flashback cost, and does not set `cast_with_flashback` — so it is NOT exiled on resolution and Burning Vengeance does not trigger. PASS
- Rooftop Storm's {0} alternative cost applies to the graveyard cast too (`alternative_costs` fed into the same path). PASS
- Submit-path abuse (opponent's graveyard, wrong count, duplicate ids): `additional_cost_is_payable`, added during the Altar's Reap audit. PASS

### Test coverage
- Main effect (exiles three, enters battlefield): `mtg-engine/tests/cards_complex_creatures.rs` `skaab_ruinator_exiles_creatures_from_graveyard`
- Ruling 1 (can't pay for itself — two others insufficient, three suffice): `cards_complex_creatures.rs` `skaab_ruinator_cannot_be_exiled_to_pay_for_itself`
- Cast from graveyard (on stack, no flashback flag): `cards_complex_creatures.rs` `skaab_ruinator_cast_from_graveyard`
- Not castable with too few creatures: `cards_complex_creatures.rs` `skaab_ruinator_not_castable_without_enough_creatures`
- Submit-side exile validation: `submitted_targets.rs` `an_exile_cost_cannot_be_paid_from_an_opponents_graveyard` (engine-generic)

Mutation checks:
- `can_cast_from_graveyard -> false`: `skaab_ruinator_cast_from_graveyard` and `skaab_ruinator_cannot_be_exiled_to_pay_for_itself` FAIL. Bites.
- Removing the `o.id != spell` exclusion in `additional_cost_plan`'s graveyard count: `skaab_ruinator_cannot_be_exiled_to_pay_for_itself` FAILS. Bites.
