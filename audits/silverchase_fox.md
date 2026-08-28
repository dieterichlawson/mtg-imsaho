## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/31/silverchase-fox?utm_source=api
**Type line**: `Creature — Fox` — {1}{W}, 2/2
**Oracle text**:
```
{1}{W}, Sacrifice this creature: Exile target enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{W}, **Sacrifice this creature**: Exile target enchantment" — the
  sacrifice is a cost, paid on activation, so the Fox is in the graveyard while
  the ability is on the stack: PASS
- **Exile**, not destroy, so indestructible does not save the enchantment and it
  does not reach a graveyard: PASS
- "target enchantment" includes an Aura or a Curse: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The sacrifice cost and the exile: `cards_sacrifice_and_additional_costs.rs`, `sacrifice_choice.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/31/silverchase-fox?utm_source=api
**Type line**: `Creature — Fox` — {1}{W}, 2/2
**Oracle text**:
```
{1}{W}, Sacrifice this creature: Exile target enchantment.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `{1}{W}, Sacrifice this creature: Exile target enchantment.`
**Type line**: `Creature — Fox` — {1}{W}, 2/2
**Status**: ISSUE (fixed) — engine-level, found through this card

### Rulings
None on Scryfall.

### Code issues

- `mtg-engine/src/stack.rs:180` — CR 608.2b was applied to an ability's *card* but not to the requirement the ability declared.
  - `resolve_spell` re-checks a spell's targets against `target_requirement`, and `resolve_next_trigger` does the same for a trigger. The `StackEntry::Ability` arm checked only `can_be_targeted_by` (hexproof, protection) and the card's own `is_valid_target`. Its own comment admitted the consequence: `Olivia's own guard caught this one; a card without a guard did not.`
  - The requirement could not be looked up at resolution, which is why it was not: paying a `SacrificeThis` cost removes the source, and its `activated_abilities` list with it. It now rides on the stack entry, read before the cost is paid (CR 601.2c), beside `x_value` and `sacrificed`. `is_target_legal` gained the source object, which it needs for a `SameNameAsSource` filter — without it, Evil Twin's granted "destroy target creature with the same name" would have re-checked as always-illegal.

- `mtg-engine/src/cards/isd/silverchase_fox.rs:47` — the card carried the missing check, in the wrong method.
  - Oracle text says: `Exile target enchantment.`
  - Code did: `if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) { ... }`, inside `resolve_activated_ability`.
  - So an enchantment that left the battlefield in response made the ability **resolve and do nothing**, where CR 608.2b says it is countered by game rules. The board ended up the same, by the wrong route — and the route mattered: with the guard removed and nothing else checking, the ability exiles the enchantment *out of the graveyard*. That is what the new test measures, and what the mutation shows.

This was noted as deferred during an earlier audit in this run — "Elder of Laurels and Silverchase Fox still guard their target inside `resolve_activated_ability` rather than `is_valid_target`". The resolution is better than the note anticipated: the check belongs in neither, because the engine can do it for every card at once.

Everything else is right: `{1}{W}`, Creature — Fox, 2/2, oracle text verbatim, `{1}{W}` + `SacrificeCost::SacrificeThis`, and `PermanentWithFilter(HasCardType([Enchantment]))` for "target enchantment" — no controller restriction, correctly.

### Tricky interactions checked

- The Fox is sacrificed as a cost, so it is gone before the ability resolves: PASS, and this is exactly why the requirement had to be captured at activation rather than looked up later. The new test asserts the premise.
- "target enchantment", either player's: PASS — no controller filter.
- CR 608.2b when the enchantment leaves in response: PASS. Wrong route until this audit.
- Exile, not destroy: PASS — `move_object(Zone::Exile)`, so indestructible and regeneration are irrelevant, which is the point of the card.
- Hexproof on the enchantment: PASS, `can_be_targeted_by`, unchanged.
- The sacrifice happens even if the ability fizzles: PASS — a cost is paid on activation (CR 601.2h) and is not refunded.
- Evil Twin's `SameNameAsSource` under the new re-check: PASS, via the threaded source id. This was the one thing the change could plausibly have broken, and it is why `is_target_legal` grew the parameter rather than passing `None`.

### Test coverage

- Exiles an opponent's enchantment: `cards_sacrifice_and_additional_costs.rs:56` `silverchase_fox_exiles_enchantment`
- The ability is countered when the enchantment leaves in response: `fizzle.rs:700` `an_abilitys_declared_requirement_is_rechecked_when_it_resolves`, added this audit
- The same rule via hexproof, on a card that does restate its guard: `fizzle.rs:638` `an_activated_abilitys_targets_are_rechecked_when_it_resolves`
- The declared cost is `{1}{W}` with no restriction flags: `card_data_invariants.rs:1706`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 revert the engine's requirement re-check | n/a (was the gap) | `an_abilitys_declared_requirement_is_rechecked_when_it_resolves` FAILED — the enchantment ends in **Exile** instead of the graveyard |

The mutation is the finding: with the card's guard removed and the engine not re-checking, Silverchase Fox exiles a card that is already in the graveyard. Before this audit the two wrongs cancelled — the card guarded, so nothing showed.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1493 passing (was 1492). `cargo check --workspace --all-targets` clean, zero warnings.
