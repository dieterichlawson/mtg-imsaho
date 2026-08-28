## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/143/furor-of-the-bitten?utm_source=api
**Type line**: `Enchantment — Aura` — {R}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If the enchanted creature can't attack for any reason (such as being
  tapped or having come under that player's control that turn), then it doesn't
  attack." An attack requirement cannot force an illegal attack (CR 508.1d):
  PASS
- "attacks each combat if able" is a requirement on the creature, so it follows
  the Aura rather than the controller: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the attack requirement: `enchantments.rs`, `combat_requirements.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/143/furor-of-the-bitten?utm_source=api
**Type line**: `Enchantment — Aura` — {R}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 and attacks each combat if able.
```

**Rulings fetched**:
- [2020-06-23] If the enchanted creature can't attack for any reason (such as being tapped or having come under that player's control that turn), then it doesn't attack. If there's a cost associated with having it attack, the player isn't forced to pay that cost, so it doesn't have to attack in that case either.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{R}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", and the one sentence as two continuous effects:

- `ModifyPT { power: 2, toughness: 2, scope: Attached }`
- `ForceAttack { scope: Attached }`

Cost, type line and subtypes pinned pool-wide by `card_data_invariants.rs`;
`Enchant` is one of the keywords the keyword invariant deliberately does not
model. `resolve_aura` for attachment, no `is_valid_target` override, no
card-side cleanup. Both effects `Attached`, which is what "enchanted creature"
means.

Structurally the same card as Spectral Flight with `ForceAttack` where that
one has `GrantKeyword`, and the compulsion is the same engine machinery the
Curses reach through a global scope — `must_attack`, applied on top of
`combat::eligible_attackers`.

### Tricky interactions checked

- The ruling, clause by clause. "If the enchanted creature can't attack for
  any reason (such as being tapped or having come under that player's control
  that turn), then it doesn't attack":
  - tapped: pass, `combat_rules.rs::a_tapped_creature_is_not_forced_to_attack`
    reaches it through Galvanic Juggernaut.
  - a "can't attack" effect: pass, `combat_rules.rs::bug_bp_forced_attack_respects_cant_attack`
    puts Bonds of Faith and this Aura on the same non-Human.
  - **came under that player's control this turn**: pass, and untested until
    now. `GameState::change_control` sets `summoning_sick` for the new
    controller, which is exactly that clause; a stolen creature with no haste
    stays home. (Traitorous Blood, the set's one steal effect, grants haste
    alongside — so the clause was reachable only by driving `change_control`
    directly, which this test does.)
  - "If there's a cost associated with having it attack": no card in this set
    puts a cost on attacking, so there is nothing to implement or test.
- Haste — the other half of "if able" (CR 302.6): pass, and untested for this
  card. The Curse of the Nightly Hunt audit fixed the forced-attacker pass to
  ask `eligible_attackers` rather than its own drifted copy; this is the Aura
  route to the same rule, and the mutation below shows it is the same code.
- Both effects end when the Aura leaves: pass, untested until now.
- CR 704.5m and fizzle: engine-level, covered by `enchantments.rs` and
  `fizzle.rs` for the Aura shape.

### Test coverage

- forced to attack when the player declares none:
  `cards_morbid_and_ltb.rs::furor_forces_attack`
- a hasty creature is forced the turn it arrives:
  `cards_morbid_and_ltb.rs::furor_forces_a_hasty_creature_the_turn_it_arrives` (new)
- a creature that just changed hands is not:
  `cards_morbid_and_ltb.rs::furor_does_not_force_a_creature_that_just_changed_hands` (new)
- both effects end with the Aura:
  `cards_morbid_and_ltb.rs::furors_buff_and_compulsion_end_with_the_aura` (new)
- +2/+2 alongside the compulsion:
  `cards_vanilla_and_keywords.rs::furor_of_the_bitten_gives_plus_two_and_forces_attack`
- a "can't attack" effect beats the compulsion:
  `combat_rules.rs::bug_bp_forced_attack_respects_cant_attack`

### Mutations run

- The card's `ForceAttack` scope `Attached` → `OnSelf`: **fails** all four
  Furor tests.
- `change_control` stops setting `summoning_sick`: **fails** the
  just-changed-hands test, and only that one — so the ruling's clause is
  pinned to the mechanism that implements it rather than to a coincidence.
- The forced-attacker pass reintroduces its own `summoning_sick` check
  (the drift fixed during the Curse of the Nightly Hunt audit): **fails** the
  haste test, and only that one.

Suite: 1540 passing, exit 0, `cargo check --workspace --all-targets` clean.
