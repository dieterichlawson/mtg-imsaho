## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/224/ghoulcallers-bell?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{T}: Each player mills a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Each** player mills a card" — no targeting, so it cannot be responded to by
  making a player untargetable, and it hits its own controller too: PASS
- The mill goes through `mill_cards`, so a creature card among them emits
  `CreatureCardMilled`: PASS
- A player with an empty library mills nothing rather than losing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both players mill: `cards_lands_and_mana_sources.rs:ghoulcallers_bell_mills_both_players`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/224/ghoulcallers-bell?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{T}: Each player mills a card.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `{T}: Each player mills a card.`
**Type line**: `Artifact` — {1}
**Status**: ISSUE (fixed) — two test gaps; the card is correct

### Rulings
None on Scryfall.

### Code issues

No issues in the card. `{1}`, Artifact, oracle text verbatim, one `{T}` ability with no mana cost and no restrictions, and the mill through `engine::mill_cards` — the shared pipeline, so a creature card milled here emits `CreatureCardMilled` for an opponent's Undead Alchemist and the log names the source. It also carries no zone-or-tapped guard of its own, with a comment explaining that `legal_actions` owns that; that is the right side of the line.

Two gaps:

- **"mills *a* card" was a claim about how many, and nothing checked the number.** The test gave each library exactly one card, so milling *two* milled the same one card and looked identical — it passed the whole workspace. The lesson was already written down one card over: Mindshrieker's test says "A second card underneath, so 'mills **a** card' is a claim about how many. With one card in the library, milling two looks the same." Each library now has that second card.
- **The empty-library case was untested**, and it is the difference between milling as an *effect* and as a *cost* (CR 701.17b). A player with no library mills nothing and the ability still works; a *cost* that includes milling cannot be paid at all, which is Deranged Assistant's gate two audits back. Refusing to act when any library is empty passed everything.

### Tricky interactions checked

- "**Each** player", including the controller: PASS, and tested — milling only the controller fails.
- Exactly one card each: PASS. Untested until this audit.
- An empty library mills nothing rather than blocking the ability: PASS. Untested until this audit.
- The mill goes through the pipeline, so `CreatureCardMilled` reaches an opponent's Undead Alchemist: PASS by construction — `mill_cards` is the only route.
- `{T}` in the cost, and no zone guard in the card: PASS, and `activated_ability_costs_are_the_costs_the_oracle_text_prints` now pins the `requires_tap` flag to the printed `{T}`.
- Instant speed, repeatable across turns: PASS, no restriction flags, pinned to the text by the same invariant.
- Milling order between players: not modelled as simultaneous; the loop runs in player order. Unobservable here — nothing in this pool watches for the order two players' cards reach their graveyards, and CR 701.17a makes each player's mill its own action anyway.

### Test coverage

- Both players mill, and exactly one card each: `cards_lands_and_mana_sources.rs:311` `ghoulcallers_bell_mills_both_players` — the "exactly one" half added this audit
- An empty library mills nothing and the ability still works: `cards_lands_and_mana_sources.rs:349` `ghoulcallers_bell_mills_what_it_can_from_an_empty_library`, added this audit
- The `{T}` cost is real — a tapped Bell offers nothing: `tap_cost_legality.rs:198` (table row)
- The declared cost matches the printed one: `card_data_invariants.rs:1706`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 mill 2 instead of 1 | passed whole workspace | `ghoulcallers_bell_mills_both_players` FAILED |
| M2 only the controller mills | `ghoulcallers_bell_mills_both_players` FAILED | (unchanged) |
| M3 `requires_tap: false` | 2 tests FAILED | (unchanged) |
| M4 refuse to act when any library is empty | passed whole workspace | `ghoulcallers_bell_mills_what_it_can_from_an_empty_library` FAILED |

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1499 passing (was 1498). `cargo check --workspace --all-targets` clean, zero warnings.
