## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/44/back-from-the-brink?utm_source=api
**Oracle text**:
```
Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.`)
- Code did: filtered the graveyard by creature-ness alone, with no card/token distinction.
- CR 109.1: a token is not a card. CR 111.7 removes a token from a graveyard as
  a state-based action, so between the moment it dies and the next SBA check it
  really is sitting there — the same window a dies-trigger sees. Measured
  directly on Boneyard Wurm: 2/2 with one creature card and one just-died token
  in the yard, 1/1 the instant SBAs ran. The oracle's answer is 1/1 throughout.
- Fixed: the graveyard filter now goes through `state.is_card`.

### How this was found
A sweep for cards whose oracle says "cards in a graveyard" against code that
never distinguishes tokens. Thirteen cards matched; five already guarded
(Gnaw to the Bone, Moorland Haunt, Past in Flames, Runechanter's Pike,
Splinterfright) and eight did not.

Splinterfright and Boneyard Wurm are the instructive pair — near-identical
text, adjacent in the set. `token_is_not_a_card.rs::cda_does_not_count_tokens_in_graveyard`
covered Splinterfright, which is why Splinterfright alone had the guard.

### Test coverage
`token_is_not_a_card.rs::a_token_in_a_graveyard_is_not_a_creature_card` —
**added by this audit**, covers Boneyard Wurm and Splinterfright together and
fails against the unfixed code.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/44/back-from-the-brink?utm_source=api
**Type line**: `Enchantment` — {4}{U}{U}
**Oracle text**:
```
Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Although you're paying the card's mana cost, you **aren't casting**
  that card." No cast triggers fire, and the ability is an activated ability
  rather than a cast: PASS
- Ruling: "If the exiled creature card has **{X}** in its mana cost, X is
  considered to be zero." The cost is taken `.without_x()`, so the engine does
  not put the player through an X-funding prompt for a value that can only be 0
  (CR 107.3e): PASS
- Ruling: "If you exile a **double-faced** creature card this way, you'll pay
  the mana cost of the **front face**. The token will be a copy of the front face
  and **it won't be able to transform**." The cost comes from `face_data` (the
  front face for an untransformed card), and `apply_transform` refuses outright
  to flip a token — CR 111.7, a token copy of one face is not a double-faced
  card: PASS
- Ruling: "Any 'enters' abilities of the creature will trigger when the token
  enters": PASS
- "Exile a creature card from your graveyard **and pay its mana cost**" — both
  are costs, paid on activation, so the card is in exile while the ability is on
  the stack: PASS
- CR 109.1: "a creature **card**", so a token in the graveyard is not one — and
  the guard for that was defeated by `&&` binding tighter than `||`, now fixed:
  PASS
- "Activate only as a sorcery": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The token copy, the X cost and the no-transform rule: `activated_no_stack.rs:back_from_the_brink_makes_its_token_on_resolution`, `cards_transforming_permanents.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/44/back-from-the-brink?utm_source=api
**Type line**: `Enchantment` — {4}{U}{U}
**Oracle text**:
```
Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
```

**Rulings fetched**:
- [2011-09-22] Although you're paying the card's mana cost, you aren't casting that card. Abilities that reduce the cost to cast a creature spell won't apply, and additional costs to cast that creature can't be paid. Alternative costs that affect what it costs to cast a creature spell, such as evoke, can't.
- [2011-09-22] Any "enters" abilities of the creature will trigger when the token enters. Any "as [this creature] enters" or "[this creature] enters with" abilities of the creature will also work.
- [2011-09-22] If you exile a double-faced creature card this way, you'll pay the mana cost of the front face. The token will be a copy of the front face and it won't be able to transform.
- [2011-09-22] If the exiled creature card has {X} in its mana cost, X is considered to be zero.

**Status**: PASS (behaviour correct; three rulings were untested and now are)

### Code issues

**The card's behaviour is correct.** All four rulings are honoured, and three
of them were untested; the only code change is a cosmetic one.

