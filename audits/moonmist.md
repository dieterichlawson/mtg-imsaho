## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/195/moonmist?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
```
**Status**: PASS

### Code issues
No issues found.

- "Transform all Humans" — selects creatures with the Human subtype through
  `state.has_subtype` (so granted types count) that actually have a back face,
  since only a double-faced card can transform. Both directions: a back face
  that is still Human transforms too.
- The prevention half is a `PreventCombatDamageExcept` naming Werewolves and
  Wolves, verified in both directions by `moonmist.rs` including a control run
  without the prevention.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/195/moonmist?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Moonmist causes **any double-faced Human** to transform, not just
  Werewolves." The filter is "has the Human subtype on its **active** face and
  has a back face", so it also flips a back face that is still Human — Thraben
  Militia is the case in this set: PASS
- "Transform **all** Humans" — a non-double-faced Human is unaffected (CR 701.28c,
  and the reminder text says as much): PASS
- Ruling: "Whether or not a creature is a Werewolf or a Wolf is checked **only as
  combat damage is dealt**" — the prevention is a live check at damage time, not
  a snapshot of the board when Moonmist resolved: PASS
- Ruling: "Moonmist will prevent combat damage dealt by a creature that isn't a
  Werewolf or a Wolf **even if that creature wasn't on the battlefield** when
  Moonmist resolved": PASS
- The prevention is combat damage only, so a Geistflame still gets through: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Flipping Humans in both directions and the damage prevention: `moonmist.rs`, `werewolf_cards.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/195/moonmist?utm_source=api
**Type line**: `Instant` — {1}{G}
**Oracle text**:
```
Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
```

**Rulings fetched**:
- [2011-09-22] Moonmist causes any double-faced Human to transform, not just Werewolves.
- [2011-09-22] Whether or not a creature is a Werewolf or a Wolf is checked only as combat damage is dealt. If the creature isn’t a Werewolf or a Wolf at that time, its combat damage will be prevented.
- [2011-09-22] Moonmist will prevent combat damage dealt by a creature that isn’t a Werewolf or a Wolf even if that creature wasn’t on the battlefield (or was a Werewolf or a Wolf) when Moonmist resolved.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**: `Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)`
**Type line**: `Instant` — {1}{G}
**Status**: ISSUE (fixed) — engine-level, plus a test that tested a copy of the card

### Rulings (all 2011-09-22)
1. "Moonmist causes any double-faced Human to transform, not just Werewolves."
2. "Whether or not a creature is a Werewolf or a Wolf is checked only as combat damage is dealt. If the creature isn't a Werewolf or a Wolf at that time, its combat damage will be prevented."
3. "Moonmist will prevent combat damage dealt by a creature that isn't a Werewolf or a Wolf even if that creature wasn't on the battlefield (or was a Werewolf or a Wolf) when Moonmist resolved."

### Code issues

- `mtg-engine/src/cards/helpers.rs:539` — `apply_transform` would transform a single-faced permanent.
  - Oracle text says: `(Only double-faced cards can be transformed.)` — CR 701.28c.
  - `apply_transform` refused a token copy of a DFC (CR 111.7) and then set `obj.is_transformed = !was_transformed` for anything else, back face or not. A single-faced Human flipped to `is_transformed = true`: a permanent claiming to show a face it does not have.
  - Moonmist filtered for `back_face_data().is_some()` itself before asking, which made *this* card right and left the function willing to corrupt anything else that asked. The check moved into `apply_transform` — which Moonmist's own comment already calls "the one place that knows what transforming means" — and the card stopped second-guessing it.
  - The comment that had kept the check out ("Some DFCs (Garruk Relentless) model their back face by branching on `is_transformed` … instead of declaring `back_face_data`") is no longer true; Garruk declares one, and `every_card_with_a_back_face_declares_it` is what holds every DFC that way.

