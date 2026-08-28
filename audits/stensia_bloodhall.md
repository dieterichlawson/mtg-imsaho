## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/247/stensia-bloodhall?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "Like other lands, Stensia Bloodhall is colorless. The damage it deals
  is from a colorless source, even though activating its ability requires
  colored mana." The damage source is the land object, so protection from black
  or red does not stop it: PASS
- "target player **or planeswalker**" — `TargetRequirement::PlayerOrPlaneswalker`,
  so it cannot hit a creature: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage ability: `cards_activated_abilities.rs`
- Its mana ability is still offered alongside: `mana_ability_offers.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/247/stensia-bloodhall?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
```

**Rulings fetched**:
- [2011-09-22] Like other lands, Stensia Bloodhall is colorless. The damage it deals is from a colorless source, even though activating its ability requires colored mana.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/247/stensia-bloodhall
**Oracle text**:
```
{T}: Add {C}.
{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
```
**Type line**: `Land` · **Mana cost**: none
**Ruling** (2011-09-22, https://api.scryfall.com/cards/cc2741d8-2c02-4acd-8ca2-55b4bf6aef1c/rulings):
"Like other lands, Stensia Bloodhall is colorless. The damage it deals is from a colorless source, even though
activating its ability requires colored mana."

**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `stensia_bloodhall.rs` | |
|---|---|---|---|
| name / cost / types | Stensia Bloodhall, none, Land | matching | ok |
| oracle_text | as above | byte-identical | ok |
| mana ability | `{T}: Add {C}` | `Colorless`, tap, free | ok |
| activation cost | `{3}{B}{R}, {T}` | `Generic(3) + Black + Red`, `requires_tap: true` | ok |
| targeting | "target player or planeswalker" | `TargetRequirement::PlayerOrPlaneswalker` | ok |
| timing | unrestricted | `sorcery_speed_only: false` | ok |

### Code issues
No issues found.

The damage goes through `PendingEffect::DealDamage { amount: 2, source_id: object_id, .. }` and
`engine::apply_pending_effect`, i.e. the shared pipeline, so the planeswalker branch (CR 120.3c — remove
loyalty, mark no damage) is the engine's and not restated here. `source_id` is the land, which is what makes
the ruling true.

### Rules check
- **CR 702.11b** (player hexproof): handled by `can_target_player`, which the `PlayerOrPlaneswalker`
  enumeration filters through, and which stops opponents only.
- **CR 104.3a**: same function drops a player who has lost.
- **CR 602.2h** (one tap pays one cost): the Bloodhall's own `{T}: Add {C}` cannot fund its `{3}{B}{R}` —
  covered by the utility-land table in `tap_cost_legality.rs`.
- **CR 302.6**: not applicable to a land; the engine applies it anyway and the card no longer restates any of
  this (the redundant zone/tapped guard came out during the Gavony Township audit).
- **The ruling**: the land's colour comes from its own characteristics, and a land with no mana cost is
  colourless. Nothing derives colour from an ability's cost.

### Changes made
Nothing in the card. `mtg-engine/tests/cards_activated_abilities.rs` gained three tests plus two small helpers.

- `stensia_bloodhall_cannot_point_at_a_creature` — "target player **or planeswalker**" excludes creatures.
  `damage_helper.rs` covers the opposite direction (an ability saying "another target creature" cannot resolve
  against a planeswalker); this is the half where the creature is the illegal one. Both legal kinds are also
  asserted, so the test is not passed by an ability that is simply unavailable.
- `stensia_bloodhall_cannot_target_a_player_with_hexproof` — Witchbane Orb, and the control that the Orb's own
  controller can still target themselves. This is the set's one activated ability that targets a player, so it
  is the only place this can be exercised from an ability rather than a spell.
- `stensia_bloodhall_is_a_colorless_source` — the ruling.

**On the ruling's reachability, stated plainly.** The ruling's practical consequence would be protection from
black or red failing to stop the damage. That is not reachable in this set: no card here grants a player
protection from a colour (`grants_player_protection_from` has no implementor in `cards/isd/`), and
`player_has_protection_from` is consulted only by `player_can_be_enchanted_by`, for Auras (CR 702.16b). So the
test asserts the ruling itself — `colors_of(bloodhall)` is empty despite `{B}{R}` in the ability's cost —
rather than staging an interaction that cannot occur. What it actually guards is a permanent's colour being
derived from its activation cost, which is the mistake the ruling exists to pre-empt.

### Mutation checks (all discriminating)
1. `PlayerOrPlaneswalker` → `AnyTarget` → `stensia_bloodhall_cannot_point_at_a_creature` FAILED.
2. Dropped the hexproof branch from `can_target_player` →
   `stensia_bloodhall_cannot_target_a_player_with_hexproof` FAILED.
3. `colors_of` rewritten to union the colours in the object's activated-ability costs (exactly the error the
   ruling warns about) → `stensia_bloodhall_is_a_colorless_source` FAILED.

### Tricky interactions checked
- Damage to a planeswalker removes loyalty and marks no damage: **pass** (`damage_helper.rs:113`).
- Cannot target a creature: **pass** (new).
- Cannot target a hexproofed opponent; can target yourself: **pass** (new).
- Cannot fund its own tap ability: **pass** (`tap_cost_legality.rs`).
- Tapped → ability not offered: **pass** (`tap_cost_legality.rs`, added during the Gavony Township audit).
- Colourless source: **pass** (new), with the caveat above.

### Test coverage
- 2 damage to a player: `cards_activated_abilities.rs:341`
- 2 damage to a planeswalker, loyalty removed: `damage_helper.rs:113`
- cannot target a creature: `cards_activated_abilities.rs:376` (new)
- hexproof player cannot be targeted: `cards_activated_abilities.rs:398` (new)
- colourless source (the ruling): `cards_activated_abilities.rs:427` (new)
- mana ability offered alongside the utility ability: `mana_ability_offers.rs:27`
- cannot fund its own tap ability: `tap_cost_legality.rs:214`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1410 passing.

