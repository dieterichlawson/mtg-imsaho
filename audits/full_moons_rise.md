## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/180/full-moons-rise?utm_source=api
**Type line**: `Enchantment` — {1}{G}
**Oracle text**:
```
Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Werewolf** creatures you control get +1/+0 and have trample" — a static
  ability, so it covers Werewolves that arrive later, and it follows a
  transformed Werewolf on both faces: PASS
- "**Sacrifice this enchantment**: Regenerate all Werewolf creatures you
  control" — the sacrifice is a cost, so the Rise is gone while the ability is
  on the stack, and the shields still land: PASS
- The shields are given on resolution, so the set of Werewolves is read then
  (CR 611.2c): PASS
- The static +1/+0 ends when the Rise is sacrificed; the shields do not: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The static buff and the regeneration: `activated_no_stack.rs:full_moons_rise_shields_on_resolution`, `werewolf_cards.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/180/full-moons-rise?utm_source=api
**Type line**: `Enchantment` — {1}{G}
**Oracle text**:
```
Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
```

**Rulings fetched**:
- [2011-09-22] In order to regenerate Werewolves involved in combat, you must sacrifice Full Moon’s Rise before combat damage is assigned. This means they will lose the +1/+0 and trample bonuses before combat damage assignment.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/180/full-moons-rise
**Oracle text**:
```
Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
```
**Type line**: Enchantment
**Mana cost**: {1}{G}
**Rulings** (1, 2011-09-22): "In order to regenerate Werewolves involved in combat, you must sacrifice Full Moon's Rise before combat damage is assigned. This means they will lose the +1/+0 and trample bonuses before combat damage assignment."

**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{1}{G}`, `card_types: [Enchantment]`, oracle text
verbatim in the current "Sacrifice this enchantment" errata wording, no
subtypes, no P/T. Two continuous effects — `ModifyPT { power: 1, toughness: 0 }`
and `GrantKeyword { Trample }` — both scoped
`Global(And([ControlledByYou, HasSubtype("Werewolf")]))`. `Global` rather than
`GlobalOther` is right: the enchantment is not a creature, so there is no self
to exclude. The ability is `ManaCost::free()` plus
`SacrificeCost::SacrificeThis`, which is the whole printed cost.

### Code issues

No issue in `full_moons_rise.rs`. Two elsewhere, one of them substantive.

1. **`activate_via_hooks` was not paying `SacrificeThis`**
   (`tests/common/mod.rs:185`, fixed).
   - The helper's own doc comment says it pays "the costs the ability declares,
     paid the way `submit_action` pays them", and records that it used to pay
     only `pay_activation_cost` "so an ability whose cost is declared rather
     than hand-written (a tap, `counter_cost`) went on the stack for free and
     every test through this path measured the effect without the cost".
     `sacrifice_cost` was not included in that fix.
   - Found by writing the test for this card's ruling. After
     `activate_via_hooks` + `resolve_top_of_stack`, the enchantment was still on
     the battlefield and the Werewolf was still a 3/2 with trample:
     ```
     before:         rise zone Battlefield  power Some(3)
     after activate: rise zone Battlefield  power Some(3)
     after resolve:  rise zone Battlefield  power Some(3)  shields 1
     ```
   - The engine itself is correct — `engine/actions/abilities.rs:150` matches
     on `ab.sacrifice_cost` and calls `destruction::sacrifice` for
     `SacrificeThis`. This was the test path only, which is why nothing was
     failing.
   - Fixed for `SacrificeThis`; the `SacrificeCreature` /
     `SacrificeAnotherCreature` variants are a choice `legal_actions`
     enumerates and this helper cannot make, so they now panic pointing at
     `activate` rather than skipping the cost silently. Nine cards in the set
     declare a sacrifice cost; the whole suite stayed green.

2. **The card's static half had no test at all**
   (`cards_lands_and_mana_sources.rs:572`, filled in).
   - Oracle text says: `Werewolf creatures you control get +1/+0 and have trample.`
   - That file lists Full Moon's Rise in its module-doc index and then carried
     `// ══ Full Moon's Rise ══` with **nothing under it** — the next test in
     the file is Stony Silence's. The card's only coverage anywhere was
     `activated_no_stack.rs:159`, about when the shields go up.
   - Four mutations each produced zero failures across the whole workspace:
     `power: 0` instead of `1`; `Keyword::Vigilance` instead of `Trample`;
     scoping the buff to `ControlledByYou` alone (every creature you control);
     and dropping `has_subtype("Werewolf")` from the regeneration sweep.

Three tests added: the buff reaches your Werewolf and neither your Zombie nor
the opponent's Werewolf; the sacrifice trades the buff for the shields — which
is exactly what the ruling describes — and the shields land on the same set the
buff does.

### Tricky interactions checked
- **The ruling** (sacrificing loses the buff): PASS —
  `sacrificing_full_moons_rise_trades_the_buff_for_the_shields`. This is what
  exposed finding 1: the buff was *not* being lost, because the enchantment was
  never leaving.
- "+1/+0", not "+1/+1": PASS — toughness asserted unchanged.
- "you control": PASS — the opponent's Werewolf gets neither the buff nor a
  shield.
- "Werewolf creatures": PASS — a Zombie you control gets neither.
- A transformed Werewolf: every ISD werewolf back face keeps the Werewolf
  subtype, and both the filter (`CreatureFilter::HasSubtype` →
  `state.has_subtype`, `state.rs:1139`) and the card's sweep read the active
  face. Correct by the accessor; not separately tested.
- The ability still resolves after the enchantment is gone (CR 113.7a — the
  cost is paid on activation, the effect resolves later): PASS — implicit in
  every one of these tests now that the cost is actually paid, and explicit in
  `activated_no_stack.rs:159`, which checks the shields do not go up until
  resolution (CR 602.2a).
- The card reads `helpers::ability_controller` rather than `o.controller`, so
  "you control" is measured from the activator (CR 602.2a) even though the
  source is already in the graveyard by then.
- Regeneration itself (the shield replacing destruction) is `sba.rs`'s and
  `destruction.rs`'s, covered by the regeneration tests.

### UI presentation
Ability description: "Sacrifice: Regenerate all Werewolf creatures you
control". Log line: "Full Moon's Rise: all Werewolf creatures gain
regeneration". Both name the source.

### Test coverage
- +1/+0 and trample, to your Werewolves only: `cards_lands_and_mana_sources.rs`
  (`full_moons_rise_buffs_only_werewolves_you_control`) — **added this audit**.
- The ruling — sacrificing trades the buff for the shields:
  (`sacrificing_full_moons_rise_trades_the_buff_for_the_shields`) —
  **added this audit**.
- Shields to your Werewolves only:
  (`full_moons_rise_shields_only_werewolves_you_control`) — **added this audit**.
- Shields go up on resolution, not on activation: `activated_no_stack.rs:159`.

### Mutations run
| mutation | result |
| --- | --- |
| `power: 0` instead of `+1/+0` | fails two new tests (before: **nothing at all**) |
| `Keyword::Vigilance` instead of `Trample` | fails two new tests (before: **nothing at all**) |
| buff scoped to every creature you control | fails the buff test (before: **nothing at all**) |
| regenerate every creature you control | fails the shields test (before: **nothing at all**) |
| `activate_via_hooks` stops paying `SacrificeThis` (the bug as found) | fails the ruling test |

Suite after: 1454 passing, exit 0, zero warnings.

