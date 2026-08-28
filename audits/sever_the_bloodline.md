## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/115/sever-the-bloodline?utm_source=api
**Type line**: `Sorcery` — {3}{B}
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

**Ruling [2025-01-24]**: "Sever the Bloodline has only one target. Other
creatures with the same name will be exiled even if they have hexproof or
protection."

- This is the trap, and the code avoids it: the "all other creatures with the
  same name" sweep is a plain name match over the battlefield with no
  targetability filter, so a hexproofed same-named creature is still exiled.
  Routing it through the targeting helpers — the obvious shortcut — would have
  been wrong.
- The target itself is included in the sweep, matching "target creature **and**
  all other creatures with the same name".
- Name comes from `o.name`, which `apply_transform` keeps in step with the
  active face, so a transformed DFC is matched by the name it currently has
  (CR 712.8).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/115/sever-the-bloodline?utm_source=api
**Type line**: `Sorcery` — {3}{B}
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Sever the Bloodline has **only one target**. Other creatures with the
  same name will be exiled **even if they have hexproof or protection**." Only
  the first creature is a target; the sweep that follows filters on name alone,
  with no targetability check: PASS
- Ruling: "If the target creature is an illegal target by the time Sever the
  Bloodline tries to resolve, the spell won't resolve. You won't exile **any**
  creatures at all." The whole body is gated on the target, and the engine
  substitutes `Target::Illegal` for one that stopped being targetable — which
  fails the `Target::Object(..)` match, so nothing is exiled: PASS
- The name comparison reads the object's name, which on the battlefield is the
  *active* face's — `apply_transform` refreshes it — so a transformed Werewolf
  matches by its back face's name, which is what "the same name as that
  creature" means: PASS
- Ruling on token names: a token's name is its subtypes plus "Token", so two
  Wolf tokens share a name and are swept together: PASS
- **Exile**, not destroy, so indestructible does not save them and they do not
  reach a graveyard: PASS
- Flashback {5}{B}{B}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The same-name sweep and the single target: `cards_removal.rs`, `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/115/sever-the-bloodline?utm_source=api
**Type line**: `Sorcery` — {3}{B}
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2025-01-24] Sever the Bloodline has only one target. Other creatures with the same name will be exiled even if they have hexproof or protection.
- [2025-01-24] Unless a token is a copy of another permanent or was explicitly given a name by the effect that created it, its name is the subtypes it was given when it was created plus the word “Token.” For example, if an effect creates a 1/1 Soldier creature token, that token is named “Soldier Token.”
- [2025-01-24] If the target creature is an illegal target by the time Sever the Bloodline tries to resolve, the spell won’t resolve. You won’t exile any creatures at all.
- [2022-06-10] Unless a token is a copy of another permanent or was explicitly given a name by the effect that created it, its name is the subtypes it was given when it was created plus the word "Token." For example, if an effect creates a 1/1 Soldier creature token, that token is named "Soldier Token."
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2017-03-14] Only creatures on the battlefield will be exiled. In other zones, they're "creature cards," not "creatures."
- [2017-03-14] A double-faced creature only has the name of the face that's up. For example, if Village Ironsmith is targeted by Sever the Bloodline, Ironfang wouldn't be exiled.
- [2017-03-14] If the targeted creature is an illegal target by the time Sever the Bloodline resolves, it won't resolve and none of its effects will happen. No creatures will be exiled.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Exile target creature and all other creatures with the same name as that creature.
Flashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: `Sorcery` — {3}{B}, Flashback {5}{B}{B}
**Status**: ISSUE (fixed)

### Rulings (13; the ones specific to this card)
1. [2025-01-24] "Sever the Bloodline has only one target. Other creatures with the same name will be exiled even if they have hexproof or protection."
2. [2025-01-24 / 2022-06-10] "Unless a token is a copy of another permanent or was explicitly given a name by the effect that created it, its name is the subtypes it was given when it was created plus the word 'Token.'"
3. [2025-01-24] "If the target creature is an illegal target by the time Sever the Bloodline tries to resolve, the spell won't resolve. You won't exile any creatures at all."
4. [2017-03-14] "Only creatures on the battlefield will be exiled. In other zones, they're 'creature cards,' not 'creatures.'"
5. [2017-03-14] "A double-faced creature only has the name of the face that's up. For example, if Village Ironsmith is targeted by Sever the Bloodline, Ironfang wouldn't be exiled."

The remaining eight are the standard flashback rulings.

### Code issues

- `mtg-engine/src/cards/isd/sever_the_bloodline.rs:41` and `mtg-engine/src/engine/targeting.rs:509` — a rules decision about a name read the display cache.
  - Ruling 5 says: `A double-faced creature only has the name of the face that's up.`
  - The card did: `.filter(|o| state.is_creature(o.id, registry) && o.name == name)`, with `name` taken from `o.name` too.
  - `state.rs` says of that field, where it defines `name_of`: `obj.name is only authoritative for tokens, which have no registry face; for a real card it is a display cache that goes stale (CR 712.8a: a DFC outside the battlefield has its front face's name).` The engine's own `SameNameAsSource` filter — the one behind Evil Twin's granted "destroy target creature with the same name" — compared `source.name == obj.name` for the same reason.
  - Both now read `name_of`, and `test_suite_guards.rs::a_rules_decision_about_a_name_goes_through_name_of` keeps the field out of future comparisons. Reading it for a log line stays fine.
  - **Behaviour-neutral today**, and reported as such: the mutation back to `obj.name` still passes, because `apply_transform` mirrors the back face's name into the field for every DFC — and every DFC declares `back_face_data`, which `every_card_with_a_back_face_declares_it` enforces. The change is that the card now states the rule instead of depending on that chain holding.

