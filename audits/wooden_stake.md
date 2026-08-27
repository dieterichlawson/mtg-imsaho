## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/237/wooden-stake?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature gets +1/+0.
Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
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
- "destroy that creature. **It can't be regenerated**" — the no-regenerate
  destruction path, not plain `try_destroy`: PASS
- "blocks **or becomes blocked by** a Vampire" — both directions: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Destroying a Vampire on block, and leaving a non-Vampire alone: `cards_equipment_costs.rs:wooden_stake_destroys_vampire_on_block`, `:wooden_stake_does_not_destroy_non_vampire`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/237/wooden-stake?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature gets +1/+0.
Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```

**Rulings fetched**:
- [2011-09-22] The Vampire is destroyed before any combat damage is dealt.

**Status**: ISSUE (fixed)

### Code issues

**The trigger condition was re-tested on resolution.**

- Oracle text says: `Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.`
- Code did, in both `on_blocks` and `on_becomes_blocked`:
  ```rust
  if state.has_subtype(other_creature, "Vampire", registry) {
      ... try_destroy_no_regen ...
  }
  ```
  under a comment saying "the resolution handlers re-check for defense in depth".

"Whenever ... becomes blocked by a Vampire" is a trigger *event* condition (CR
603.2), asked once, when the ability would trigger — which the card already does
correctly in `should_trigger_on_blocks` / `should_trigger_on_becomes_blocked`.
CR 603.4 re-checks only an intervening-if clause ("..., **if** ..."), and this
ability has none. Once it has triggered, "destroy that creature" is
unconditional.

So a creature that stopped being a Vampire between the block and the trigger's
resolution would have been spared. Nothing in this pool removes a creature type
— Olivia Voldaren only adds one — so it is not reachable here, but it was a
rules error wearing a reassuring comment, and it duplicated a condition that
already had one correct home. Both handlers now go straight to the destruction
through a shared `stake()`.

Also replaced a comment on the equip gate that pointed at Cobbled Wings for an
explanation that no longer exists there. The gate itself is right and is worth
naming: CR 301.5c, an Equipment that is also a creature can't equip.

### Rulings checked

- **"The Vampire is destroyed before any combat damage is dealt."** The trigger
  fires when blockers are declared, so it resolves in the declare blockers step,
  a whole step before combat damage. Verified end to end by advancing into the
  damage step and checking the equipped creature took nothing — Markov Patrician
  is a 3/1 and would otherwise have killed it. PASS.

### Tricky interactions checked

- **"destroy that creature" is the Vampire, not the equipped creature.** "A
  Vampire" is the nearest noun, and the ruling settles it by naming the Vampire
  as the one destroyed. The code destroys the Vampire. PASS.
- **"It can't be regenerated"** goes through `try_destroy_no_regen`, not
  `try_destroy`. Tested against a live shield — the test first proves the shield
  saves the Vampire from an ordinary `try_destroy`, so that surviving the Stake
  would be a real difference. PASS.
- **Both directions of "blocks or becomes blocked by".** Two separate trigger
  kinds, and the collector routes both to attached Equipment as well as to the
  creature itself (`triggers/collect/combat.rs:97` and `:132`). The
  becomes-blocked direction had no test; it has one now. PASS.
- **The Stake being removed in response** does not stop the destruction — the
  handlers never look at the Equipment, only at the Vampire (CR 113.7a). PASS.
- **Equipment detaching when the equipped creature dies** is the engine's, at
  `sba.rs:161` — Equipment detaches rather than being destroyed (CR 704.5n),
  which is the branch that separates it from an Aura. PASS.
- **Two Vampires blocking** the equipped creature: the collector loops per
  (attacker, blocker) pair, so the ability triggers once per Vampire and both
  die. PASS.
- **`ModifyPT { scope: Attached }`** for "+1/+0", so the bonus follows the
  Equipment rather than being written onto the creature. PASS.
- **Equip is sorcery-speed and targets a creature you control** —
  `sorcery_speed_only: true`, `CreatureWithFilter(YouControl)`, and
  `is_valid_target` re-checks the controller (CR 608.2b). PASS.

### A correction to my own work

I first asserted that the regeneration shield should survive being destroyed
through `try_destroy_no_regen` — that the shield "was not even spent". That was
wrong: the shield is zeroed as ordinary leave-the-battlefield cleanup (CR
400.7), and a destroyed creature's shield count is not observable. The code was
right and my assertion was not. Replaced with the contrast against `try_destroy`
described above, which tests the thing that actually matters.

### Test coverage

- Vampire destroyed on block: `cards_equipment_costs.rs::wooden_stake_destroys_vampire_on_block`.
- non-Vampire untouched: `::wooden_stake_does_not_destroy_non_vampire`.
- the ruling, destroyed before combat damage: `::wooden_stakes_vampire_dies_before_it_can_deal_combat_damage` (new).
- can't be regenerated, against a shield proven live: `::wooden_stakes_vampire_cannot_regenerate` (new, mutation-checked against `try_destroy`).
- the becomes-blocked direction: `::wooden_stake_destroys_a_vampire_that_blocks_the_equipped_creature` (new).
- equip cost and +1/+0: `::equipping_grants_the_printed_bonus`, `cards_equipment_costs.rs:85`.
- detaches when the creature dies: `::equipment_detaches_when_creature_dies`.

