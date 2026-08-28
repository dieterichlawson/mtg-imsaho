## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/5/bonds-of-faith?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{W}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "gets +2/+2 as long as it's a Human. **Otherwise**, it can't attack or block" —
  three conditional continuous effects: `AttachedHasSubtype("Human")` for the
  pump, `AttachedLacksSubtype("Human")` for both restrictions, so the two halves
  are mutually exclusive by construction: PASS
- Ruling: "Once the enchanted creature has been declared as an attacking or
  blocking creature, causing it to stop being a Human won't remove it from
  combat. It will lose the +2/+2 bonus, however." The P/T is re-evaluated live;
  the attack restriction is only consulted at declaration: PASS
- A Human Werewolf that transforms into a non-Human back face loses the pump and
  gains the restrictions in real time: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both halves and the transform interaction: `enchantments.rs`, `moonmist.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/5/bonds-of-faith?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{W}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

**Rulings fetched**:
- [2011-09-22] Once the enchanted creature has been declared as an attacking or blocking creature, causing it to stop being a Human won’t remove it from combat. It will lose the +2/+2 bonus, however.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```
**Type line**: `Enchantment — Aura` — {1}{W}
**Status**: ISSUE (fixed) — one untested ruling; the card is correct

### Ruling (2011-09-22)
"Once the enchanted creature has been declared as an attacking or blocking creature, causing it to stop being a Human won't remove it from combat. It will lose the +2/+2 bonus, however."

### Code issues

No issues. `{1}{W}`, `Enchantment — Aura`, oracle text verbatim, `TargetRequirement::Creature` for "Enchant creature", attachment through the shared `helpers::resolve_aura`, and the whole card expressed as three declared `ContinuousEffect::when` entries rather than any code: `+2/+2` under `AttachedHasSubtype("Human")`, and `PreventAttack` / `PreventBlock` under `AttachedLacksSubtype("Human")`. That is the general conditional-effect mechanism, so the "as long as" is re-read rather than snapshotted — which is what the ruling turns on.

The card is unusually well covered: every mutation I could make to it was caught by tests that already existed, several of them by four or five tests at once. The one claim nobody had written down was the ruling itself.

### Tricky interactions checked

- "as long as" is continuous, not a snapshot at attachment: PASS, `continuous_effects.rs:191` transforms Cloistered Youth out of being a Human and watches the bonus go.
- "Otherwise" — the two halves are exclusive and both directions hold: PASS, `continuous_effects.rs:23`.
- The ruling, a creature already attacking that stops being Human: PASS. Untested until this audit.
- The bonus applies to the *enchanted* creature, not the Aura: PASS — `EffectScope::Attached`, and `OnSelf` fails five tests.
- Enchant any creature, either player's: PASS, no controller restriction.
- A transformed Werewolf under Bonds: PASS, `bonds_of_faith_prevents_attack_on_transformed_werewolf`.
- "can't attack" against an effect that *forces* an attack: PASS, `bug_bp_forced_attack_respects_cant_attack` — a restriction beats a requirement (CR 508.1).

### Test coverage

- Both directions of the condition, P/T and both restrictions: `continuous_effects.rs:23` `bonds_of_faith_reads_the_condition_in_both_directions`
- Not a snapshot — the condition is re-read after a transform: `continuous_effects.rs:198` `bug_bonds_of_faith_snapshot_instead_of_continuous`
- The ruling — stays in combat, loses the bonus: `continuous_effects.rs:390` `bonds_of_faith_loses_its_bonus_mid_combat_without_removing_the_attacker`, added this audit
- Buffs a Human / locks a non-Human, through the card's own tests: `cards_morbid_and_ltb.rs:420` and neighbours
- A forced attack still respects "can't attack": `bug_bp_forced_attack_respects_cant_attack`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `+2/+2` -> `+3/+3` | 5 tests FAILED | (unchanged) |
| M2 drop `PreventAttack` | 4 tests FAILED | (unchanged) |
| M3 drop `PreventBlock` | 4 tests FAILED | (unchanged) |
| M4 pump scope `Attached` -> `OnSelf` | 5 tests FAILED | (unchanged) |
| M5 pump unconditional (survives the transform) | 2 tests FAILED | + the new ruling test FAILED |

One half of the new test is not falsifiable by any mutation of this card, and is recorded as such rather than presented as covered ground: "a creature already attacking is not pulled out of combat" holds because nothing consults `can_attack` after attackers are declared, so no change to Bonds of Faith can break it. It is there to hold the engine to CR 508.1 if that ever changes, which is the same reason `a_target_creature_that_stopped_being_a_creature_is_no_longer_legal` builds its state by hand.

Source restored from `/tmp/bof.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1494 passing (was 1493). `cargo check --workspace --all-targets` clean, zero warnings.