- The flashback cost was unpinned. Turning `{5}{B}{B}` into `{4}{B}{B}` passed the whole workspace, and so did deleting `flashback_cost` outright. A flashback test pays what the card asks and then checks the card was cast and exiled, so it cannot notice the ask changing. Closed pool-wide by `card_data_invariants.rs::flashback_costs_are_the_costs_the_oracle_text_prints`, in both directions, over 20+ cards.

Everything else is right: `{3}{B}`, Sorcery, oracle text verbatim, `TargetRequirement::Creature` for the single target, the sweep over battlefield creatures only, and exile through `move_object` with no self-cleanup.

### Tricky interactions checked

- Ruling 1, hexproof does not save a non-target: PASS. The card checks nothing about targetability for the others, correctly. Untested until this audit.
- Ruling 5, the face that is up: PASS.
- Ruling 4, only creatures on the battlefield: PASS, and already guarded — extending the sweep to graveyards was caught by `a_card_enumerating_a_graveyard_excludes_tokens`.
- "and all **other** creatures": the target is included in the sweep by name, so the wording is satisfied without a special case; exiling it twice is impossible because it has already moved.
- Ruling 3, an illegal target means nothing is exiled: PASS, and it is the engine's — `stack::resolve_spell` applies CR 608.2b before `on_resolve` is reached. Covered generally in `fizzle.rs`.
- Ruling 2, token names: PASS by construction. `create_token_with_subtypes` implements CR 111.4, which `tokens_are_named_after_their_subtypes` pins; two Spirit tokens therefore share a name and are swept together.
- Your own creatures are swept too: PASS, tested.
- Redundant `zone == Battlefield` preamble in `on_resolve`: present, left alone, consistent with the other cards in this run.

### Test coverage

- Exiles the target and all others with the name, including your own: `cards_spells_and_enchantments.rs:742` `sever_the_bloodline_exiles_all_with_same_name`
- Ruling 5, DFC face: `cards_spells_and_enchantments.rs:777` `sever_the_bloodline_reads_the_face_that_is_up`, added this audit
- Ruling 1, hexproof same-name creature still exiled: `cards_spells_and_enchantments.rs:803` `sever_the_bloodline_exiles_same_named_creatures_that_could_not_be_targeted`, added this audit
- Ruling 4, graveyard creature cards untouched: `test_suite_guards.rs::a_card_enumerating_a_graveyard_excludes_tokens`
- Ruling 3, fizzle: `fizzle.rs` (general)
- Flashback cost is `{5}{B}{B}`: `card_data_invariants.rs:1897`, added this audit
- The name comparison reads the active face: `test_suite_guards.rs::a_rules_decision_about_a_name_goes_through_name_of`, added this audit

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 exile only the target | `sever_the_bloodline_exiles_all_with_same_name` FAILED | (unchanged) |
| M2 also sweep graveyards | `a_card_enumerating_a_graveyard_excludes_tokens` FAILED | (unchanged) |
| M3 skip your own creatures | `sever_the_bloodline_exiles_all_with_same_name` FAILED | (unchanged) |
| M4 flashback `{5}{B}{B}` -> `{4}{B}{B}` | passed whole workspace | `flashback_costs_are_the_costs_the_oracle_text_prints` FAILED |
| M5 delete `flashback_cost` | passed whole workspace | same invariant FAILED |
| M6 name comparison back to `obj.name` | passed | passed behaviourally — **vacuous**; now caught by the new guard |
| M7 skip hexproof creatures in the sweep | n/a | `sever_the_bloodline_exiles_same_named_creatures_that_could_not_be_targeted` FAILED |

M6 is recorded as vacuous rather than as a caught bug. `obj.name` and `name_of` give the same answer for every object in this pool, so no test can tell them apart by behaviour; the guard is what makes the change enforceable.

A hypothesis I formed and discarded mid-audit, recorded so it is not re-derived: I expected `name_of` to be *wrong* for a permanent that entered as a copy, since `face_data` resolves through `card_id` and I thought a copy kept its own. It does not — `engine/effects.rs` sets `obj.card_id` to the copied card, so both accessors agree there too.

Sources restored from `/tmp/stb.bak` and `/tmp/stb2.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1489 passing (was 1485). `cargo check --workspace --all-targets` clean, zero warnings.
