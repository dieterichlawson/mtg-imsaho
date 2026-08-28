## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/46/cackling-counterpart?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "a token that's a **copy of** target creature you control" — copies the
  printed characteristics, not counters, Auras, or non-copy effects (CR 707.2):
  PASS
- "target creature **you control**": PASS
- The token is a token, so it ceases to exist if it leaves the battlefield: PASS
- Flashback {5}{U}{U}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The token copy and the flashback: `cards_flashback.rs`, `cards_complex_creatures.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/46/cackling-counterpart?utm_source=api
**Type line**: `Instant` — {1}{U}{U}
**Oracle text**:
```
Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2025-01-24] The token copies exactly what was printed on the original creature and nothing else (unless that creature is copying something else or is a token; see below). It doesn’t copy whether that creature is tapped or untapped, whether it has any counters on it or Auras and Equipment attached to it, or any non-copy effects that have changed its power, toughness, types, color, or so on.
- [2025-01-24] If the copied creature has {X} in its mana cost, X is considered to be 0.
- [2025-01-24] If the copied creature is a token, the token that’s created copies the original characteristics of that token as stated by the effect that created the token.
- [2025-01-24] If the copied creature is copying something else, then the token enters the battlefield as whatever that creature copied.
- [2025-01-24] Any “enters” triggered ability of the copied creature will trigger when the token enters the battlefield. Any “as [this creature] enters” or “[this creature] enters with” abilities of the chosen creature will also work.
- [2024-11-08] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2024-11-08] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2024-11-08] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2024-11-08] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2024-11-08] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2024-11-08] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: `Instant` — {1}{U}{U}, Flashback {5}{U}{U}
**Status**: ISSUE (fixed) — a real gameplay bug in the copy machinery

### Rulings (the copy-specific ones; the rest are the standard flashback five)
1. [2025-01-24] "The token copies exactly what was printed on the original creature and nothing else (unless that creature is copying something else or is a token; see below). It doesn't copy whether that creature is tapped or untapped, whether it has any counters on it or Auras and Equipment attached to it, or any non-copy effects that have changed its power, toughness, types, color, or so on."
2. [2025-01-24] "If the copied creature has {X} in its mana cost, X is considered to be 0."
3. [2025-01-24] "If the copied creature is a token, the token that's created copies the original characteristics of that token as stated by the effect that created the token."
4. [2025-01-24] "If the copied creature is copying something else, then the token enters the battlefield as whatever that creature copied."
5. [2025-01-24] "Any 'enters' triggered ability of the copied creature will trigger when the token enters the battlefield."

### Code issues

- `mtg-engine/src/state.rs:614` — `create_token_copy` copied the object's current fields rather than the printed face.
  - Ruling 1 says: `It doesn't copy ... any non-copy effects that have changed its power, toughness, types, color, or so on.`
  - Code did: `Some(o) => (o.name.clone(), o.power, o.toughness, ...)` for P/T, and `registry.card_data(card_id)` — the card's **front** face — for the rest.
  - Tree of Redemption's "exchange your life total with this creature's toughness" writes `obj.toughness`, and it is the only card in the set that writes those fields. Confirmed by running it: a Tree exchanged down to 0/4 was copied as a **0/4**, not its printed 0/13.
  - The second half is the same read in the other direction: a **transformed** permanent was copiable as its front face, where CR 712.8a gives it the characteristics of the face that is up. Copying a flipped Village Ironsmith produced a 1/1 Village Ironsmith rather than a 3/1 Ironfang.
  - Both fixed by one change: read the active face through `face_data`, falling back to the object's own fields only for a token — which has no face, and whose fields *are* its printed characteristics. That fallback is ruling 3.

The card itself is right: `{1}{U}{U}`, Instant, oracle text verbatim, flashback `{5}{U}{U}`, `CreatureWithFilter(YouControl)` for "target creature you control", and the copy through the shared `create_token_copy`.

### Tricky interactions checked

- Ruling 1, P/T written by an effect: FAIL before this audit, PASS now.
- Ruling 1, counters and tapped state not copied: PASS — counters live in `obj.counters` and are not read; `create_token_internal` starts untapped. Now asserted.
- Ruling 1, Auras and Equipment not copied: PASS — attachments are separate objects pointing at the original.
- Ruling 1, until-end-of-turn pumps not copied: PASS — they live in `until_end_of_turn` keyed by the original's id.
- Ruling 4 / CR 712.8a, the face that is up: FAIL before this audit, PASS now.
- Ruling 3, copying a token: PASS, and it is exactly the fallback arm — a token has no `face_data`, so its object fields are used.
- Ruling 5, the copied creature's ETB triggers fire for the token: PASS, and tested from the other side in `replacement_effects.rs::a_token_entering_as_a_copy_of_a_human_gets_the_counter`, added during the Dearly Departed audit.
- Ruling 2, `{X}` counts as 0: N/A — the token has no mana cost to pay and the engine reads `x_value` from the spell object, which a token has none of.
- The legend rule on a copy of a legend: PASS — `is_legendary` is carried across deliberately, with a comment saying why.
- "target creature **you control**": PASS, and the token is created under the *spell's* controller, which is the same player.

### Test coverage

- A token copy is made, with the copied creature's P/T: `cards_spells_and_enchantments.rs:717` `cackling_counterpart_creates_token_copy`
- Ruling 1 — printed P/T, no counters, not tapped: `cards_spells_and_enchantments.rs:737` `cackling_counterpart_copies_the_printed_creature_and_nothing_else`, added this audit
- CR 712.8a — the face that is up: `cards_spells_and_enchantments.rs:775` `cackling_counterpart_copies_the_face_that_is_up`, added this audit
- Ruling 5 — the token's ETB is the copied creature's: `replacement_effects.rs:436`
- Flashback cost: `card_data_invariants.rs` (added during the Sever the Bloodline audit)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 read `obj.power` / `obj.toughness` again | n/a (was the bug) | both new tests FAILED |
| M2 read `card_data(card_id)` (front face) instead of `face_data` | n/a (was the bug) | `cackling_counterpart_copies_the_face_that_is_up` FAILED |

The bug was found by reading `create_token_copy` and asking which fields could differ from the printed card, then grepping the card pool for anything that writes them — one card does. Confirmed with a throwaway run before writing the fix, rather than reasoning about it.

Source restored from `/tmp/state_cc.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1502 passing (was 1500). `cargo check --workspace --all-targets` clean, zero warnings.
