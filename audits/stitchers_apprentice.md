## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/81/stitchers-apprentice?utm_source=api
**Type line**: `Creature — Homunculus` — {1}{U}, 1/2
**Oracle text**:
```
{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The creature you sacrifice ... could be the Homunculus you've just
  created. It could also be Stitcher's Apprentice itself" — the sacrifice is
  part of the *effect* (after the colon), so the token is on the battlefield and
  eligible: PASS
- Ruling: "You create a token and sacrifice a creature all while the activated
  ability is resolving. Nothing can happen between the two" — both happen inside
  one `resolve_activated_ability`: PASS
- Ruling: "Any abilities that trigger on the Homunculus token entering the
  battlefield will resolve after you've sacrificed a creature" — triggers are
  collected and resolved after the ability finishes: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Token then sacrifice, in that order: `cards_complex_creatures.rs:stitchers_apprentice_creates_token_then_sacrifices`
- The token is a 2/2 Homunculus: `cards_complex_creatures.rs:stitchers_apprentice_token_is_2_2_homunculus`
- ETB triggers fire after the sacrifice: `phantom_triggers.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/81/stitchers-apprentice?utm_source=api
**Type line**: `Creature — Homunculus` — {1}{U}, 1/2
**Oracle text**:
```
{1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
```

**Rulings fetched**:
- [2018-12-07] The creature you sacrifice for the ability of Stitcher’s Apprentice could be the Homunculus you’ve just created. It could also be Stitcher’s Apprentice itself.
- [2018-12-07] You create a token and sacrifice a creature all while the activated ability is resolving. Nothing can happen between the two, and no player may choose to take actions.
- [2018-12-07] Any abilities that trigger on the Homunculus token entering the battlefield will resolve after you’ve sacrificed a creature.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/81/stitchers-apprentice
**Oracle text**: {1}{U}, {T}: Create a 2/2 blue Homunculus creature token, then sacrifice a creature.
**Type line**: Creature — Homunculus
**Mana cost**: {1}{U} — **P/T**: 1/2
**Rulings** (3, all 2018-12-07):
1. "The creature you sacrifice for the ability of Stitcher's Apprentice could be the Homunculus you've just created. It could also be Stitcher's Apprentice itself."
2. "You create a token and sacrifice a creature all while the activated ability is resolving. Nothing can happen between the two, and no player may choose to take actions."
3. "Any abilities that trigger on the Homunculus token entering the battlefield will resolve after you've sacrificed a creature."

**Status**: ISSUE (fixed) — the card code is correct; three of its claims had no test.

### Card data
Matches the fetched text: `{1}{U}`, `card_types: [Creature]`,
`subtypes: ["Homunculus"]`, 1/2, oracle text verbatim, no keywords. The ability
is `{1}{U}` plus `requires_tap: true`, `target_requirement: None` (it targets
nothing — "sacrifice a creature" is a choice, not a target), `once_per_turn:
false`. The `zone == Battlefield` gate in `activated_abilities` is the
redundant-but-kept kind recorded in the Mirror-Mad Phantasm entry.

### How it meets its rulings
- **Ruling 2** (both halves in one resolution, no priority between): the token
  is created and then the sacrifice is raised as an `AwaitingAction::
  ResolutionChoice`, which suspends the resolution rather than passing priority.
  Correct by construction.
