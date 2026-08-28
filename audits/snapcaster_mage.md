## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/78/snapcaster-mage?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 2/1
**Oracle text**:
```
Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **instant or sorcery** card in **your** graveyard" —
  `GraveyardCardOwnedByCaster` plus the card's own type filter, and CR 109.1 now
  keeps tokens out of that enumeration engine-side: PASS
- "The flashback cost is equal to its **mana cost**" — the mana cost, not the
  mana value, so colours are preserved: PASS
- "gains flashback **until end of turn**", so it lapses if unused: PASS
- Flash, so it can be cast at instant speed to give an instant flashback in
  response: PASS
- Casting the granted flashback exiles the card (CR 702.33a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Granting flashback and the exile after: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/78/snapcaster-mage?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 2/1
**Oracle text**:
```
Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] If you cast an instant or sorcery with {X} in its mana cost this way, you still choose the value of X as part of casting the spell and pay that cost.
- [2021-03-19] If you cast a spell with flashback, you can't pay any alternative costs such as overload costs. You can pay additional costs such as kicker costs. If the spell has any mandatory additional costs, you must pay those to cast the spell with flashback.
- [2021-03-19] If a card has multiple instances of flashback, you may choose any of its flashback costs to pay.
- [2021-03-19] If a split card gains flashback, you pay only the cost of the half you're casting.
- [2021-03-19] If a card with no mana cost gains flashback, it has no flashback cost. It can't be cast this way.

**Status**: PASS


Eleven rulings, all of them the generic flashback set. The ones that bite here
are #2 (timing restrictions still apply), #4 (exiled afterward however it
leaves the stack), #9 (multiple flashback instances — pay any of them) and #11
(a card with no mana cost gains no usable flashback cost).

### Code issues
No issues found. Card data matches the fetched type line and text exactly:
{1}{U}, Creature — Human Wizard, 2/1, Flash.

- The target is declared on the `TriggeredAbilityDef` as
  `GraveyardCardOwnedByCaster`, so the engine picks it when the trigger goes on
  the stack (CR 603.3d) and re-checks it on resolution (CR 608.2b); the owner
  scoping is what "in **your** graveyard" means under CR 404.3.
- `is_valid_target` narrows that to instants and sorceries and deliberately
  does *not* reject a card that already has flashback — CR 702.33 allows
  several instances at once, and refusing meant a second Snapcaster found no
  legal target and lost its trigger to CR 603.3c.
- Ruling 11 is handled: a card with no mana cost gets no grant at all, rather
  than a free one. In this engine nothing else asks whether a card "has
  flashback", so not granting and granting-an-uncastable-cost are
  indistinguishable; the comment says so.
- The grant is pushed to `until_end_of_turn`, which is what "until end of turn"
  requires, and the cleanup step clears it.
- `on_enter_battlefield` never re-derives its controller from the source, so
  the trigger is source-independent (CR 113.7a).

I checked one thing that could have been a real bug and was not: the engine
sets `cast_with_flashback` from `is_flashback = in_graveyard &&
!can_cast_from_graveyard`, which is true on the *granted* branch as well as the
printed one — so ruling 4's exile applies to a Snapcaster-granted cast too.
That now has a test rather than only a reading.

### Tricky interactions checked
- Granted flashback alongside a printed one, both costs offered (CR 702.33):
  pass
- A card that already has flashback is still a legal target: pass
- A card with no mana cost gains no usable flashback (ruling 11): pass
- The grant expires at end of turn: pass
- A sorcery granted flashback still needs sorcery timing (ruling 2): pass
- A spell cast on a granted flashback is exiled afterward (ruling 4): pass
- "in your graveyard" excludes an opponent's: pass
- Target leaves the graveyard in response — the trigger is countered
  (CR 608.2b): pass
- {X} in the target's mana cost — the granted cost carries the {X} and funding
  is prompted like any other X cast: pass (engine path, not card-specific)

### Test coverage
- The grant happens and the engine honours it: `cards_rule_modifiers.rs:252`
- Granted and printed costs both offered:
  `flashback_multiple_instances.rs:41`
- An unaffordable granted cost does not hide a payable printed one:
  `flashback_multiple_instances.rs:71`
- No mana cost, no flashback: `flashback_multiple_instances.rs:96`
- An existing flashback never makes a card an illegal target:
  `flashback_multiple_instances.rs:126`
- The target is locked in when the trigger goes on the stack:
  `trigger_dispatch.rs:538`
- Target exiled from the graveyard in response makes the trigger fizzle:
  `trigger_target_recheck.rs:172`
- Hexproof/graveyard target filtering: `hexproof_filter.rs:499`
- **NEW** the grant is gone next turn: `flashback.rs:562`
- **NEW** a granted flashback on a sorcery still obeys sorcery timing:
  `flashback.rs:598`
- **NEW** a spell cast on a granted flashback is exiled after resolving:
  `flashback.rs:621`
- **NEW** "in your graveyard" excludes an opponent's: `flashback.rs:641`

### What the new tests are for
Every flashback test in the file tested flashback *printed* on the card. A
granted one takes a different branch at each step — the cost comes from a
`GrantFlashback` entry rather than `data.flashback_cost`, it lasts only until
end of turn, and there is no printed flashback to fall back on — so the printed
tests could not speak for it. The four new cases are the granted twins of the
printed ones, plus the durational clause, which was the only part of this
card's text with no test at all.

### A test that passed for the wrong reason
The expiry test initially advanced one turn and asserted Mulch was no longer
castable. It passed — and went on passing when I deliberately stopped the
cleanup step from clearing the grant. Two separate reasons: after one turn it
was P1's turn, when P0 could not cast a sorcery whatever the grant said, and
with no libraries stocked both players decked out over two turn cycles, ending
the game so that *no* action was legal. The test now stocks both libraries,
advances two turns back to P0's own main phase, and asserts the game still has
legal actions before concluding that this one is missing. Only then does the
mutation fail it.

