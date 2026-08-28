## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Type line**: `Artifact — Equipment` — {1}
**Oracle text**:
```
Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```

**Status**: ISSUE

### Code issues
See below.


### Tricky interactions checked
- Equipment enters unattached and stays on the battlefield when what it equipped
  leaves (CR 704.5n), rather than going to the graveyard as an unattached Aura
  would (CR 704.5m): PASS — and this is the one that was wrong. Being an
  Equipment was a per-object `is_equipment` bool that eleven cards set in an
  `on_resolve` override which otherwise only repeated the trait default's "move
  a permanent to the battlefield". An Equipment that reached the battlefield any
  other way left the flag false and was then read as an Aura. Now derived from
  the Equipment subtype (CR 301.5) through the characteristics layer, and the
  eleven dead overrides are gone.
- "Equip only as a sorcery" — `sorcery_speed_only: true`: PASS
- "Attach to target creature **you control**" — `TargetFilter::YouControl` and
  the card's own `is_valid_target`: PASS
- The equip ability is offered on the Equipment, not duplicated onto the
  creature it is attached to: PASS
- The attach happens on resolution, not on activation (CR 602.2a): PASS
- "{T}, Sacrifice Blazing Torch:" — the sacrifice is a cost, so it is paid on
  activation (CR 601.2h) and an opponent responding already sees the Torch in
  the graveyard. The Torch is not the object the ability is activated on, so the
  `ActivatedAbilityDef`'s `SacrificeCost` cannot express it; it is now the one
  card besides Moorland Haunt that uses `pay_activation_cost`.
- Ruling: "The source of the damage is Blazing Torch, not the equipped
  creature." The Torch is in the graveyard by resolution, so it is found through
  the `last_attached_to` the engine records on every zone change — last known
  information, CR 608.2g: PASS
- "Equipped creature can't be blocked by Vampires or Zombies" — a blocking
  restriction, not evasion, and not menace: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Damage to a creature and to a player, sourced from the Torch: `cards_equipment_and_artifacts.rs:blazing_torch_deals_damage_to_player`, `:blazing_torch_deals_its_damage_as_the_torch_not_the_creature`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Type line**: `Artifact — Equipment` — {1}
**Oracle text**:
```
Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```

**Rulings fetched**:
- [2009-10-01] If a Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can’t be activated by either player. Only the creature’s controller may activate the ability — but since that player can’t sacrifice Blazing Torch (a permanent they don’t control), the ability’s cost can’t be paid.
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature. However, the equipped creature’s ability is what targets the permanent or player. If Blazing Torch is equipped to a red creature, for example, the ability couldn’t target a creature with protection from red. It could target a creature with protection from artifacts, but all the damage would be prevented.

**Status**: PASS (card); engine ordering fixed

### Code issues

No issues in the card's own behaviour. Both rulings hold, and the two lines of oracle text are implemented as written.

