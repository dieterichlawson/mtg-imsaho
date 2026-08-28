## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/20/mausoleum-guard?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{W}, 2/2
**Oracle text**:
```
When this creature dies, create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.

Two 1/1 white Spirit tokens with flying and their subtype, created for the last-known **controller** rather than the owner — so a stolen Guard's tokens go to whoever controlled it.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/20/mausoleum-guard?utm_source=api
**Type line**: `Creature — Human Scout` — {3}{W}, 2/2
**Oracle text**:
```
When this creature dies, create two 1/1 white Spirit creature tokens with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "create **two** 1/1 white Spirit creature tokens **with flying**" — two, with
  colour, subtype and keyword via `create_token_with_subtypes`: PASS
- A death trigger, so it resolves from the graveyard: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The two Spirit tokens: `cards_morbid_and_ltb.rs`, `subtype.rs`

## Audit — 2026-08-28 18:03

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Mausoleum Guard"`, https://scryfall.com/card/isd/20/mausoleum-guard
**Oracle text**:
```
When this creature dies, create two 1/1 white Spirit creature tokens with flying.
```
**Type line**: Creature — Human Scout
**Mana cost**: {3}{W}   **P/T**: 2/2
**Rulings**: none on Scryfall for this card.
**Status**: PASS (two test gaps closed)

### Code issues
No issues found in `mtg-engine/src/cards/isd/mausoleum_guard.rs`.

`{3}{W}`, `Creature`, `subtypes: ["Human", "Scout"]` — both — 2/2, oracle text verbatim,
`TriggerKind::SelfDies` with no target requirement.

`on_dies` creates the tokens through `create_token_with_subtypes` with the full definition:
1/1, `Color::White`, `CardType::Creature`, `Keyword::Flying`, `subtypes: ["Spirit"]`. The empty
name is the convention the helper reads — CR 111.4 derives "Spirit Token" from the subtypes —
and the set's other three Spirit-token makers pass the same thing.

The controller comes from `helpers::controller_of`, which is the *last known* controller
(CR 608.2g). Reading the object's field instead would give the owner, because the zone change
to the graveyard resets it (CR 400.7) — so a stolen Guard would have left its Spirits to the
wrong player.

### Tricky interactions checked
- **Two tokens, not one**: PASS.
- **The whole token definition — white, Spirit, creature, 1/1, flying**: PASS.
- **Tokens go to the ability's controller, not the card's owner**: PASS, and this is the reason
  the card calls `controller_of`. The card itself goes to its owner's graveyard (CR 404.3)
  while the tokens arrive under the controller.
- **A token's owner (CR 111.2)**: the player it was created under, which is the same player.
- **The trigger resolves with the Guard in the graveyard**: PASS — nothing in the effect asks
  where the Guard is (CR 113.7a).
- **A token doubler**: `create_token_with_subtypes` runs the CR 614 replacement per call, so
  two calls with a doubler would give four. Nothing in this pool doubles tokens.
- **The Spirits are targetable as Spirits**: covered set-wide by `subtype.rs:452` (Urgent
  Exorcism), which is the bug that made token subtypes matter.

### Test coverage
- two Spirits, 1/1, flying: `cards_death_triggers_and_tokens.rs:73 creatures_that_leave_spirits_behind_leave_the_right_number`
  (table, shared with Doomed Traveler)
- white, Spirit subtype, creature type: same table, NEW assertions — it checked count, P/T and
  keywords, and the printed line has three more words in it
- the tokens go to whoever controlled the Guard:
  `cards_death_triggers_and_tokens.rs:105 mausoleum_guard_leaves_its_spirits_to_whoever_controlled_it` (NEW)
- a Spirit token is a legal "target Spirit": `subtype.rs:452`

Mutation-checked: making the tokens black kills the table; making one instead of two kills both;
and reading the object's `controller` field instead of `controller_of` kills only the new
controller test — which is the point of giving the Guard a different owner.

### Changes made
- `cards_death_triggers_and_tokens.rs`: three assertions added to the shared table, and the
  controller test. No code change.
