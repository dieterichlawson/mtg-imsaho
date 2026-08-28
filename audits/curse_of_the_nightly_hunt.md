## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/137/curse-of-the-nightly-hunt?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{R}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls attack each combat if able.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The enchanted player still chooses which player or planeswalker each
  creature they control attacks" — `ForceAttack` is an attack *requirement*, not
  a choice of defender: PASS
- Ruling: "If, during the enchanted player's declare attackers step, a creature
  they control is tapped, is affected by a spell or ability that says it can't
  attack, or hasn't been under that player's control continuously since the turn
  began (and doesn't have haste), then it doesn't attack." A requirement cannot
  force an illegal attack (CR 508.1d): PASS
- A static ability, so it covers creatures that arrive after it resolved: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The attack requirement and its exceptions: `combat_requirements.rs`, `curse_and_equip_scope.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/137/curse-of-the-nightly-hunt?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{R}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls attack each combat if able.
```

**Rulings fetched**:
- [2011-09-22] The enchanted player still chooses which player or planeswalker each creature they control attacks.
- [2011-09-22] If, during the enchanted player’s declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can’t attack, or hasn’t been under that player’s control continuously since the turn began (and doesn’t have haste), then it doesn’t attack. If there’s a cost associated with having the creature attack, the player isn’t forced to pay that cost, so it doesn’t have to attack in that case either.

**Status**: ISSUE (fixed)

### Code issues

**One, in the engine: the prompt and the handler disagreed about "if able".**

- Oracle text says: `Creatures enchanted player controls attack each combat if able.`
- Ruling (2011-09-22): `If, during the enchanted player's declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can't attack, or hasn't been under that player's control continuously since the turn began (and doesn't have haste), then it doesn't attack.`

"and doesn't have haste" is the parenthesis that matters. `legal_actions`
builds the prompt's `must_attack` list by filtering
`combat::eligible_attackers`, which asks

```rust
&& (!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))
```

while `engine/actions/combat.rs::declare_attackers` rolled its own eligibility
check for the forced-attacker pass:

```rust
if creature.zone != Zone::Battlefield || creature.controller != active
    || !state.is_creature(creature.id, registry) || creature.tapped || creature.summoning_sick {
    continue;
}
```

— stopping at `summoning_sick`. The copy had drifted from the original, and
the two halves of the same rule gave different answers: the prompt told the
enchanted player their hasty creature had to attack, and the engine then let
it stay home. Manor Skeleton, Night Revelers, Falkenrath Marauders and
Traitorous Blood all bring haste to this set.

Fixed by deleting the copy. The forced pass now filters the same `eligible`
list the declaration itself was validated against, so there is one definition
of "able to attack" (CR 508.1a) and the requirement (CR 508.1d) is applied on
top of it. The hand-rolled version's other four conditions — zone, controller,
creature-ness, tapped, Defender, `can_attack` — were all already in
`eligible_attackers`, so nothing else changes.

### Card data

`{2}{R}`, `Enchantment — Aura Curse` with both subtypes,
`ContinuousEffect::ForceAttack` with
`EffectScope::Global(CreatureFilter::ControlledByAttachedPlayer)` for
"creatures enchanted player controls", `TargetRequirement::PlayerOnly` for
"Enchant player", `resolve_curse` for the attachment. No triggered or
activated abilities. Cost and type line pinned pool-wide by
`card_data_invariants.rs`; the `Enchant` keyword Scryfall lists is one the
keyword invariant deliberately does not model.

### Tricky interactions checked

- Haste: **was broken, fixed**.
- Tapped: pass — `combat_rules.rs::a_tapped_creature_is_not_forced_to_attack`.
- "Can't attack" effects (Pacifism, Bonds of Faith): pass —
  `a_creature_under_pacifism_is_not_forced_to_attack`,
  `bug_bp_forced_attack_respects_cant_attack`.
- Defender: pass, through `eligible_attackers`.
- The Curse on the player who controls it — "enchanted player", not "your
  opponents": pass, and now pinned from both sides.
- Forced attackers tap unless they have vigilance (CR 508.1f): pass, in the
  same handler.
- A creature dragged in by the requirement has attacked for the turn
  (CR 508.1): pass, `attacked_on_turn` is stamped.

### Recorded, not fixed

**Ruling 1 — "The enchanted player still chooses which player or planeswalker
each creature they control attacks" — is only half available.** Attacking a
planeswalker is not modelled anywhere in this engine:
`CombatState::attackers` is a `HashMap<ObjectId, PlayerId>`, so the data model
cannot express it, and `declare_attackers` validates every declaration against
`*def == state.opponent(active)`. That is an engine-wide scope limit, not
something this card introduces or could fix; with two players and no
planeswalker attacks there is exactly one legal defender, so the choice the
ruling protects has one option and the forced pass picking it takes nothing
away. Worth revisiting whenever planeswalker combat is implemented — the
forced pass would then need to ask rather than assume, and it is the same line
either way.

**"If there's a cost associated with having the creature attack, the player
isn't forced to pay that cost"** — no card in this set puts a cost on
attacking, so there is nothing to implement and nothing to test.

### Test coverage

- the enchanted player's creatures are forced, the controller's are not:
  `cards_upkeep_triggers_and_curses.rs::curse_of_nightly_hunt_forces_attack`
- the Curse on its own controller — where "enchanted player" and "anyone who
  isn't me" disagree:
  `…::curse_of_nightly_hunt_forces_its_own_controller_when_it_enchants_them`
  (new), which also submits an empty declaration and checks the creature is
  dragged in anyway
- haste: `combat_rules.rs::a_hasty_creature_is_forced_to_attack_the_turn_it_arrives` (new)
- tapped: `combat_rules.rs::a_tapped_creature_is_not_forced_to_attack`
- can't attack: `combat_rules.rs::a_creature_under_pacifism_is_not_forced_to_attack`

### Mutations run

- Add `!summoning_sick` back to the forced filter: **fails** the haste test,
  and nothing else — which is exactly the drift that existed.
- Restore the `ControlledByAttachedPlayer` "opponents" guess in
  `matches_filter`: **fails** the self-curse test, passes the opposing-player
  one.
- Change the card's filter to `ControlledByYou`: **fails** the
  opposing-player test, passes the self-curse one. With the mutation above,
  the two tests pin the filter from both sides.

Suite: 1515 passing, exit 0, `cargo check --workspace --all-targets` clean.
