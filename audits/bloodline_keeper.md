## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/90/bloodline-keeper-lord-of-lineage?utm_source=api
**Type line**: `Creature — Vampire` — {2}{B}{B}, 3/3
**Oracle text**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```
**Back face**: Lord of Lineage — `Creature — Vampire`, 5/5
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```

**Status**: ISSUE

### Code issues
See below.


- The back face's printed P/T came from a `dynamic_pt` override that did nothing
  but restate this card's own `back_face_data` — one derived fact written twice,
  in two places free to disagree, and every test that covered a flip asserted the
  *hook* rather than `effective_power`. CR 712.8: a transformed permanent has its
  back face's characteristics. `effective_power`/`effective_toughness` now read
  the back face directly when `is_transformed`, the nineteen echoes are deleted,
  and a guard fails the build on a new one.

### Tricky interactions checked
- "{T}: Create a 2/2 black Vampire creature token **with flying**" — colour,
  P/T, subtype and keyword all set via `create_token_with_subtypes`: PASS
- "{B}: Transform this creature. **Activate only if you control five or more
  Vampires.**" — an activation restriction, so the ability is not offered below
  five, and the count includes the tokens it made: PASS
- The token ability is on the front face and the lord ability on the back: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Token creation and the transform gate: `cards_transforming_permanents.rs`, `subtype.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/90/bloodline-keeper-lord-of-lineage?utm_source=api
**Type line**: `Creature — Vampire` — {2}{B}{B}, 3/3
**Oracle text**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```
**Back face**: Lord of Lineage — `Creature — Vampire`, 5/5
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

**A token copy could transform, which CR 111.7 forbids.**

- Oracle text says: `{B}: Transform this creature. Activate only if you control five or more Vampires.`
- Code did:
  ```rust
  1 => {
      // Transform into Lord of Lineage.
      if let Some(obj) = state.get_object_mut(object_id) {
          obj.is_transformed = true;
          obj.name = "Lord of Lineage".into();
      }
      ...
  }
  ```

CR 111.7: a token that is a copy of a double-faced card is not itself
double-faced — it has only the copied face — so it cannot transform. The engine
knows this and enforces it in `helpers::apply_transform`, which refuses for
`o.is_token` and carries a comment citing the rule. Setting `is_transformed` and
the name by hand went straight around it.

Reachable: **Cackling Counterpart** ("Create a token that's a copy of target
creature you control") makes exactly such a token of Bloodline Keeper, and the
token was also *offered* the transform ability in the first place.

Fixed on both sides — the ability is not offered to a token, and the flip goes
through `apply_transform`, which is where "what transforming means" belongs. The
log now reads the resulting name from the face rather than hardcoding "Lord of
Lineage".

### Rulings checked

The only published ruling is a link to a mechanics article, with no rules
content.

### Tricky interactions checked

- **"Activate only if you control five or more Vampires"** — counted over the
  controller's battlefield by subtype, and the Keeper counts itself, which is
  correct: it is a Vampire. So four other Vampires suffice. PASS.
- **The count uses `has_subtype`**, which sees a runtime-granted Vampire type as
  well as a printed one — right here, because Olivia Voldaren's grant genuinely
  makes a creature a Vampire for this purpose. (This is the opposite of the copy
  case in Evil Twin, where the grant must *not* carry.) PASS.
- **Both faces have `{T}`: create a 2/2 black Vampire with flying** — offered
  regardless of `is_transformed`, and only the transform ability is front-face
  only. PASS.
- **The token is a 2/2 black Vampire with flying**, created through
  `create_token_with_subtypes` so it carries the Vampire type — which matters,
  since each token raises the count toward five and is pumped by the back face's
  anthem. PASS.
- **"Other Vampire creatures you control get +2/+2"** — `GlobalOther`, so Lord of
  Lineage does not pump itself. PASS.
- **The transform has no once-per-turn or sorcery-speed restriction** — matching
  the oracle text, which gives neither. PASS.
- **`should_transform` returns false** — this is not a Werewolf; it flips only
  through its own activated ability. PASS.

### Test coverage

- a token copy is neither offered the transform nor able to perform it: `copy_effects.rs::a_token_copy_of_bloodline_keeper_cannot_transform` (new, mutation-checked).
- transforms at five Vampires and gains the anthem: `cards_transforming_permanents.rs:540`.
- back-face anthem and token creation: `cards_transforming_permanents.rs`.


## Follow-up — 2026-08-28 — back-face colour indicator established

**Colour source**: external, fetched this session — a web search over the Scryfall and mtg.wtf results for the card returned that "Lord of Lineage has a color indicator showing that it is black". Not from memory.
**Status**: ISSUE (fixed)

### Code issue
- CR 204.2: a back face has no mana cost, so its colour comes from the printed
  colour indicator. `back_face_data` declared none, so a transformed permanent
  was **colourless** — it dodged protection, intimidate, and every
  "non-colour" filter in the set. This was the class opened under Gatstaf
  Shepherd; this card's full audit predated the colour-indicator sweep, and
  `audits/BACK_FACE_COLORS.md` carried it as "not yet established" until now.
- Fixed: `color_indicator: vec![Color::Black]` on the back face (Lord of Lineage is black).

### Test coverage
- The colour is pinned, with the other nineteen back faces, by
  `card_data_invariants.rs::every_back_face_declares_the_colour_its_indicator_prints`,
  whose table also fails the build on any declared back face it does not name.
  Mutation-checked by emptying Ironfang's indicator, which fails the sweep by
  name.
