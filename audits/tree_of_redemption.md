## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/207/tree-of-redemption?utm_source=api
**Type line**: `Creature — Plant` — {3}{G}, 0/13
**Oracle text**:
```
Defender
{T}: Exchange your life total with this creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{T}: **Exchange** your life total with this creature's toughness" — both
  halves, and the exchange goes through `change_life` so LifeChanged is emitted
  like any other life change: PASS
- The ability does nothing if the Tree is no longer on the battlefield when it
  resolves — destroyed or bounced in response — because the exchange is with
  *this creature's* toughness (CR 608.2): PASS
- Defender, 0/13: PASS
- The toughness it takes on is the life total it gave away, so a subsequent
  exchange is not the printed 13: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The exchange, and being removed in response: `activated_no_stack.rs:tree_of_redemption_exchanges_on_resolution`, `token_is_not_a_card.rs:tree_destroyed_in_response_no_exchange`, `:tree_bounced_in_response_no_exchange`, `:tree_on_the_battlefield_exchanges_normally`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/207/tree-of-redemption?utm_source=api
**Type line**: `Creature — Plant` — {3}{G}, 0/13
**Oracle text**:
```
Defender
{T}: Exchange your life total with this creature's toughness.
```

**Rulings fetched**:
- [2018-03-16] If Tree of Redemption isn't on the battlefield when its activated ability resolves, the exchange can't happen and the ability will have no effect.
- [2018-03-16] When its activated ability resolves, Tree of Redemption's toughness will become your former life total and you will gain or lose an amount of life necessary so that your life total equals Tree of Redemption's former toughness. Other effects that interact with life gain or life loss will interact with this effect accordingly.
- [2018-03-16] Any toughness-modifying effects, counters, Auras, or Equipment will apply after its toughness is set to your former life total. For example, say Tree of Redemption is enchanted with Lunarch Mantle (which makes it 2/15) and your life total is 7. After the exchange, Tree of Redemption would be a 2/9 creature (its toughness became 7, which was then modified by Lunarch Mantle) and your life total would be 15.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/207/tree-of-redemption
**Oracle text**:
```
Defender
{T}: Exchange your life total with this creature's toughness.
```
**Type line**: Creature — Plant
**Mana cost**: {3}{G} — **P/T**: 0/13 — **Keywords**: Defender
**Rulings** (3, all 2018-03-16):
1. "If Tree of Redemption isn't on the battlefield when its activated ability resolves, the exchange can't happen and the ability will have no effect."
2. "When its activated ability resolves, Tree of Redemption's toughness will become your former life total and you will gain or lose an amount of life necessary so that your life total equals Tree of Redemption's former toughness. Other effects that interact with life gain or life loss will interact with this effect accordingly."
3. "Any toughness-modifying effects, counters, Auras, or Equipment will apply after its toughness is set to your former life total. For example, say Tree of Redemption is enchanted with Lunarch Mantle (which makes it 2/15) and your life total is 7. After the exchange, Tree of Redemption would be a 2/9 creature (its toughness became 7, which was then modified by Lunarch Mantle) and your life total would be 15."

**Status**: ISSUE (fixed) — the card code is correct, ruling 3 included; nothing tested that half.

### Card data
Matches the fetched text: `{3}{G}`, `card_types: [Creature]`,
`subtypes: ["Plant"]`, 0/13, `keywords: [Defender]`, oracle text verbatim in
the current "this creature's toughness" errata wording. The ability is
`ManaCost::free()` plus `requires_tap: true`, which is the whole printed cost,
and `activated_abilities` carries no zone-or-tapped guard — the comment there
explains why, and it is right.

### How it meets its rulings
- **Ruling 1**: `helpers::still_on_battlefield` guards the whole effect.
- **Ruling 2**: the life half goes through `state.change_life`, so `LifeChanged`
  is emitted exactly as everywhere else — which is what "other effects that
  interact with life gain or life loss" needs.
- **Ruling 3**: the card reads `state.effective_toughness(..)` — the toughness
  *after* modifiers — and writes `obj.toughness`, the **base**. So the modifier
  counts toward the life you gain, and then applies again on top of the new
  base. That is exactly the ruling's arithmetic.

### Code issues

No issue in `tree_of_redemption.rs`. Ruling 3 had no test.

1. **Reading the base toughness instead of the effective one broke nothing**
   (`cards_complex_creatures.rs:1067`, test added).
   - Ruling says: `its toughness became 7, which was then modified by Lunarch Mantle … and your life total would be 15`
   - Code says:
     `let current_toughness = state.effective_toughness(object_id, registry).unwrap_or(13);`
     then `obj.toughness = Some(current_life);`
   - Verified: replacing that read with
     `state.get_object(object_id).and_then(|o| o.toughness)` produced zero
     failures across the whole workspace. Every existing test had the Tree at a
     naked 0/13, where the base and the effective toughness are the same number,
     so the distinction the ruling is entirely about was invisible.
   - Added `tree_of_redemption_exchanges_the_toughness_it_actually_has`.
     Lunarch Mantle is not in this pool, so the modifier is two +1/+1 counters —
     which ruling 3 names in the same breath as Auras and Equipment — and the
     numbers are the ruling's own: a 2/15 at 7 life becomes a 2/9 at 15 life.
   - It also pins the other direction: writing `current_toughness` back into
     `obj.toughness` (double-counting the modifier) now fails three tests.

### Tricky interactions checked
- **Ruling 1** (not on the battlefield → no effect): PASS —
  `token_is_not_a_card.rs:129` and `:146`
  (`tree_destroyed_in_response_no_exchange`, `tree_bounced_in_response_no_exchange`);
  removing the guard fails both.
- **Ruling 2** (the life change is a life change): PASS structurally —
  writing `player.life` directly instead of calling `change_life` fails
  `only_change_life_writes_a_life_total`, the source guard that exists for this.
- **Ruling 3** (modifiers apply after, to the new base): PASS — new test.
- The exchange does not follow the card out of play (CR 400.7): PASS —
  `zone_change_resets_object.rs:50`, which checks the graveyard object is
  printed 0/13 again.
- The ability goes on the stack rather than resolving on activation (CR 602.2a):
  PASS — `activated_no_stack.rs:135`.
- Power is untouched: PASS — asserted in the new test.
- `{T}` cost legality and summoning sickness: the engine's, and the card is in
  the `tap_cost_legality.rs:200` list that checks it does not re-decide them.
  This matters here in a way it did not for the utility lands that once shared
  that guard — the Tree is a creature, so CR 302.6 actually applies.
- Defender: a keyword read through `has_keyword`, covered by the combat tests.
- Life becoming 0 or negative through the exchange: SBA territory, not this
  card's.
- Self-cleanup: none; this is a permanent.

### UI presentation
Ability description: "{T}: Exchange life total with Tree's toughness". The log
line names both numbers: "Tree of Redemption: exchanged life (7) with toughness
(15)".

### Test coverage
- The plain exchange (0/13, 20 life): `cards_complex_creatures.rs:1067`
  (`tree_of_redemption_swaps_life_and_toughness`).
- Ruling 3 (effective read, base written, modifier reapplied):
  (`tree_of_redemption_exchanges_the_toughness_it_actually_has`) —
  **added this audit**.
- Ruling 1 (off the battlefield): `token_is_not_a_card.rs:129`, `:146`.
- Ruling 2 (a real life change): `test_suite_guards.rs`
  (`only_change_life_writes_a_life_total`).
- Not until it resolves: `activated_no_stack.rs:135`.
- The exchange does not survive a zone change: `zone_change_resets_object.rs:50`.
- Tap-cost legality not re-decided by the card: `tap_cost_legality.rs:200`.

### Mutations run
| mutation | result |
| --- | --- |
| read `obj.toughness` (base) instead of `effective_toughness` | fails the new test (before: **nothing at all**) |
| write `current_toughness` back into `obj.toughness` | fails three tests |
| drop the `still_on_battlefield` guard | fails the two ruling-1 tests |
| write `player.life` directly instead of `change_life` | fails `only_change_life_writes_a_life_total` |

Suite after: 1459 passing, exit 0, zero warnings.

