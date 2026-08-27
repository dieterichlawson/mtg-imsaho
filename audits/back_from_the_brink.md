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