- **Ruling 3** (the token's own ETB triggers resolve after the sacrifice):
  triggers collected during a resolution go on the stack and resolve after it
  finishes — the engine's, not this card's.
- **Ruling 1** (the token and the Apprentice are both eligible): the token is
  created *before* `creatures_controlled_by` is called, so it is in the list,
  and the Apprentice is on the battlefield (tapped, but still a creature) so it
  is too. Correct — and it had no assertion, which is finding 1 below.

### Code issues

No issue in `stitchers_apprentice.rs`. Three of the card's claims had nothing
holding them down; each mutation passed the entire workspace.

1. **The offered list — where ruling 1 lives — was never read**
   (`cards_sacrifice_and_additional_costs.rs:555`, rebuilt).
   - Ruling says: `could be the Homunculus you've just created. It could also be Stitcher's Apprentice itself`
   - The test asserted `state.awaiting_action.is_some()` and then found the
     token by scanning `state.objects` — never looking at what the prompt
     offered.
   - Verified: swapping `helpers::creatures_controlled_by(state, controller, registry)`
     for `helpers::creature_targets(..)` — every creature on the battlefield,
     including the opponent's, against CR 701.16b — produced zero failures.
   - Now the options are read and checked to contain the token **and** the
     Apprentice, and not the opponent's creature.

2. **"then sacrifice a creature" is not "you may"**
   (same test).
   - Oracle text says: `then sacrifice a creature`
   - Code says: `present_target_choice(.., false, // mandatory .., registry)`
   - Verified: flipping that to `true` produced zero failures.
   - Now checked through `legal_actions`: no `ChosenTarget(None)` is offered,
     so there is nothing to decline.

3. **The token's colour had no assertion**
   (`stitchers_apprentice_token_is_2_2_homunculus`, rebuilt).
   - Oracle text says: `a 2/2 blue Homunculus creature token`
   - The test read `token.power`, `token.toughness` and `token.name` — the raw
     fields it was built with — and nothing about colour.
   - Verified: creating it `Color::Black` produced zero failures.
   - Rebuilt to read `effective_power`, `effective_toughness`, `colors_of`,
     `has_subtype` and `is_creature` through the accessors, plus the CR 111.4
     name.

### Tricky interactions checked
- Sacrificing the token you just made (ruling 1): PASS — asserted in the
  options and carried out in `stitchers_apprentice_creates_token_then_sacrifices`.
- Sacrificing the Apprentice itself (ruling 1): PASS — asserted in the options.
  Not carried out, because the sacrifice-the-token row already proves the choice
  is acted on and the Apprentice is tapped either way.
- Only your own creatures are eligible (CR 701.16b): PASS — new assertion.
- The sacrifice is mandatory: PASS — new assertion.
- No priority between the two halves (ruling 2): correct by construction — a
  `ResolutionChoice` suspends the resolution rather than passing priority.
  Structural, not separately tested.
- The sacrifice is a death that watchers see: PASS —
  `phantom_triggers.rs:174` (`a_sacrifice_made_during_an_activation_is_seen_by_death_watchers`),
  where Falkenrath Noble drains for it.
- Sacrifice bypasses indestructible (CR 701.16a): the effect is
  `PendingEffect::SacrificeCreature`, the sacrifice pipeline, not `try_destroy`.
  Correct by construction; the generic case is covered in the sacrifice tests
  around it.
- The Apprentice leaves the battlefield in response to its own ability: the
  card reads `helpers::ability_controller` rather than `o.controller`, so the
  activator still gets the token (CR 602.2a / 608.2g).
- No creatures at all to sacrifice: unreachable — the token has just been
  created and is one. The `creatures.is_empty()` early return is defensive.
- `{T}` cost legality and summoning sickness: the engine's; the card does not
  re-decide them.

### UI presentation
Ability description:
`"{1}{U}, {T}: Create a 2/2 blue Homunculus token, then sacrifice a creature"`,
and the prompt reads "Stitcher's Apprentice: choose a creature to sacrifice".
Both name the source, and the token is logged as it is created.

### Test coverage
- Ruling 1, both eligible creatures: `cards_sacrifice_and_additional_costs.rs`
  (`stitchers_apprentice_offers_every_creature_you_control_and_only_those`) —
  **added this audit**.
- Only creatures you control: same test — **added this audit**.
- Mandatory, nothing to decline: same test — **added this audit**.
- The chosen creature is the one that goes:
  (`stitchers_apprentice_creates_token_then_sacrifices`) — **tightened this
  audit** to assert the token's zone rather than only a head count.
- Token is a 2/2 blue Homunculus creature:
  (`stitchers_apprentice_token_is_a_two_two_blue_homunculus`) — **rebuilt this
  audit**; the colour is new.
- Ruling 2 (one resolution): structural, not separately tested.
- Ruling 3 (token ETB triggers resolve after): the engine's trigger ordering,
  not separately tested for this card.
- The sacrifice is a visible death: `phantom_triggers.rs:174`.

### Mutations run
| mutation | result |
| --- | --- |
| token created `Color::Black` instead of blue | fails the rebuilt token test (before: **nothing at all**) |
| sacrifice made optional | fails the new options test (before: **nothing at all**) |
| offer every creature on the battlefield, not only yours | fails the new options test (before: **nothing at all**) |

Suite after: 1451 passing, exit 0, zero warnings.

