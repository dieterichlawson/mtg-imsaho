## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/11/doomed-traveler?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
When this creature dies, create a 1/1 white Spirit creature token with flying.
```

**Status**: PASS

### Code issues
No issues found.

One 1/1 white Spirit token with flying, with its subtype set via `create_token_with_subtypes`.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/11/doomed-traveler?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}, 1/1
**Oracle text**:
```
When this creature dies, create a 1/1 white Spirit creature token with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature dies, create a 1/1 white Spirit creature token **with
  flying**": PASS
- It triggers on any death — sacrificed, destroyed, or lethal damage: PASS
- Exiling it instead of letting it die gives no token: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Spirit token on death: `cards_morbid_and_ltb.rs`

## Audit — 2026-08-28 18:20

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Doomed Traveler"`, https://scryfall.com/card/isd/11/doomed-traveler
**Oracle text**:
```
When this creature dies, create a 1/1 white Spirit creature token with flying.
```
**Type line**: Creature — Human Soldier
**Mana cost**: {W}   **P/T**: 1/1
**Rulings**: none on Scryfall for this card.
**Status**: PASS (one engine path found untested)

### Code issues
No issues found in `mtg-engine/src/cards/isd/doomed_traveler.rs`.

`{W}`, `Creature`, `subtypes: ["Human", "Soldier"]` — both — 1/1, oracle text verbatim,
`TriggerKind::SelfDies` with no target requirement, and the token built through
`create_token_with_subtypes` with the full definition (1/1, `Color::White`,
`CardType::Creature`, `Keyword::Flying`, `subtypes: ["Spirit"]`; the empty name lets CR 111.4
derive "Spirit Token"). The controller is `helpers::controller_of` — the last known controller,
so a stolen Traveler leaves its Spirit to whoever controlled it.

### Tricky interactions checked
- **One Spirit, not two**: PASS — the shared table's count is exact.
- **The whole token definition**: PASS.
- **The Spirit is a creature *entering***: PASS, and this turned out to be the untested part —
  see below.
- **The Traveler is a Human, so its death feeds Village Cannibals**: PASS, and it is the Human
  row in the Cannibals' matched pair.
- **The Spirit is not a Human, so it feeds no Human-watcher when it enters**: covered by
  `champion_of_the_parish_puts_nothing_on_the_stack_for_a_creature_it_does_not_care_about`.
- **Tokens go to the controller, not the owner**: shared with Mausoleum Guard, tested there.
- **The trigger resolves with the Traveler in the graveyard**: PASS — nothing in the effect asks
  where it is.

### Test coverage
- one Spirit, 1/1, flying, white, Spirit, creature:
  `cards_death_triggers_and_tokens.rs:73 creatures_that_leave_spirits_behind_leave_the_right_number` (table row)
- the Spirit's arrival is an event watchers see:
  `cards_death_triggers_and_tokens.rs:~700 the_spirit_a_death_trigger_leaves_is_a_creature_entering` (NEW)
- a Human dying feeds Village Cannibals: `cards_death_triggers_and_tokens.rs:527` (row)
- a Spirit token is a legal "target Spirit": `subtype.rs:452`

**The new test covers a path nothing reached.** `copy_effects.rs::a_token_copy_fires_the_copied_creatures_etb_ability`
checks that a token *copy* fires an ETB watcher, but `create_token_copy` and
`create_token_with_subtypes` are separate functions with their own
`EnteredBattlefield` push. Every other token test in the suite checks what the token *is* —
its P/T, colour, subtypes, targetability — and none checked that anything noticed it arrive.
Mutation-checked: deleting the plain path's event push fails the new test and leaves the copy
test passing, which is what says they were two paths and only one was covered.

### Changes made
- `cards_death_triggers_and_tokens.rs`: one new test. No code change — the card is correct.