**Cosmetic: a parameter named `_registry` that is used.**
`pay_activation_cost` took `_registry: &CardRegistry` — the underscore says
"unused" — and then wrote `state.move_object(creature_id, Zone::Exile,
_registry)`. Renamed, and the "everything before the colon is cost" note moved
to a doc comment on the method where it belongs.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{4}{U}{U}` | `Generic(4), Blue, Blue` OK |
| type | `Enchantment` | `[CardType::Enchantment]`, no P/T OK |
| oracle text | verbatim match | OK |
| ability | exile a creature card from your graveyard + pay its mana cost, sorcery speed | one `ActivatedAbilityDef` per eligible card, cost taken from that card, `sorcery_speed_only: true` OK |

### Tricky interactions checked

- **Ruling: "If the exiled creature card has {X} in its mana cost, X is
  considered to be zero."** **Pass** — the cost is built with `.without_x()`,
  so the engine does not put the player through an X-funding prompt for a value
  that can only be 0 (CR 107.3e). Tested in `trigger_snapshots.rs`, and
  removing `.without_x()` fails it.
- **Ruling: "If you exile a double-faced creature card this way, you'll pay the
  mana cost of the front face. The token will be a copy of the front face and
  it won't be able to transform."** **Pass** — `face_data` on a graveyard card
  gives the front face, because the zone change clears `is_transformed`
  (CR 712.8a); and `apply_transform` refuses tokens outright, with a comment
  citing this very ruling. Was untested for this card; now is, in all three
  parts.
- **Ruling: "Any 'enters' abilities of the creature will trigger when the token
  enters."** **Pass** — token creation emits `EnteredBattlefield` like any
  other entry. Was untested; now tested with Ghoulraiser, whose enters trigger
  returns a Zombie card from the graveyard.
- **Ruling: "Although you're paying the card's mana cost, you aren't casting
  that card. Abilities that reduce the cost to cast a creature spell won't
  apply... Alternative costs... can't."** **Pass**, and structurally rather than
  by accident: alternative and reduced costs are generated in
  `engine/legal/casting.rs`, which the activated-ability path does not go
  through. Was untested; now tested against Rooftop Storm, whose "{0} rather
  than pay the mana cost for Zombie creature spells you cast" leaves a
  graveyard Zombie's ability at its full {1}{B}.
- **"a creature card"** — a token in a graveyard is not one (CR 109.1), and it
  sits there until the next state-based-action check so it can be seen. **Pass**
  — the `is_card` guard is there, enforced by a source scanner
  (`test_suite_guards.rs::a_card_enumerating_a_graveyard_excludes_tokens`,
  whose doc records that this card's guard was once defeated by `&&` binding
  tighter than `||`). Now also tested behaviourally: dropping the guard makes
  the new test fail.
- **The exile is a cost**, paid on activation, so an opponent responding to the
  ability sees the card already in exile (CR 601.2h). **Pass**, tested in
  `activated_no_stack.rs`.
- **`ability_index` carries the chosen card's `ObjectId`.** Unusual, but sound:
  the engine looks abilities up by `a.ability_index == ability_index` rather
  than positionally, and `once_per_turn` (which is the other consumer of the
  index) is false here. Skirsdag High Priest encodes a pair-index the same way.
- **The controller of the resolving ability** is the recorded activator
  (CR 602.2a), fixed during the Skirsdag audit.
- **A token copy of a `*/*` creature** keeps the copied card's CDA, because
  `create_token_copy` copies `card_id`. Tested in `token_copy.rs`.

### Test coverage

- creates a token copy and exiles the card:
  `cards_complex_creatures.rs::back_from_the_brink_creates_token_copy`
- one ability per creature card, each at that card's cost:
  `::back_from_the_brink_ability_per_creature_in_graveyard`,
  `::back_from_the_brink_uses_creature_mana_cost`
- none with an empty graveyard:
  `::back_from_the_brink_no_abilities_without_creatures_in_graveyard`
- the ability goes on the stack rather than resolving immediately:
  `activated_no_stack.rs:45`
- X is 0, so no X-funding prompt:
  `trigger_snapshots.rs::x_cost_creature_activation_costs_only_non_x_portion`
- **a DFC: front-face cost, front-face token, cannot transform**:
  `::back_from_the_brink_copies_a_dfcs_front_face_only` (new)
- **the token's enters trigger fires**:
  `::back_from_the_brinks_token_brings_its_enters_trigger` (new)
- **an alternative cost for casting does not apply**:
  `::back_from_the_brink_ignores_an_alternative_cost_for_casting` (new)
- **a token in the graveyard is not offered**:
  `::back_from_the_brink_does_not_offer_a_token_in_the_graveyard` (new)

Mutation-checked: dropping `.without_x()`, dropping the `is_card` guard, and
taking the cost from the back face each fail the test that covers it.
