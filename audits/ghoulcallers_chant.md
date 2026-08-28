## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/101/ghoulcallers-chant?utm_source=api
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`• Return target creature card from your graveyard to your hand.`)
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
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/101/ghoulcallers-chant?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Choose one —**" is modal, so the mode is chosen as the spell is cast
  (CR 601.2b) and the targets follow from it: PASS
- Mode 2 is "**two target** Zombie cards", so it needs two legal Zombie cards to
  be chosen — not "up to two": PASS
- "creature **card**" / "Zombie **cards**" — CR 109.1, and `is_valid_target`
  asks `state.is_card`: PASS
- "from **your** graveyard" on both modes: PASS
- With one of two targets illegal on resolution, the other is still returned
  (CR 608.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both modes and the card filter: `cards_modal.rs`, `cards_graveyard_recursion.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/101/ghoulcallers-chant?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/101/ghoulcallers-chant
**Oracle text**:
```
Choose one —
• Return target creature card from your graveyard to your hand.
• Return two target Zombie cards from your graveyard to your hand.
```
**Type line**: `Sorcery` · **Mana cost**: `{B}`
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — "your graveyard" was enforced by the card for one mode and by the requirement for
the other.

### Card data
| field | oracle | `ghoulcallers_chant.rs` | |
|---|---|---|---|
| cost | `{B}` | `Colored(Black)` | ok |
| types | Sorcery | `vec![CardType::Sorcery]` | ok |
| oracle_text | as above | byte-identical, bullets and em dash included | ok |
| modes | two | `ModalChoice([GraveyardCreature, TwoTargets(Zombie, Zombie)])` | ok |

**On "Zombie cards" vs creature cards.** The oracle says "two target **Zombie cards**", not "Zombie creature
cards", and the code uses `GraveyardCreatureOfSubtype`, which requires `is_creature`. These are the same set in
practice: "Zombie" is a creature type, and only a creature or Tribal card can carry one (CR 205.3m). There are
no Tribal cards in this set. Recorded rather than flagged — the quotes differ, the meaning does not.

### Code issues

**"From your graveyard" was enforced in two different places for the two modes.** Fixed.

- Mode 1 uses `GraveyardCreature`, whose enumeration filters `o.owner == caster`.
- Mode 2 uses `GraveyardCreatureOfSubtype`, whose enumeration was
  `o.zone == Zone::Graveyard && state.is_card(o.id) && state.is_creature(..) && state.has_subtype(..)` — no
  owner check, and its comment said `// Creature cards with a specific subtype in all graveyards.`
- The card put the restriction back for both modes in `is_valid_target`:
  `o.zone == Zone::Graveyard && o.owner == caster && state.is_card(o.id)`.

Nothing was wrong in play: `legal_actions` ANDs the requirement with `is_valid_target`, and `stack.rs`'s CR
608.2b re-check calls both, so an opponent's Zombie was never a legal target. The defect is the arrangement —
two sibling requirements meaning different graveyards, with the only card using the looser one compensating in
its own file. The next card to reach for `GraveyardCreatureOfSubtype` inherits a requirement whose comment
promises all graveyards and whose behaviour depends on the card remembering to narrow it.

Both doc comments were also wrong in the same direction: each said "in any graveyard", and `GraveyardCreature`
has never meant that.

### Rules check
- **CR 601.2c** — the same object cannot be chosen twice for one instance of "target". Handled by
  `options.retain(|t| t != t1)` in the `TwoTargets` enumeration, so one Zombie in your graveyard offers no
  mode-2 pair.
- **Mode-2 pairs are sets, not sequences.** `dedup_by_target_set` collapses the two orderings — the engine
  comment already names this card.
- **CR 601.2b** — a mode with no legal targets is simply not offered; `ModalChoice` concatenates each mode's
  actions, so an empty mode contributes none.
- **CR 608.2b** — with two targets, one becoming illegal does not counter the spell; the other still returns.
  The `zone == Graveyard` guard in the `on_resolve` loop is what skips the departed one, and it is genuinely
  load-bearing here (unlike the single-target case, where the spell never resolves at all).
- **CR 109.1** — `is_card` excludes a token sitting in a graveyard before the next SBA pass. Checked by both
  requirements and by the re-check.

### Changes made
- `mtg-engine/src/engine/targeting.rs` — `GraveyardCreatureOfSubtype` now filters `o.owner == caster`, matching
  its sibling, with a comment saying why.
- `mtg-engine/src/cards/mod.rs` — both doc comments corrected from "any graveyard" to the caster's.
- `mtg-engine/src/cards/isd/ghoulcallers_chant.rs` — `is_valid_target` removed as a restatement of both
  requirements. The doc comment records that the `on_resolve` zone guard is *not* in the same category.
- `mtg-engine/tests/cards_graveyard_interaction.rs` — two table rows and one new test.

### The coverage hole was exactly at that seam
The existing table already had a "'your graveyard' — an opponent's creature card is not a legal target" row.
But it used an opponent's **Grizzly Bears**, and mode 2's Zombie restriction explains that result just as well
as the ownership restriction does. Only an opponent's **Zombie** separates "not yours" from "not a Zombie" —
and that is precisely the case the requirement did not cover and the card did. Added:

- two Zombies in the opponent's graveyard → no singles, no pairs;
- one Zombie of yours and one of theirs → one single, still no pair.

Plus `mode_two_returns_the_zombie_that_is_still_there` for CR 608.2b in both directions (one target gone, both
gone).

### Mutation checks (all discriminating)
1. Owner check removed from `GraveyardCreatureOfSubtype` again — now with the card's `is_valid_target` gone —
   → `each_mode_offers_exactly_the_cards_it_may_name` FAILED. Only the new rows catch this; the pre-existing
   "not your graveyard" row does not.
2. `on_resolve`'s `zone == Graveyard` guard removed → `mode_two_returns_the_zombie_that_is_still_there` FAILED.
3. Mode 2's `GraveyardCreatureOfSubtype("Zombie")` → `GraveyardCreature` →
   `each_mode_offers_exactly_the_cards_it_may_name` FAILED.
4. `options.retain(|t| t != t1)` removed (CR 601.2c) → same test FAILED.

### Tricky interactions checked
- One Zombie only → mode 1 offered, mode 2 not: **pass** (existing row).
- Opponent's Zombies → neither mode: **pass** (new).
- Mode 2 cannot borrow an opponent's Zombie for its second slot: **pass** (new).
- One of two targets leaves in response → the other still returns: **pass** (new).
- Both leave → countered by game rules: **pass** (new).
- A token in a graveyard is not a card: **pass** (`token_is_not_a_card.rs:217`).

### Test coverage
- what each mode may name, across seven graveyard shapes: `cards_graveyard_interaction.rs:433` (two rows new)
- mode 1 returns what it named: `cards_graveyard_interaction.rs:490`
- mode 2 returns both: `cards_graveyard_interaction.rs:507`
- CR 608.2b, one target gone and both gone: `cards_graveyard_interaction.rs:522` (new)
- token in a graveyard is not a legal target: `token_is_not_a_card.rs:217`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1411 passing.

