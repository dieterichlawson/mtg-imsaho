## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/103/gruesome-deformity?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Intimidate is the printed keyword — "can't be blocked except by artifact
  creatures and/or creatures that share a color with it" — and not menace: PASS
- The evasion is evaluated against the blocker's colours at declare-blockers: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Intimidate blocking: `evasion.rs`, `enchantments.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/103/gruesome-deformity?utm_source=api
**Type line**: `Enchantment — Aura` — {B}
**Oracle text**:
```
Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{B}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", one continuous effect
`GrantKeyword { keyword: Intimidate, scope: Attached }`. Cost, type line and
subtypes pinned pool-wide by `card_data_invariants.rs`, and the `Intimidate`
keyword is one the keyword invariant *does* model, so the grant is pinned to
the printed word as well. `resolve_aura` for attachment, no `is_valid_target`
override, no card-side cleanup.

The intimidate rule itself lives in `combat::can_block_attacker` and reads
`state.colors_of(attacker_id, ..)` — the *attacker's* colours, which is the
enchanted creature's, not the Aura's.

### Tricky interactions checked

- **Whose colour intimidate compares.** Gruesome Deformity is black and grants
  the keyword to whatever it enchants, so a white creature wearing it is
  blockable by white creatures and not by black ones. The engine gets this
  right; nothing pinned it, and reading the Aura's colour instead gives
  exactly the opposite answer on that board. Untested until now.
- **A colourless host.** Colourless is not a colour (CR 105.1), so a
  colourless creature shares one with nothing and the Aura leaves it blockable
  only by artifact creatures. Pass, untested until now — and reachable here
  because One-Eyed Scarecrow is both the colourless attacker and the legal
  blocker.
- Artifact creatures block regardless of colour: pass, covered for the rule by
  `keywords.rs::artifact_creature_blocks_intimidate`.
- The keyword ends with the Aura: `EffectScope::Attached` and the same two
  guards recorded under Spectral Flight.
- CR 704.5m and fizzle: engine-level, covered for the Aura shape.

### Test coverage

- grants the keyword: `cards_vanilla_and_keywords.rs::gruesome_deformity_grants_intimidate`
- the colour compared is the host's, not the Aura's:
  `cards_vanilla_and_keywords.rs::gruesome_deformity_reads_the_creatures_colour_not_its_own` (new)
- a colourless host is blockable only by artifacts:
  `cards_vanilla_and_keywords.rs::gruesome_deformity_on_a_colourless_creature_leaves_only_artifacts` (new)
- the intimidate rule itself: `keywords.rs::intimidate_blocks_different_color`,
  `keywords.rs::artifact_creature_blocks_intimidate` — both through a creature
  that *prints* the keyword (Spectral Rider) and with `obj.colors` written by
  hand, which is why the granted case with real printed colours was worth
  adding.

### Mutations run

- `can_block_attacker` compares the colours of whatever is attached to the
  attacker instead of the attacker's own: **fails** both new tests, passes the
  accessor test.
- Artifact creatures lose the exemption: **fails** the colourless test, and
  only that one — the white-host test does not depend on it.
- The card grants Vigilance instead of Intimidate: **fails** all three.

Suite: 1545 passing, exit 0, `cargo check --workspace --all-targets` clean.
