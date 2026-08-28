## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/27/purify-the-grave?utm_source=api
**Type line**: `Instant` — {W}
**Oracle text**:
```
Exile target card from a graveyard.
Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: ISSUE

### Code issues
See below.


- The engine's graveyard target enumeration offered tokens.
  - Oracle text says: `Exile target card from a graveyard.`
  - Code did: `state.objects.values().filter(|o| o.zone == Zone::Graveyard)` in
    `engine/targeting.rs`, with no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so an enumeration taken in between
    can see one. This is the *engine's* variant list rather than this card's
    code — six `TargetRequirement` variants name a "card" in a graveyard or in
    exile and none of them asked, so the fix is shared by everything that uses
    them, and `stack.rs`'s resolution-time re-check now asks too.

### Tricky interactions checked
- "from **a** graveyard" — any graveyard, not only your own: PASS
- Flashback {W}, the same cost as the front face, and the card is exiled after
  the flashback resolution: PASS
- Exiling the card the spell itself would go to is fine — the spell is on the
  stack while it resolves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not offered: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_a_targetable_card`
- Exile from either graveyard, and flashback: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/27/purify-the-grave?utm_source=api
**Type line**: `Instant` — {W}
**Oracle text**:
```
Exile target card from a graveyard.
Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2011-09-22] If it's your turn, and you cast a spell with flashback from your hand, you'll have priority to cast that spell again using flashback before any other player has priority to cast Purify the Grave in order to remove that card from your graveyard.
- [2011-09-22] Purify the Grave can't be used in response to a spell being cast with flashback or a spell that requires cards to exiled from a graveyard as an additional cost to try and counter the spell or prevent it from being cast.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine: a spell cast from a graveyard was offered as its own
target.**

- Oracle text says: `Exile target card from a graveyard.` plus `Flashback {W}`.
- `engine/targeting.rs` enumerated `GraveyardCard` as
  `.filter(|o| o.zone == Zone::Graveyard && state.is_card(o.id))` — every card
  in every graveyard, this spell included.

CR 601.2a moves the card to the stack; CR 601.2c chooses targets after that.
A spell being cast from its graveyard is not in that graveyard any more and
cannot be one of its own targets. `legal_actions` offered it anyway — I probed
it and got two casts, one of them `targets=[Object(purify)]`.

The end state happened to be the same (the self-targeting cast fizzles at
resolution, because by then the zone check does see the stack, and a flashback
cast is exiled either way), but the spell is *countered* rather than resolved
— which is not nothing to anything watching `SpellResolved` — and an illegal
action was in the list a player or an LLM picks from.

Fixed where the enumeration happens rather than per card: every arm of
`valid_targets_for_req` that walks a zone of cards now excludes `spell_id` —
the five graveyard requirements and `ExileCard`. Two cards in the set can ask
the question (this one, and Memory's Journey with flashback `{G}` and a
graveyard-card slot); Unburial Rites has flashback too but its
`GraveyardCreature` requirement already excludes a Sorcery.

### Card data

`{W}` Instant, flashback `{W}`, `TargetRequirement::GraveyardCard` for "target
card from a graveyard" — all matching, and pinned pool-wide by
`card_data_invariants.rs`. `GraveyardCard` is the right requirement precisely
because it is the unrestricted one: "a graveyard", not "your graveyard", and
"card", not "creature card". Both halves are now asserted rather than assumed.
`GraveyardCard => true` in the CR 608.2b re-check (added during the Unburial
Rites audit) is correct for the same reason: this requirement really does say
only which zone.

### Tricky interactions checked

- Self-target when cast via flashback: **was offered, fixed**.
- "a graveyard" — the caster's own counts: pass, now asserted on the *offer*,
  which is where the scope lives. Resolving a submitted target proves only
  that the card exiles what it was pointed at; the cast handler takes the
  targets it is given.
- "card" — a land, not just a creature card: pass.
- Ruling (2011-09-22): "Purify the Grave can't be used in response to a spell
  being cast with flashback, or a spell that requires cards to be exiled from a
  graveyard as an additional cost." Both follow from CR 601.2: the card leaves
  the graveyard and the cost is paid while the spell is being cast, before
  anyone gets priority. The engine does this — the Corpse Lunge audit's
  resolution-timing test asserts the exiled card is already gone before the
  response window — so there is nothing here to implement.
- Ruling (2011-09-22) about the flashback caster keeping priority: CR 117.3b,
  engine-level.
- Fizzle when the target leaves the graveyard: the same shape covered by
  `fizzle.rs::a_graveyard_target_that_leaves_the_graveyard_counters_the_spell`.
- Tokens: `is_card` excludes one sitting in a graveyard before SBAs (CR 109.1).

### Test coverage

- exiles an opponent's card:
  `cards_graveyard_interaction.rs::purify_the_grave_exiles_card_from_graveyard`
- "a graveyard" and "card" — a land in the caster's own graveyard, checked as
  an offer:
  `cards_graveyard_interaction.rs::purify_the_grave_exiles_any_card_from_any_graveyard` (new)
- not its own target when cast from a graveyard:
  `flashback.rs::a_spell_cast_from_a_graveyard_is_not_offered_as_its_own_target` (new)
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- Drop the `o.id != spell_id` exclusion again: **fails** the self-target test.
- Restrict `GraveyardCard` generation to the caster's own graveyard: passed the
  first version of the any-graveyard test, because it resolved a submitted
  target rather than checking the offer. Rewritten to assert the offer, after
  which the mutation **fails** it. Recorded because the first version was the
  weaker test and would have shipped.
- The card moves its target to hand rather than exile: **fails** both card
  tests.

Suite: 1525 passing, exit 0, `cargo check --workspace --all-targets` clean.
