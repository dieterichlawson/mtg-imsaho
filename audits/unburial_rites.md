## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/122/unburial-rites?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Return target creature card from your graveyard to the battlefield.
Flashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Return target creature **card** from **your** graveyard to the battlefield" —
  a card (CR 109.1, now enforced in the engine's graveyard enumeration), from
  the caster's own graveyard: PASS
- The creature returns under the *caster's* control, not its owner's — the card
  says "to the battlefield" with no owner clause: PASS
- Its enters-the-battlefield triggers fire: PASS
- The spell stays on the stack while the choice chain runs, and the engine moves
  it afterwards (CR 608.2m): PASS
- Flashback {3}{W} is a different colour from the {4}{B} front cost, which is
  the card's whole design: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The reanimation and the flashback: `cards_flashback.rs`, `spell_cleanup.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/122/unburial-rites?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Return target creature card from your graveyard to the battlefield.
Flashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine: the CR 608.2b re-check for a graveyard target said only
which zone it was in.**

- Oracle text says: `Return target creature card from your graveyard to the battlefield.`
- `stack.rs::is_target_legal` said, for every graveyard requirement in the set:
  ```rust
  TargetRequirement::GraveyardCard
  | TargetRequirement::GraveyardCreature
  | TargetRequirement::GraveyardCreatureOfSubtype(_)
  | TargetRequirement::GraveyardCardOwnedByCaster
  | TargetRequirement::GraveyardCardOwnedByOpponent
  | TargetRequirement::GraveyardCardOwnedByTargetPlayer =>
      obj.zone == Zone::Graveyard && state.is_card(*id),
  ```

`targeting.rs` generates `GraveyardCreature` targets as "creature cards in the
**caster's** graveyard" — `o.owner == caster && state.is_card(o.id) &&
state.is_creature(o.id, registry)`. The re-check kept only the middle clause,
so "your graveyard" and "creature" were enforced when the target was chosen and
taken on trust when the spell resolved. A target the engine never offers — an
opponent's creature card — passed the re-check and was reanimated onto the
battlefield.

This is the same gap that had already been closed for bare `Creature` and
`CreatureWithFilter` a few cards ago; the graveyard requirements were the rest
of it. Each arm now re-checks the clauses `targeting.rs` generates against:
owner for `GraveyardCreature`, `GraveyardCardOwnedByCaster` and
`GraveyardCardOwnedByOpponent`, plus creature-ness and, for Ghoulcaller's
Chant, the subtype. `GraveyardCardOwnedByTargetPlayer` stays zone-only with a
comment: whose graveyard is named by the spell's *other* target (Memory's
Journey), which this function is asked one target at a time and cannot see.

### Card data

`{4}{B}` Sorcery, flashback `{3}{W}`, `TargetRequirement::GraveyardCreature`
for "target creature card from your graveyard" — all matching, with cost, type
line and the two-colour flashback cost pinned pool-wide by
`card_data_invariants.rs`, and the graveyard cast covered by the flashback
sweep. No `is_valid_target` override, and correctly so now that the requirement
carries its whole meaning on both sides.

`move_object(id, Zone::Battlefield)` rather than `move_object_under_control`:
equivalent here and only here. A card in a graveyard is in its *owner's*
graveyard (CR 404.3), "your graveyard" makes that owner the caster, and
`move_object` reset `controller` to `owner` on the way in — so the three
players who could be meant are the same player. Worth knowing rather than
worth changing.

### Tricky interactions checked

- "your graveyard" — an opponent's creature card: **was reanimated, fixed**.
- A noncreature card in your own graveyard: not offered, and now not legal on
  re-check either.
- Fizzle — the target exiled in response: pass, and newly tested. The existing
  fizzle cases all move their target *into* a graveyard, so a re-check that
  only asked "is this in a graveyard" passed every one of them for the wrong
  reason; this one moves a target the other way.
- The reanimated creature enters under the caster's control and summoning sick
  (CR 400.7 makes it a new object): pass, now asserted.
- Tokens: a token in a graveyard ceases to exist (CR 111.7) and `is_card`
  excludes it either way.

### Recorded, not fixed

**CR 400.7 target identity is not modelled.** `zone_change_count` is tracked on
every object and consulted by nothing, so a target that leaves its zone and
comes back is still the same target as far as the re-check is concerned, when
the rules make it a new object and an illegal one. Not fixed because it is not
constructible in this pool: it needs an instant-speed way to move a card out of
a graveyard and back while a spell targeting it is on the stack. Purify the
Grave is the only instant that touches a graveyard card and it exiles one
permanently; nothing returns a card from exile to a graveyard; and the two
cards that return a graveyard card to hand (Ghoulcaller's Chant, and this one
to the battlefield) are sorceries.

### Test coverage

- returns the creature, under the caster's control, summoning sick:
  `flashback.rs::unburial_rites_returns_creature` (extended)
- the chosen target and only it:
  `cards_morbid_and_ltb.rs::unburial_rites_choice_with_multiple_creatures`
- uncastable with no creature card anywhere:
  `characteristics_targeting.rs::unburial_rites_is_not_castable_with_no_creature_card_to_return`
- not from an opponent's graveyard, at cast *and* on resolution:
  `characteristics_targeting.rs::unburial_rites_cannot_reanimate_out_of_an_opponents_graveyard` (new)
- fizzle when the target leaves the graveyard:
  `fizzle.rs::a_graveyard_target_that_leaves_the_graveyard_counters_the_spell` (new)
- flashback reachable from the graveyard:
  `flashback.rs::every_flashback_card_is_offered_from_the_graveyard`

### Mutations run

- Drop the new `graveyard_ok` re-check: **fails** the opponent's-graveyard test.
- Drop `o.owner == caster` from `targeting.rs`'s generation instead: **fails**
  the same test on its first assertion — the two halves are pinned separately.
- The card returns the creature to hand rather than the battlefield: **fails**
  the main test.
- The graveyard arm drops its *zone* clause: **fails** the new fizzle test,
  which is what that test is for.

Suite: 1523 passing, exit 0, `cargo check --workspace --all-targets` clean.
