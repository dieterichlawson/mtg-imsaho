## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/204/spidery-grasp?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Spidery Grasp can target a creature that's already untapped. It will
  still get +2/+4 and gain reach" — the untap is not a condition: PASS
- Untapping an attacking creature does not remove it from combat: PASS
- Reach until end of turn lets it block a flier this turn only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The untap, pump and reach: `cards_pump_spells.rs`, `evasion.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/204/spidery-grasp?utm_source=api
**Type line**: `Instant` — {2}{G}
**Oracle text**:
```
Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)
```

**Rulings fetched**:
- [2011-09-22] Spidery Grasp can target a creature that’s already untapped. It will still get +2/+4 and gain reach.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)`
**Type line**: `Instant` — {2}{G}
**Status**: ISSUE (fixed) — two test gaps; the card is correct

### Ruling (2011-09-22)
"Spidery Grasp can target a creature that's already untapped. It will still get +2/+4 and gain reach."

### Code issues

No issues in the card. `{2}{G}`, Instant, oracle text verbatim, `TargetRequirement::Creature` for "target creature" with no restriction — which is what the ruling turns on — the untap through `state.untap` (the CR 701.20a helper added during the Traitorous Blood audit), and both grants as `until_end_of_turn` temporary effects.

Two gaps, and they are the two things the card says beyond "pump a creature":

- **The ruling.** Making the whole effect conditional on the creature having been tapped passed the entire workspace. Every existing test cast it on a tapped creature — which is precisely the case the ruling exists to distinguish from.
- **"until end of turn".** Granting the +2/+4 and the reach permanently, through `instance_continuous_effects`, also passed the whole workspace. Same class as the Kessig Wolf finding: a duration is a claim about the *next* turn, and nothing looked there.

Both are now one test, since they share a board: cast on an untapped creature, assert both halves land, advance a real turn, assert both are gone.

### Tricky interactions checked

- Ruling, an already-untapped target: PASS. Untested until this audit.
- "until end of turn" for both the P/T and the keyword: PASS. Untested until this audit.
- The untap goes through the pipeline rather than writing `tapped`: PASS, and enforced by `only_the_untap_helper_untaps_a_permanent`.
- "target creature", either player's: PASS, no controller restriction — untapping an opponent's creature is legal and occasionally what you want.
- Reach specifically, not some other evasion-answering keyword: PASS, granting Trample instead fails.
- Instant speed, so it can be cast after blockers: PASS by card type; the reminder text "(It can block creatures with flying.)" is why the card exists.
- Redundant `zone == Battlefield` preamble in `on_resolve`: present, and now doubly redundant — CR 608.2b re-checks battlefield *and* creature-ness for a bare `Creature` requirement since the Traitorous Blood audit. Left alone, consistent with the other cards in this run.

### Test coverage

- Untaps, +2/+4, reach, on a tapped creature: `cards_vanilla_and_keywords.rs:108` `spidery_grasp_untaps_and_buffs`
- The ruling, and both grants wearing off: `cards_vanilla_and_keywords.rs:126` `spidery_grasp_works_on_an_untapped_creature_and_wears_off`, added this audit
- Cost and type line: `card_data_invariants.rs`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `+2/+4` -> `+4/+2` | `spidery_grasp_untaps_and_buffs` FAILED | (unchanged) |
| M2 grant Trample instead of Reach | `spidery_grasp_untaps_and_buffs` FAILED | (unchanged) |
| M3 drop the untap | `spidery_grasp_untaps_and_buffs` FAILED | (unchanged) |
| M4 do nothing unless the creature was tapped | passed whole workspace | `spidery_grasp_works_on_an_untapped_creature_and_wears_off` FAILED |
| M5 permanent grants instead of until-end-of-turn | passed whole workspace | same test FAILED |

M2's first attempt did not compile: deleting the reach grant left the `TemporaryEffect` import unused, which `warnings = "deny"` rejects. Redone as a swap to Trample, which is a real way to write the card wrongly, and only then counted.

Source restored from `/tmp/sg.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1498 passing (was 1497). `cargo check --workspace --all-targets` clean, zero warnings.
