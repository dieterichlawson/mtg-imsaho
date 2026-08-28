## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/72/runic-repetition?utm_source=api
**Type line**: `Sorcery` — {2}{U}
**Oracle text**:
```
Return target exiled card with flashback you own to your hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target exiled card **with flashback** **you own**" — all three: in exile,
  owned by the caster, and its card data declares a flashback cost: PASS
- CR 109.1 now keeps tokens out of the `ExileCard` enumeration engine-side: PASS
- The returned card's `cast_with_flashback` flag is cleared on the move, so it
  can be cast normally from hand and flashed back again later: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Returning an exiled flashback card: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/72/runic-repetition?utm_source=api
**Type line**: `Sorcery` — {2}{U}
**Oracle text**:
```
Return target exiled card with flashback you own to your hand.
```

**Rulings fetched**:
- [2011-09-22] The card could have been exiled for any reason, not just because it was cast using flashback.
- [2011-09-22] An effect that gives flashback to an instant or sorcery card in your graveyard stops applying once that card has left the stack. The card won’t have flashback while exiled and can’t be the target of Runic Repetition (unless it naturally has flashback).
- [2011-09-22] A card that’s exiled face down doesn’t have any characteristics or abilities, so it can’t be the target of Runic Repetition.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/72/runic-repetition
**Oracle text**:
```
Return target exiled card with flashback you own to your hand.
```
**Type line**: `Sorcery` · **Mana cost**: `{2}{U}`
**Rulings** (3, all 2011-09-22, https://api.scryfall.com/cards/53e47ba6-3a55-41b4-b8fe-580041669408/rulings):
1. "The card could have been exiled for any reason, not just because it was cast using flashback."
2. "An effect that gives flashback to an instant or sorcery card in your graveyard stops applying once that card
   has left the stack. The card won't have flashback while exiled and can't be the target of Runic Repetition
   (unless it naturally has flashback)."
3. "A card that's exiled face down doesn't have any characteristics or abilities, so it can't be the target of
   Runic Repetition."

**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `runic_repetition.rs` | |
|---|---|---|---|
| cost | `{2}{U}` | `Generic(2) + Blue` | ok |
| types | Sorcery | `vec![CardType::Sorcery]` | ok |
| oracle_text | as above | byte-identical | ok |
| targeting | exiled card with flashback you own | `ExileCard` + `is_valid_target` | ok |

### Code issues
No issues found — and one thing that looks like a shortcut but is the rule.

`is_valid_target` asks `state.face_data(o.id, registry).is_some_and(|d| d.flashback_cost.is_some())` — the
**printed** flashback cost, not "does this card have flashback right now". That is exactly what ruling 2
requires. Snapcaster Mage's grant is a `TemporaryEffect::GrantFlashback` on `until_end_of_turn`, which is still
sitting in that list while the card it named is in exile the same turn; asking the live view would wrongly make
that card targetable. This is the mirror image of the Maw of the Mire finding, where reading `face_data` instead
of the general helper was the *narrower*, wrong answer. Here narrow is right, and the difference is that the
oracle text is about a printed ability rather than a current characteristic.

- **Ruling 1** — nothing checks *why* the card was exiled, only that it is there. Correct.
- **Ruling 3** — face-down exile is not modelled in this engine and no ISD card creates it. Unreachable.
- **`on_resolve` has no zone guard**, and needs none: a single-target spell whose target left exile is countered
  by game rules before `on_resolve` runs (`is_target_legal`'s `ExileCard` arm checks the zone). Same reasoning
  as Maw of the Mire.
- **"to your hand"** — `move_object(target_id, Zone::Hand)` lands in the owner's hand, and "you own" makes owner
  and caster the same player.

### Changes made
Nothing in the card. `mtg-engine/tests/cards_lands_and_mana_sources.rs` gained two tests. The only existing
coverage was the positive case — an exiled Think Twice comes back — which an implementation ignoring both
restrictions also passes.

- `runic_repetition_targets_only_your_own_exiled_flashback_cards` — a card without flashback and an opponent's
  exiled flashback card are both refused, with the legal one offered from the same board.
- `runic_repetition_ignores_flashback_that_was_only_granted` — ruling 2, with a control showing a naturally
  flashback card is still offered from that same board.

### Mutation checks
1. Flashback restriction dropped → both new tests FAILED. **Discriminating.**
2. `is_valid_target` widened to also accept a `GrantFlashback` temporary effect — the exact misreading ruling 2
   warns about → `runic_repetition_ignores_flashback_that_was_only_granted` FAILED. **Discriminating.**
3. Owner restriction dropped from `is_valid_target` → **vacuous.** The `ExileCard` enumeration already filters
   `o.owner == caster` (`targeting.rs:384`, and the ability path at `:602`), and ownership never changes
   (CR 108.3), so that half of the card's guard is redundant everywhere while the flashback half is
   load-bearing. Left in place rather than splitting the expression for no behavioural gain — recorded here so
   the redundancy is known rather than rediscovered.

### Tricky interactions checked
- Returns an exiled flashback card you own: **pass** (`cards_lands_and_mana_sources.rs:486`).
- A card with no flashback is not a legal target: **pass** (new).
- An opponent's exiled flashback card is not a legal target: **pass** (new; enforced by the requirement).
- Granted flashback does not make a card targetable in exile: **pass** (new).
- The returned card's `cast_with_flashback` flag is cleared, so a later normal cast goes to the graveyard rather
  than exile: **pass** (`flashback.rs:505`).
- Target leaves exile in response → spell countered by game rules: by construction, not separately tested.

### Test coverage
- returns the card: `cards_lands_and_mana_sources.rs:486`
- both restrictions: `cards_lands_and_mana_sources.rs:511` (new)
- ruling 2, granted flashback: `cards_lands_and_mana_sources.rs:545` (new)
- stale flashback flag cleared on return: `flashback.rs:505`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1427 passing.

