## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/25/nevermore?utm_source=api
**Type line**: `Enchantment` — {1}{W}{W}
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```

**Status**: ISSUE

### Code issues
See below.

- Oracle text says: `As this enchantment enters, choose a nonland card name.`
- Code did: declared `TriggerKind::EntersBattlefield` with
  `has_etb_handler() -> true`, so the choice was a triggered ability that went
  on the stack.
- CR 614.12: "**As** [this] enters, choose ..." is a replacement effect applied
  as the permanent enters, not a trigger. Measured before the fix: Nevermore
  resolved onto the battlefield, `awaiting_action` was `false` — **no name
  chosen** — and one trigger sat on the stack. That is a priority window in
  which Nevermore is on the battlefield naming nothing, long enough for an
  opponent to cast the very card it was about to name. For a card whose entire
  function is to pre-empt one card, that window is the card.
- Fixed: new `CardBehavior::chooses_as_it_enters` hook, called from the entering
  path in `move_object` beside the existing copy-guard arming — the one moment
  before any state-based action or priority. Nevermore declares it and no longer
  declares an `EntersBattlefield` trigger. After the fix the same probe shows the
  choice pending at entry with an empty stack.
- The hook is general, not a Nevermore special case: any card whose text begins
  "As this ... enters, choose" belongs on it. Evil Twin stays on the ETB path
  because its consequence is different — it needs its printed 0/0 to survive
  until the copy applies, which the engine already bridges with
  `entering_copy_source` armed at the same point.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`enters_tapped_replacement.rs::a_name_chosen_as_a_permanent_enters_is_chosen_before_anyone_has_priority` — **added by this audit**, asserting both halves: the choice is pending at entry, and nothing reaches the stack.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/25/nevermore?utm_source=api
**Type line**: `Enchantment` — {1}{W}{W}
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As this enchantment enters**, choose a nonland card name" is CR 614.12 — a
  choice made *as* it enters, not a triggered ability. It is declared through
  `chooses_as_it_enters`, so the engine asks during the entry event rather than
  afterwards: PASS
- Ruling: "**No one can cast spells or activate abilities** between the time a
  card is named and the time that Nevermore's ability starts to work" — a
  consequence of it being an as-enters choice rather than a trigger: PASS
- Ruling: "Spells with the chosen name that somehow happen to **already be on
  the stack** when Nevermore enters are **not affected**" — the restriction is
  on *casting*, checked when a spell would be cast: PASS
- Ruling: "Although the named card can't be cast, it **can still be put onto the
  battlefield** by a spell or ability" — Unburial Rites returning a named
  creature still works: PASS
- "a **nonland** card name": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The as-enters choice and the cast restriction: `cards_rule_modifiers.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/25/nevermore?utm_source=api
**Type line**: `Enchantment` — {1}{W}{W}
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```

**Rulings fetched**:
- [2021-03-19] If you name one of the faces of a modal double-faced card, players may still play the face you didn't name. (Players can never cast the back face of a transforming double-faced card)
- [2011-09-22] No one can cast spells or activate abilities between the time a card is named and the time that Nevermore's ability starts to work.
- [2011-09-22] Spells with the chosen name that somehow happen to already be on the stack when Nevermore enters are not affected by Nevermore's ability.
- [2011-09-22] Although the named card can't be cast, it can still be put onto the battlefield by a spell or ability (if it's a permanent card).
- [2011-09-22] You can name either half of a split card, but not both. If you do so, that half (and both halves, if the split card has fuse) can't be cast. The other half is unaffected.
- [2011-09-22] The named card can be cast again once Nevermore leaves the battlefield.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/25/nevermore
**Oracle text**:
```
As this enchantment enters, choose a nonland card name.
Spells with the chosen name can't be cast.
```
**Type line**: Enchantment
**Mana cost**: {1}{W}{W}
**Rulings** (6):
1. (2021-03-19) Naming one face of a modal double-faced card leaves the other playable.
2. (2011-09-22) "No one can cast spells or activate abilities between the time a card is named and the time that Nevermore's ability starts to work."
3. (2011-09-22) "Spells with the chosen name that somehow happen to already be on the stack when Nevermore enters are not affected by Nevermore's ability."
4. (2011-09-22) "Although the named card can't be cast, it can still be put onto the battlefield by a spell or ability (if it's a permanent card)."
5. (2011-09-22) Split cards — name either half, not both.
6. (2011-09-22) "The named card can be cast again once Nevermore leaves the battlefield."

**Status**: ISSUE (fixed) — the card code is correct; two rulings had no test.

### Card data
Matches the fetched text: `{1}{W}{W}`, `card_types: [Enchantment]`, oracle text
verbatim in the current "As this enchantment enters" errata wording, no
subtypes, no P/T, no keywords.

`chooses_as_it_enters()` returns true and `has_etb_handler()` stays false, which
is the right distinction: CR 614.12 makes "as this enters, choose" a
replacement-style choice made as the permanent enters, not a triggered ability
— so no `EntersBattlefield` trigger is created, and **ruling 2** ("no one can
cast spells or activate abilities between the time a card is named and the time
Nevermore's ability starts to work") falls out of there being no stack entry to
respond to.

### Code issues

No issue in `nevermore.rs`. Two rulings had no test.

1. **Ruling 6 — the ban lifts when Nevermore leaves**
   (`cards_rule_modifiers.rs`, test added).
   - Ruling says: `The named card can be cast again once Nevermore leaves the battlefield.`
   - Added `nevermores_ban_lifts_when_it_leaves_the_battlefield`: the Bolt is
     uncastable, Nevermore goes to the graveyard, the Bolt is castable.
   - **Worth stating plainly**: no single-line mutation can break this test,
     because two independent guards enforce it — `state.global_effects` skips
     any source not on the battlefield (`state.rs:1349`), and `move_object`
     clears `instance_continuous_effects` on the way out (`state.rs:810`).
     Removing either alone leaves the other in force and the suite green.
     Removing **both at once** fails the new test, which is what says it is
     about the rule rather than about one of its two implementations. I
     recorded this rather than claim a mutation result the test does not have.

2. **Ruling 4 — a named card can still be put onto the battlefield**
   (same file, test added).
   - Ruling says: `Although the named card can't be cast, it can still be put onto the battlefield by a spell or ability (if it's a permanent card).`
   - "Can't be cast" is a restriction on casting and nothing else, which is why
     the check lives only in `legal/casting.rs` — both the from-hand path
     (line 28) and the flashback path (line 258).
   - Added `a_named_card_can_still_be_put_onto_the_battlefield`: Nevermore names
     Walking Corpse, a copy in hand is confirmed uncastable (so the test cannot
     pass for a Nevermore that banned nothing), and Unburial Rites returns
     another copy from the graveyard anyway.
   - Verified: teaching Unburial Rites to refuse a named creature fails it.

### Tricky interactions checked
- The controller is asked, from the nonland card pool, rather than the
  implementation peeking at a hand: PASS — `auto_pick.rs:344`
  (`nevermore_asks_for_a_name_instead_of_reading_the_opponents_hand`), which
  also checks lands are filtered out and that no name is locked in until the
  choice is answered.
- The named spell can't be cast from hand, by **either** player, and everything
  else still can: PASS — `cards_rule_modifiers.rs:153`
  (`nevermore_bans_the_name_it_chose_and_nothing_else`). Disabling the
  from-hand check fails it.
- Flashback is a way of casting, so the ban stops it too (CR 702.33a): PASS —
  `flashback.rs:409` (`nevermore_stops_the_card_it_names_from_being_flashed_back`).
- **Ruling 3** (a spell already on the stack is unaffected): structural — the
  ban is consulted only when enumerating castable cards in hand and graveyard,
  so a spell that is already on the stack is never re-examined. Nothing to
  test; recorded.
- **Ruling 2** (no window between naming and the ban taking effect): structural
  — `chooses_as_it_enters` means no trigger goes on the stack. Recorded.
- **Rulings 1 and 5** (modal DFCs and split cards): neither exists in this
  pool. Innistrad's double-faced cards are transforming DFCs, whose back faces
  can never be cast at all, which ruling 1 says in its own parenthesis. Nothing
  to test.
- "Nonland card name": the option list filters on the registry's card types.
  PASS — asserted in the auto_pick test.
- Nevermore names a **card**, not a permanent, so it reads the registry by name
  with no game object involved and the characteristics layer does not apply.
  The card's comment says so; correct.
- Self-cleanup: none; this is a permanent.

### UI presentation
The prompt reads "Nevermore: choose a nonland card name (spells with that name
can't be cast)" and offers the sorted pool of nonland card names.

### Test coverage
- The naming prompt, its options, and that the chosen name is the one banned:
  `auto_pick.rs:344`.
- The ban applies from hand, to either player, and only to that name:
  `cards_rule_modifiers.rs:153`.
- The ban applies to flashback casts: `flashback.rs:409`.
- Ruling 6 (the ban lifts): `nevermores_ban_lifts_when_it_leaves_the_battlefield`
  — **added this audit**.
- Ruling 4 (put onto the battlefield anyway):
  `a_named_card_can_still_be_put_onto_the_battlefield` — **added this audit**.
- Rulings 1, 2, 3, 5: NOT TESTED — structural or absent from this pool; see above.

### Mutations run
| mutation | result |
| --- | --- |
| Unburial Rites refuses a named creature | fails the new ruling-4 test (before: **nothing at all**) |
| `global_effects` stops skipping non-battlefield sources | **nothing** — `move_object` still clears the effects |
| `move_object` stops clearing `instance_continuous_effects` | **nothing** — `global_effects` still skips the zone |
| both of the above at once | fails the new ruling-6 test |
| disable the from-hand ban check | fails `nevermore_bans_the_name_it_chose_and_nothing_else` |

Suite after: 1464 passing, exit 0, zero warnings.

