## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/221/demonmail-hauberk?utm_source=api
**Type line**: `Artifact — Equipment` — {4}
**Oracle text**:
```
Equipped creature gets +4/+2.
Equip—Sacrifice a creature.
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

### Deviation recorded, not changed
`legal_actions` filters out every (target, sacrifice) combo where the two are
the same object, and `activated_abilities` used to return nothing below two
creatures. Both are deliberate — `sacrifice_choice.rs`'s module doc explains
that the engine once auto-picked the sacrifice and fizzled the equip, and the
filter is there so a player cannot pick a fizzling combo by accident.

It is still a legal play the engine will not offer. Targets are chosen first
(CR 601.2c) and costs paid after (CR 601.2h); nothing stops you sacrificing the
creature you targeted, and the equip then fizzles (CR 608.2b). With Falkenrath
Noble out — "whenever this creature or another creature dies, target player
loses 1 life and you gain 1 life" — the fizzle is what you were buying.

Flagged rather than reversed: it spans three cards and several tests that
pin it on purpose, so it is the project's call, not the audit's.

- Ruling: "You can sacrifice the creature Demonmail Hauberk is equipping in
  order to equip it to another creature" — supported, as long as a second
  creature exists to be the new target: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The (target, sacrifice) enumeration: `sacrifice_choice.rs:hauberk_legal_actions_enumerate_target_sacrifice_combos`
- Explicit sacrifice attaches correctly: `sacrifice_choice.rs:hauberk_explicit_sacrifice_attaches_correctly`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/221/demonmail-hauberk?utm_source=api
**Type line**: `Artifact — Equipment` — {4}
**Oracle text**:
```
Equipped creature gets +4/+2.
Equip—Sacrifice a creature.
```

**Rulings fetched**:
- [2011-09-22] You can sacrifice the creature Demonmail Hauberk is equipping in order to equip it to another creature.

**Status**: ISSUE


One ruling: "You can sacrifice the creature Demonmail Hauberk is equipping in
order to equip it to another creature."

### Code issues

**1. The card re-implemented an engine rule, and got it wrong.**

`demonmail_hauberk.rs` counted creatures and refused to offer equip at all
below two:

```rust
// The equip cost is "Sacrifice a creature." After paying this cost, there
// must still be a creature to equip to. Require at least 2 creatures: one
// to sacrifice as the cost, and one remaining to be the equip target.
if creature_count < 2 { return vec![]; }
```

Nothing in the oracle text says that. "Equip—Sacrifice a creature" is a cost,
and CR 601.2b chooses targets *before* CR 601.2h pays costs, so with a single
creature you may target it, sacrifice it to pay, and have the equip countered
on resolution for want of a legal target (CR 608.2b). The creature still dies —
which is frequently the reason to do it. Demonmail Hauberk is a free
sorcery-speed sacrifice outlet, and next to a Doomed Traveler that is most of
what it is for.

Whether a cost can be paid is the engine's question, not a card's. Removed.

**2. The engine hid the legal play too.**

`legal/abilities.rs` filtered out every (target, sacrifice) pair where the
sacrifice was the target:

```rust
// We exclude pairs where the sacrifice IS the target — sacrificing the
// target makes the ability fizzle, no rational player picks that.
```

Both premises are true and the conclusion does not follow. The activation is
legal, the sacrifice happens, and only the ability is countered. Hiding it is a
player-protection heuristic sitting in the rules layer, and for this card it
removed the only way to use the outlet at all. The filter is gone; every legal
pair is now offered, and the engine's CR 608.2b re-check already handles the
fizzle correctly (`helpers::resolve_equip` refuses a target that has left the
battlefield).

This does not undo the original fix that filter was bolted onto. The real bug
was the engine *auto-picking* the sacrifice and choosing the target by
accident; the fix — enumerate one action per (target, sacrifice) pair so the
player chooses explicitly — is untouched.

### Tests that enshrined the wrong behaviour
Six, all in `sacrifice_choice.rs`, and one of them argued itself there from
this card's ruling:

> "(Per the ruling, you CAN sacrifice the equipped creature to equip another,
> but with only 1 creature there's no valid target to equip TO.)" …
> "Actually, per the ruling: … the equip ability should NOT be available"

The ruling grants a permission. It says you *may* sacrifice the equipped
creature; it says nothing about a minimum board. Reading a restriction out of a
permission is how the card lost its outlet. That test is now
`hauberk_can_sacrifice_the_creature_it_is_equipping_to_move_itself`, which
tests what the ruling actually says: Hauberk on A, equip B, pay by sacrificing
A, Hauberk ends on B with +4/+2 applied.

The other five: the two combo-count tests updated (3×3 = 9 for the Hauberk,
5×2 = 10 for Skirsdag Cultist), and the three "must not offer the fizzling
pair" tests inverted into "offers it, and paying the cost kills the creature".

### Tricky interactions checked
- The ruling — sacrifice the equipped creature to move the Hauberk: pass
- One creature: equip is a sacrifice outlet, the equip then fizzles: pass
- Sacrificing the target is offered but the equipment does not attach to a
  creature in the graveyard: pass
- The player picks the sacrifice; the engine never auto-picks: pass
- Equip is sorcery-speed only (CR 702.6b): pass, `sorcery_speed_only: true`
- Equip targets a creature *you control*: pass, via
  `helpers::equip_target_is_legal`
- The Equipment cannot equip while it is itself a creature (CR 301.5c): pass,
  `!state.is_creature` in `activated_abilities` and again in `resolve_equip`
- No autotap for sacrifice-cost abilities (a creature cannot be both a mana
  source and the sacrifice): unchanged, still enforced

### Test coverage
- Every (target, sacrifice) pair enumerated: `sacrifice_choice.rs:38`
- An explicit sacrifice attaches and applies +4/+2: `sacrifice_choice.rs:75`
- **REWRITTEN** one creature is a sacrifice outlet: `sacrifice_choice.rs:106`
- **REWRITTEN** the self-sacrificing pair is offered: `sacrifice_choice.rs:137`
- **REWRITTEN** the ruling's own case: `sacrifice_choice.rs:376`
- **REWRITTEN** Skirsdag Cultist pair count and self-sacrifice:
  `sacrifice_choice.rs:250`, `sacrifice_choice.rs:277`
- Equip needs sorcery speed / target legality: covered by the shared equipment
  tests via `helpers::resolve_equip`

### Note on card data
Scryfall lists `Keywords: Equip` and `card_data.keywords` is empty. That is the
codebase's convention for equip across every Equipment in the set — equip is
modelled as an entry in `activated_abilities`, not as a `Keyword`, and nothing
queries `Keyword::Equip`. Consistent, so not flagged.

