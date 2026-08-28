## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/106/manor-skeleton?utm_source=api
**Type line**: `Creature — Skeleton` — {1}{B}, 1/1
**Oracle text**:
```
Haste
{1}{B}: Regenerate this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{B}: Regenerate this creature" — a regeneration shield, which taps the
  creature, removes its damage and removes it from combat when it applies
  (CR 701.15): PASS
- Haste, so it can attack the turn it arrives: PASS
- Shields stack, so two activations survive two lethal events: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Regeneration and haste: `cards_morbid_and_ltb.rs`, `activated_abilities.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/106/manor-skeleton?utm_source=api
**Type line**: `Creature — Skeleton` — {1}{B}, 1/1
**Oracle text**:
```
Haste
{1}{B}: Regenerate this creature.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Haste
{1}{B}: Regenerate this creature.
```
**Type line**: `Creature — Skeleton` — {1}{B}, 1/1, Haste
**Status**: ISSUE (fixed) — one test gap; the card is correct

### Rulings
None on Scryfall.

### Code issues

No issues in the card. `{1}{B}`, Creature — Skeleton, 1/1, `Keyword::Haste`, oracle text verbatim, one `ActivatedAbilityDef` at `{1}{B}` with no tap, no target and no restriction, and the regeneration through `state.add_regeneration_shield` — the shared pipeline, so the shield behaves per CR 701.15 without the card knowing what that means.

The gap was haste. Dropping `Keyword::Haste` is caught by the keyword invariant added during the Feral Ridgewolf audit, but only as a *declaration* — nothing asked whether a **printed** haste reaches combat. `keywords.rs::haste_overrides_summoning_sickness` granted haste through `until_end_of_turn` and carried the comment "We don't have a haste creature card yet, so manually set the keyword", stale since this card was implemented.

The two roads are genuinely different, which is why this matters rather than being a duplicate: `has_keyword` deliberately ignores `obj.keywords` for a card with a registry entry, so a printed keyword is read off the active face and a granted one off the effects layer. Only the granted road was tested. Manor Skeleton now covers the printed one, with a plain summoning-sick creature beside it as the control.

### Tricky interactions checked

- Regeneration is a shield, not a prevention: PASS, `add_regeneration_shield`, and what the shield then does is tested as a rule in `regeneration.rs` — tap, remove from combat, remove all damage (CR 701.15a), expire at cleanup, not saving from 0 toughness, not saving from a sacrifice.
- The shield goes on this creature: PASS, tested.
- Repeatable, so two activations are two shields: PASS by the absence of a restriction, which the cost invariant pins to the text; the two-shield behaviour is `regeneration.rs:57`.
- Printed haste lets it attack the turn it arrives: PASS. Untested until this audit.
- Instant speed, so the shield can be put up in response to removal: PASS, `sorcery_speed_only: false`, pinned by the cost invariant.
- Regeneration does not stop exile or sacrifice: PASS, `regeneration.rs:154` — and this card says "regenerate", which is what the code does, rather than "prevent" or "indestructible".
- Redundant `zone == Battlefield` gate in `activated_abilities`: present, left alone — one of the 29 recorded in the Mirror-Mad Phantasm entry.

### Test coverage

- Activating places a shield, and the shield saves it from lethal damage and clears the damage: `activated_abilities.rs:81` `manor_skeleton_regenerates_out_of_lethal_damage`
- What a shield does, as a rule: `regeneration.rs` — nine tests covering lethal damage, zero toughness, multiple shields, cleanup expiry, `try_destroy`, sacrifice, and deathtouch
- Printed haste reaches `eligible_attackers`: `keywords.rs:130` `printed_haste_overrides_summoning_sickness`, added this audit
- Haste is printed on the card: `card_data_invariants.rs:1790` (added during the Feral Ridgewolf audit)
- The declared cost is `{1}{B}` with no restriction flags: `card_data_invariants.rs:1706` (added during the Darkthicket Wolf audit)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 place no shield | `manor_skeleton_regenerates_out_of_lethal_damage` FAILED | (unchanged) |
| M2 shield on `ObjectId(0)` | same test FAILED | (unchanged) |
| M3 drop printed `Keyword::Haste` | only `keywords_say_what_scryfall_says` FAILED — the declaration, not the behaviour | + `printed_haste_overrides_summoning_sickness` FAILED |

M1's first attempt did not compile — deleting the call left `state` unused, which `warnings = "deny"` rejects. Recorded because a mutation that fails to compile proves nothing; it was redone with a binding and only then counted.

Source restored from `/tmp/ms2.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1492 passing (was 1491). `cargo check --workspace --all-targets` clean, zero warnings.
