## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/24/moment-of-heroism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- +2/+2 and lifelink until end of turn, both as `TemporaryEffect`s so they
  expire together: PASS
- Ruling: "Multiple instances of lifelink on the same creature are redundant":
  PASS
- Lifelink applies to all damage the creature deals, not only combat damage: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the lifelink: `cards_pump_spells.rs`, `keywords_lifelink.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/24/moment-of-heroism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
```

**Rulings fetched**:
- [2019-07-12] Multiple instances of lifelink on the same creature are redundant.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)`
**Type line**: `Instant` — {1}{W}
**Status**: ISSUE (fixed) — one test gap; the card is correct

### Ruling (2019-07-12)
"Multiple instances of lifelink on the same creature are redundant." — CR 702.15b, and the engine's: `has_keyword` is a boolean, so a second grant changes nothing.

### Code issues

No issues in the card. `{1}{W}`, Instant, oracle text verbatim, `TargetRequirement::Creature` for "target creature" with no restriction, and both halves as `until_end_of_turn` temporary effects.

The card's own test is one of the stronger ones in the suite — it asserts the pump, the keyword, and both expiring — and every mutation of the card is caught by it. The gap was one step further out.

**`has_keyword` reads three separate places**: the printed face, a permanent's continuous effects, and `until_end_of_turn`. Two of the three were followed all the way to the life gain — Markov Patrician's printed lifelink, and Butcher's Cleaver's granted one, whose test already articulates the principle: "That the keyword is granted and that the combat damage step honours a keyword granted by *another permanent* are two different claims, and only the second is what the card promises." The third road — a spell's until-end-of-turn grant, which is this card's — stopped at `has_keyword`. Granting Vigilance instead of Lifelink was caught only by the `has_keyword` assertion, never by a life total.

This is the same shape as the Manor Skeleton finding two audits back, where printed haste reached `has_keyword` but nothing asked whether it reached combat.

### Tricky interactions checked

- The ruling, redundant lifelink: PASS trivially — `has_keyword` is a boolean.
- "until end of turn" for both halves: PASS, tested, and a permanent-grant mutation fails.
- Granted lifelink actually gains life in combat: PASS. Untested until this audit.
- CR 702.15a, lifelink on *any* damage rather than only combat: covered for the continuous-effect road at `keywords.rs:359`; not duplicated for this card, which has no way to deal noncombat damage of its own.
- "target creature", either player's: PASS, no restriction — pumping an opponent's blocker is legal and occasionally right.
- Redundant `zone == Battlefield` preamble: present, left alone, consistent with the rest of this run.

### Test coverage

- Pump, keyword, and both expiring at end of turn: `keywords.rs:592` `spell_grants_keyword_until_eot`
- The granted lifelink gains life in combat: `keywords.rs:325` `lifelink_granted_until_end_of_turn_gains_life_in_combat`, added this audit
- The other two roads to lifelink: `keywords.rs:290`, `:308` (printed), `:331`, `:359` (continuous-effect grant)
- Cost and type line: `card_data_invariants.rs`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 `+2/+2` -> `+3/+3` | `spell_grants_keyword_until_eot` FAILED | (unchanged) |
| M2 grant Vigilance instead of Lifelink | `spell_grants_keyword_until_eot` FAILED — on `has_keyword` alone | + `lifelink_granted_until_end_of_turn_gains_life_in_combat` FAILED, on the life total |
| M3 permanent grants instead of until-end-of-turn | `spell_grants_keyword_until_eot` FAILED | (unchanged) |

M2 is the finding, and it is a finer point than the usual "passed the whole workspace": the mutation *was* caught, but only by an assertion that the keyword was written down — not by anything that used it. A card that grants a keyword promises the keyword's effect, and that is a separate claim.

Source restored from `/tmp/moh.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1503 passing (was 1502). `cargo check --workspace --all-targets` clean, zero warnings.
