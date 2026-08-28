## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/182/gatstaf-shepherd-gatstaf-howler?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{G}, 2/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Gatstaf Howler — `Creature — Werewolf`, 3/3
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.

- The transform pair is the set's shared werewolf mechanism, and it turns on a
  distinction the wording makes precisely:
  - front → back, "if **no spells were cast** last turn":
    `num_spells_cast_last_turn.values().sum::<u32>() == 0` — nobody cast anything.
  - back → front, "if **a player** cast two or more spells last turn":
    `.values().any(|&count| count >= 2)` — one player individually reached two,
    not the table's total.
  Summing for the second, or requiring a single caster for the first, would both
  be wrong; neither is done.
- Both are intervening-if clauses (CR 603.4), and the card overrides
  `should_trigger` through `helpers::werewolf_should_trigger`, so with the
  condition unmet nothing reaches the stack.
- That helper also refuses the trigger for a token copy, which cannot transform
  (CR 111.7) — so it does not sit on the stack with nothing to do.
- Back-face P/T comes from `dynamic_pt`, which supplies the *base* in
  `effective_power`, leaving counters and anthems to layer on top.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`werewolf_cards.rs` (29 tests over the family), `intervening_if.rs` (the spell-count condition), `transform_dfc.rs` (CR 712 face characteristics).
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/182/gatstaf-shepherd-gatstaf-howler?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{G}, 2/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Gatstaf Howler — `Creature — Werewolf`, 3/3
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.

- The flip conditions are the shared `werewolf_should_trigger` /
  `werewolf_should_transform` helpers — one implementation of "if no spells were
  cast last turn" and "if a player cast two or more spells last turn" for the
  whole family: PASS
- CR 603.4: both are intervening-ifs, checked when the trigger would go on the
  stack *and* again on resolution: PASS
- "At the beginning of **each** upkeep" — `TriggerScope::Each`, so it fires on
  the opponent's turn too: PASS
- Each face's P/T, keywords and subtypes were checked against the fetched oracle
  face by face, not against Scryfall's merged `Keywords` list — which is the trap
  here, since that list names keywords the front face does not have: PASS
- Intimidate is on the **back** face only. Scryfall lists `Keywords: Transform,
  Intimidate` for the card as a whole, but Gatstaf Shepherd's own oracle text has
  no intimidate — only Gatstaf Howler's does, and the card data matches the
  faces rather than the merged list: PASS
- Intimidate is the printed keyword, not menace: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The flip and the back face's intimidate: `werewolf_cards.rs`, `evasion.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/182/gatstaf-shepherd-gatstaf-howler?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{G}, 2/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Gatstaf Howler — `Creature — Werewolf`, 3/3
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

One found. The card data matched; the bug was that the engine had no notion of the back face's color.

1. **Gatstaf Howler was colorless, so its own intimidate let nothing but artifacts block.** `state.rs:2230` (before the fix)
   - Oracle text (back face) says: `Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)`
   - Code did: `if let Some(cost) = self.face_data(id, registry).and_then(|d| d.cost) { ... }` — colors derived from the active face's mana cost, with the doc comment `(Color indicators are not modeled.)`
   - A transforming back face has no mana cost (CR 204.2 puts a color indicator beside its type line instead), so `colors_of` returned an empty vector for every transformed permanent. `combat.rs:499` then computes `shares_color` between the attacker's colors and the blocker's, and nothing shares a color with a colorless creature — so the Howler could only ever be blocked by artifact creatures. That is a strictly better card than the one printed, and it is this card's own ability.
   - Now `CardData::color_indicator` carries the indicator and both `colors_of` and `printed_colors_of` read it in preference to the cost. Gatstaf Howler's is green.

**Where the green came from.** The oracle cache records no back-face colors, and Scryfall, mtg.wtf and Gatherer are all unreachable from this environment. A web search this session returned, from the mtg.wtf/Scryfall/Gatherer results for this card, that "Gatstaf Howler has a color indicator showing it is green." That is an external source obtained during this audit, not memory.

**The other nineteen back faces.** Every declared back face in the set has the same gap, and I have not guessed at them: `audits/BACK_FACE_COLORS.md` lists all twenty with Gatstaf Howler marked established and the rest marked not. Each has to be established from an external source one at a time, which is what that card's own audit is for — and all nineteen are still ahead on `ISD_FULL_AUDIT_TODO.md`. Assuming "the back face is the colour of the front" would be wrong for at least Garruk Relentless, whose back face is black-green, so the shortcut is not available.

### Checked and correct

- Front: `{1}{G}`, `Creature — Human Werewolf`, 2/2, **no** keywords. Correct — Scryfall's aggregate keyword list is `Transform, Intimidate`, and intimidate belongs to the back face.
- Back: `Gatstaf Howler`, `Creature — Werewolf`, 3/3, `keywords: [Intimidate]`.
- Oracle text on both faces matches the cache (the back face's stale "transform Gatstaf Howler" was fixed in the set-wide sweep under Village Ironsmith, and is now held by `oracle_text_says_what_scryfall_says`).
- Intimidate is enforced in `can_block_attacker` as "artifact creature **or** shares a color", which is the printed restriction and not menace.
- The front face is green from its `{1}{G}` cost, and the indicator belongs to the back face only — a face with a mana cost has no indicator (CR 204.2).
- Both faces declare the upkeep trigger at the default `TriggerScope::Each`.
- The transform runs on the shared werewolf mechanism.

### Tricky interactions checked

- Green creature blocking the Howler: legal, shares a color. PASS (after fix).
- Black creature blocking the Howler: illegal. PASS.
- Artifact creature blocking the Howler: legal regardless of color — the "or" branch.
- Intimidate is lost on transforming back: PASS.
- Olivia turning the Shepherd into a Vampire, then Moonmist: PASS (`subtype.rs`).
- Token copy cannot transform: PASS (set-wide werewolf coverage).

### Test coverage

- loses intimidate on transforming back: `werewolf_cards.rs:148`
- Moonmist transforms an Olivia-bitten Shepherd, and the bite survives: `subtype.rs:366`
- intervening-if on the upkeep trigger: `intervening_if.rs`
- the Howler is green, and green blocks it while black does not: `werewolf_cards.rs` `gatstaf_howler_is_green_and_its_intimidate_lets_green_through` (NEW, mutation-checked by removing the indicator)
- the front face takes its color from its cost: `werewolf_cards.rs` `gatstaf_shepherd_is_green_from_its_mana_cost` (NEW)

