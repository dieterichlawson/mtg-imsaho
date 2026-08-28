## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/142/feral-ridgewolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 1/2
**Oracle text**:
```
Trample
{1}{R}: This creature gets +2/+0 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- No activation limit, so it stacks with itself: PASS
- Trample is printed, not granted by the ability: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Stacking activations: `activated_abilities.rs:an_unrestricted_pump_ability_stacks_with_itself`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/142/feral-ridgewolf?utm_source=api
**Type line**: `Creature — Wolf` — {2}{R}, 1/2
**Oracle text**:
```
Trample
{1}{R}: This creature gets +2/+0 until end of turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Trample
{1}{R}: This creature gets +2/+0 until end of turn.
```
**Type line**: `Creature — Wolf` — {2}{R}, 1/2, Trample
**Status**: ISSUE (fixed) — one test gap, closed pool-wide; the card is correct

### Rulings
None on Scryfall.

### Code issues

No issues in the card. `{2}{R}`, Creature — Wolf, 1/2, `Keyword::Trample`, oracle text verbatim, and one `ActivatedAbilityDef` at `{1}{R}` with no tap, no target, no restriction — correctly, since unlike its shelf-mate Darkthicket Wolf this one prints no "Activate only once each turn". The pump is a `TemporaryEffect::ModifyPT` in `until_end_of_turn`.

One gap, and it was the keyword: **deleting `Keyword::Trample` passed the entire workspace**, and so did adding a Flying the card does not have. Neither of the card's two tests touches combat. That is the general shape of the thing — a keyword is only ever exercised by the combat or targeting scenario that happens to need it, so a wrong one is invisible in every test that does not stage that scenario.

Closed pool-wide rather than per card: `card_data_invariants.rs::keywords_say_what_scryfall_says` compares the fifteen keywords `Keyword` models against the checked-in oracle cache in both directions, over 139 cards. It deliberately ignores what Scryfall lists that the engine models elsewhere — Flashback as `flashback_cost`, Protection and Enchant as continuous effects, Transform as `back_face_data`, and the keyword *actions* (Mill, Fight, Proliferate) — because reading those as missing keywords would be misreading the cache rather than the card. Scryfall's list is card-level and covers both faces, so a keyword counts as declared if either face carries it.

This completes the printed-characteristics cross-check begun two cards ago: rules text and back face were already pinned; type line, mana cost and P/T were added during the Selfless Cathar audit; activated-ability costs during Darkthicket Wolf's; keywords here.

### Tricky interactions checked

- No "once each turn" restriction, unlike Darkthicket Wolf: PASS, and the contrast is the subject of `activated_abilities.rs:110`, where paying twice pumps twice.
- "+2/+0", not +2/+2: PASS, tested.
- "until end of turn": PASS, through `until_end_of_turn` so the engine's cleanup removes it.
- The pump lands on this creature: PASS, tested.
- Trample declared: PASS. Untested until this audit.
- Trample's *behaviour* (excess damage assigned to the defending player): engine-general, exercised by the combat tests rather than per card; not duplicated here.
- Redundant `zone == Battlefield` gate in `activated_abilities`: present, left alone — one of the 29 recorded in the Mirror-Mad Phantasm entry.

### Test coverage

- One activation makes it a 3/2: `activated_abilities.rs:26` `a_pump_ability_changes_the_creature_it_is_activated_on` (table row)
- No restriction, so it stacks with itself: `activated_abilities.rs:110` `an_unrestricted_pump_ability_stacks_with_itself`
- The declared cost is `{1}{R}` and no restriction flag is set: `card_data_invariants.rs:1706` (added during the Darkthicket Wolf audit)
- Trample is printed and declared: `card_data_invariants.rs:1790` `keywords_say_what_scryfall_says`, added this audit (139 cards)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 delete `Keyword::Trample` | passed whole workspace | `keywords_say_what_scryfall_says` FAILED |
| M2 add a spurious `Keyword::Flying` | passed whole workspace | `keywords_say_what_scryfall_says` FAILED |
| M3 `+2/+0` -> `+2/+2` | `a_pump_ability_changes_the_creature_it_is_activated_on` FAILED | (unchanged) |
| M4 cost `{1}{R}` -> `{2}` | (already caught by the invariant added for Darkthicket Wolf) | FAILED |
| M5 `once_per_turn: true` | `an_unrestricted_pump_ability_stacks_with_itself` FAILED | + the cost invariant FAILED |
| M6 pump `ObjectId(0)` instead of this creature | n/a | 2 tests FAILED |

M1's first attempt did not compile — removing the `keywords` line left the `Keyword` import unused, which `warnings = "deny"` rejects. Recorded because a mutation that fails to compile proves nothing; it was redone as `Vec::<Keyword>::new()` and only then counted.

Source restored from `/tmp/fr.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1485 passing (was 1484). `cargo check --workspace --all-targets` clean, zero warnings.
