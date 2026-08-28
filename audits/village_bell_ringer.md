## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/216/village-bell-ringer?utm_source=api
**Type line**: `Creature — Human Scout` — {2}{W}, 1/4
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```

**Status**: PASS

### Code issues
No issues found.

- "untap **all creatures you control**" — filters to the controller's creatures
  and to tapped ones; no targeting, matching "all" rather than "target".
- Flash is a keyword on the card, so instant-speed casting is the engine's, not
  a hand-rolled timing exception.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/216/village-bell-ringer?utm_source=api
**Type line**: `Creature — Human Scout` — {2}{W}, 1/4
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, **untap all creatures you control**" — all of
  them, no targeting, so hexproof is irrelevant: PASS
- Untapping an attacking creature does not remove it from combat (CR 506.4c),
  which is the point of the card with flash: PASS
- Flash: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mass untap: `cards_complex_creatures.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/216/village-bell-ringer?utm_source=api
**Type line**: `Creature — Human Scout` — {2}{W}, 1/4
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```

**Rulings fetched**:
- [2011-09-22] Untapping an attacking creature doesn't remove it from combat.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/moc/216/village-bell-ringer
(the cache holds a later printing of the same card; oracle text is per-card, not per-printing)
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```
**Type line**: Creature — Human Scout
**Mana cost**: {2}{W} — **P/T**: 1/4 — **Keywords**: Flash
**Rulings** (1, 2011-09-22): "Untapping an attacking creature doesn't remove it from combat."

**Status**: ISSUE (fixed) — the card code is correct; two of its claims had no test.

### Card data
Matches the fetched text: `{2}{W}`, `card_types: [Creature]`,
`subtypes: ["Human", "Scout"]` (both), 1/4, `keywords: [Flash]`, oracle text
verbatim in the current "When this creature enters" errata wording, and one
`TriggeredAbilityDef` of kind `EntersBattlefield` with
`target_requirement: None` — correct, the ability targets nothing ("all
creatures you control" is not a target).

### Code issues

No issue in `village_bell_ringer.rs`. Two mutations passed the entire
workspace.

1. **"untap all *creatures* you control" was not a claim about what kind of
   permanent** (`cards_death_triggers_and_tokens.rs:104`, test extended).
   - Oracle text says: `untap all creatures you control`
   - Code says: `.filter(|o| state.is_creature(o.id, registry) && o.tapped)`
   - Verified: dropping the `is_creature` half — untap every tapped permanent
     you control — produced zero failures. The test stood up two tapped
     creatures of yours and one of the opponent's, and no non-creature at all.
   - A tapped Forest of P0's now stands there and must stay tapped.

2. **The ruling had no test** (same file, test added).
   - Ruling says: `Untapping an attacking creature doesn't remove it from combat.`
   - Verified: adding `crate::destruction::remove_from_combat(state, id)` beside
     the untap produced zero failures.
   - Added `village_bell_ringer_leaves_an_untapped_attacker_in_combat`: a
     declared attacker is tapped by attacking (CR 508.1f), the Bell-Ringer
     untaps it, and it is still in `combat.attackers`. CR 506.4 lists what
     removes a creature from combat, and being untapped is not on it.

### Tricky interactions checked
- All of your creatures, not just one: PASS — two are checked, and untapping
  only the first fails.
- "you control": PASS — the opponent's tapped creature stays tapped.
- Creatures only: PASS — new assertion.
- The ruling (attacker stays attacking): PASS — new test.
- **"Doesn't untap during its controller's untap step" does not stop this.**
  Claustrophobia's `ContinuousEffect::PreventUntap` is read only by
  `state.untaps_normally` (CR 502.2), which only `engine.rs`'s untap step
  consults. The card untapping directly is therefore right, not an oversight:
  CR 502.2 restricts the untap *step*, and this is an untap *effect*.
- Flash: `keywords: [Flash]`, so the timing rule is the engine's; the existing
  test casts it in a main phase, and the new one during declare blockers, which
  only a flash creature could do.
- The Bell-Ringer untaps itself? It enters untapped, so there is nothing to do;
  the filter is `o.tapped`, so it is simply not in the list.
- Self-cleanup: none; this is a permanent.

### Noted, not acted on
The untap step pushes `GameEvent::Untapped { object }`; this card, and the five
other cards that untap (`grimgrin_corpse_born`, `civilized_scholar`,
`traitorous_blood`, `spidery_grasp`, `galvanic_juggernaut`), write
`obj.tapped = false` directly and emit nothing. That is the shape that made
`mill_one` necessary — but `GameEvent::Untapped` is consumed **nowhere**: no
`TriggerKind` watches it and no card reads it, so nothing in the pool can
observe the difference. Building an untap pipeline for a rule no card observes
would be speculative plumbing; recorded here so the next card that triggers on
untapping starts from a known position.

### UI presentation
Trigger description: "untap all creatures you control". No choices.

### Test coverage
- Untaps all your creatures, not the opponent's, and not your non-creatures:
  `cards_death_triggers_and_tokens.rs`
  (`village_bell_ringer_untaps_creatures`) — **the non-creature row added this audit**.
- The ruling: (`village_bell_ringer_leaves_an_untapped_attacker_in_combat`) —
  **added this audit**.
- Flash timing: the engine's; exercised incidentally by the new test, which
  casts it during the declare-blockers step.

### Mutations run
| mutation | result |
| --- | --- |
| untap every tapped permanent you control | fails the extended test (before: **nothing at all**) |
| remove each untapped creature from combat | fails the new ruling test (before: **nothing at all**) |
| untap every creature, not just yours | fails the extended test |
| untap only the first creature | fails the extended test |

Suite after: 1460 passing, exit 0, zero warnings.