**Found while verifying the rulings**, in the engine paths this card exercises: the last places the object map was walked in HashMap order were the ones the player sees most directly. Eighteen scans in action generation build the target lists for spells and abilities, plus the list of abilities that attached permanents grant (`engine/legal/abilities.rs:46`, the loop that implements this card's first ruling), and `legal_actions` drained its ability groups out of a `HashMap`. A player picks from those lists by position, so the same board offered the same choices under different indices on a replay of the same game. All eighteen now go through `objects_in_id_order`, the ability groups are sorted by (object, granting card, ability index), and the map guard covers `src/cards`, `src/triggers` and `src/engine` alike.

### Checked against each ruling

- `If a Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player. Only the creature's controller may activate the ability — but since that player can't sacrifice Blazing Torch (a permanent they don't control), the ability's cost can't be paid.` — PASS. `engine/legal/abilities.rs` walks the acting player's permanents and only collects granted abilities from attachments where `attached.controller == player`, so the ability appears for neither player when the two differ. The comment there cites this card by name.
- `The source of the damage is Blazing Torch, not the equipped creature. However, the equipped creature's ability is what targets the permanent or player. If Blazing Torch is equipped to a red creature, for example, the ability couldn't target a creature with protection from red. It could target a creature with protection from artifacts, but all the damage would be prevented.` — PASS, both halves, and they come from two different places, correctly:
  - Targeting: `generate_ability_targets` passes `Some(source_id)` where `source_id` is the **creature** the ability was activated on, so `can_be_targeted_by` reads the creature's characteristics.
  - Damage: `resolve_activated_ability` computes `let damage_source = sacrificed_torch(state, object_id, registry)`, finding the sacrificed Torch through `card_state["last_attached_to"]` (written by `move_object` on leaving the battlefield, CR 608.2g), and `has_protection_from` reads that object's characteristics — an artifact, even in the graveyard.

### Checked and correct

- Cost `{1}`, `Artifact — Equipment`, subtypes `["Equipment"]`.
- `Equipped creature can't be blocked by Vampires or Zombies` is `CanOnlyBeBlockedBy { allowed_blockers: Not(Or([Vampire, Zombie])), scope: Attached }` — a blocking restriction, not menace, and not a targeting restriction.
- The sacrifice is paid in `pay_activation_cost`, i.e. on activation (CR 601.2h via 602.2b), not on resolution — an opponent responding to the ability already sees the Torch in the graveyard.
- `SacrificeCost::None` on the granted ability with a comment explaining why: the Torch is not the object the ability is activated on, so the enum cannot express it.
- `is_valid_target` here accepts anything on the battlefield or any live player, which is not a widening: `generate_ability_targets` applies it **after** the `TargetRequirement`, so equip is still `CreatureWithFilter(YouControl)` and the damage ability is still `AnyTarget`.
- Damage goes through `damage::deal_damage` with `DamageKind::NonCombat`.
- `Target::Illegal` returns without dealing damage (CR 608.2b).

### Noted, not fixed

Two Blazing Torches equipped to the same creature grant two copies of the same ability, and `pay_activation_cost` receives only the creature and the ability index — not which attachment granted it — so `attached_torch` sacrifices the lower-id one either way. Making the player's choice of Torch meaningful means carrying the granting object through action generation, action application and every card's `pay_activation_cost` signature. The outcome is identical in every respect a player can observe here (2 damage, one Torch sacrificed); only the identity of two otherwise-identical objects differs. That is a plumbing change of a size that belongs on its own, not folded into this card's audit.

### Tricky interactions checked

- Torch and equipped creature controlled by different players: ability offered to neither. PASS.
- Damage source is the Torch, not the creature (for `damaged_by` watchers and protection): PASS.
- Targeting source is the creature, not the Torch: PASS.
- Protection from artifacts: targetable, damage fully prevented. PASS.
- Vampire and Zombie blockers stopped, others not: PASS.
- Equip restricted to creatures you control: PASS.
- Equip only at sorcery speed: `sorcery_speed_only: true`.

### Test coverage

- grants the damage ability to the equipped creature: `cards_equipment_and_artifacts.rs:226`
- deals 2 damage to a player and the Torch is sacrificed: `cards_equipment_and_artifacts.rs:244`
- damage source is the Torch, not the creature: `cards_equipment_and_artifacts.rs:272`
- equip only targets creatures you control: `cards_equipment_and_artifacts.rs:292`
- ability not offered when the Torch's controller differs from the creature's: `combat_rules.rs:509`
- equip autotap: `equipment_autotap.rs:161`
- can't be blocked by Vampires or Zombies: `cards_equipment_and_artifacts.rs` `blazing_torch_stops_vampires_and_zombies_from_blocking` (NEW, mutation-checked)
- targets as the creature, damages as the Torch (protection from artifacts): `cards_equipment_and_artifacts.rs` `blazing_torch_targets_as_the_creature_and_damages_as_the_torch` (NEW, mutation-checked)
- two Torches on one creature: NOT TESTED — see "Noted, not fixed".