- `mtg-engine/tests/moonmist.rs:14` — the combat table tested a copy of the card, not the card.
  - The helper built the prevention effect itself, under a comment claiming it was `Built here the way the card builds it, so the test exercises the same filter`. It was a duplicate, so it exercised the duplicate: **dropping `Werewolf` from the card's filter passed every row**, and it took me two attempts to see why the mutation kept passing.
  - It now casts Moonmist. And "other than Werewolves **and Wolves**" gets both exceptions asserted, because the single Wolf row that existed was satisfied by a filter naming only Wolves.

The card is otherwise right: `{1}{G}`, Instant, oracle text verbatim, transform through `apply_transform`, and the prevention as a `TemporaryEffect::PreventCombatDamageExcept` carrying the two subtypes the card names — a live filter rather than a snapshot, which is what rulings 2 and 3 require.

### Tricky interactions checked

- Ruling 1, any double-faced Human and not only Werewolves: PASS — the filter is `has_subtype("Human")` with no Werewolf condition.
- A back face that is still a Human (Thraben Militia) transforms back: PASS, `moonmist.rs:119`.
- A Werewolf already showing its back face is not a Human, so a second Moonmist leaves it alone: PASS, `moonmist.rs:173` — "transform all Humans" is not "transform all werewolf cards".
- Ruling 3, a creature that arrives after Moonmist resolved is still prevented: PASS, and structurally — the spell resolves before the attacker is created in every row of the table.
- Ruling 2, the check happens as damage is dealt: PASS by construction, since the effect stores a filter rather than a list of creatures. Not separately tested: nothing in this pool changes a creature's subtypes between Moonmist resolving and combat damage, so the snapshot and the live reading cannot be told apart.
- Both exceptions, Wolves and Werewolves: PASS. Untested until this audit.
- Tokens do not transform (CR 111.7): PASS, in `apply_transform`.
- Prevention applies to both players' creatures, not just the caster's: PASS — the filter names subtypes only. Covered by the blocked row, where the opponent's blocker is also prevented.

### Test coverage

- The prevention is put up: `moonmist.rs:13` `sets_prevention_flag`
- Prevented and control rows, unblocked and blocked, both directions of a block: `moonmist.rs:36` `moonmist_prevents_combat_damage_from_everything_but_wolves`
- A Wolf and a Werewolf are both spared: same test, Werewolf row added this audit
- Ruling 3: same test, structurally
- Front-face Human transforms: `moonmist.rs:89` `transforms_front_face_human`
- Back-face Human transforms back: `moonmist.rs:108` `transforms_back_face_human`
- A non-DFC Human does not: `moonmist.rs:131` `does_not_transform_non_dfc_human` — now for the engine's reason
- Ruling 1 / "Humans right now": `moonmist.rs:162` `moonmist_only_transforms_whatever_is_a_human_right_now`

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 transform every DFC, not just Humans | `moonmist_only_transforms_whatever_is_a_human_right_now` FAILED | (unchanged) |
| M2 prevention filter names only Wolves | passed whole workspace | `moonmist_prevents_combat_damage_from_everything_but_wolves` FAILED |
| M3 remove the new CR 701.28c guard from `apply_transform` | n/a | `does_not_transform_non_dfc_human` FAILED |

M2 is the one worth recording properly. My first attempt at closing it — adding a Village Ironsmith row to the existing table — **still passed the mutation**, and the reason was not the row: the test built the prevention effect itself instead of casting the card, so mutating the card could not reach it. The row only started biting once the helper cast Moonmist. A test that duplicates the thing it is testing cannot fail for the right reason, and I nearly recorded a fix that had not fixed anything.

Also recorded: I twice restored `helpers.rs` from a backup taken *before* my own change, silently reverting the fix. The second time `does_not_transform_non_dfc_human` caught it. Backups here need to be taken after the fix, not before.

Sources restored from `/tmp/mm3.bak` and `/tmp/help2.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1493 passing. `cargo check --workspace --all-targets` clean, zero warnings.
