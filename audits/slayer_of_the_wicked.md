## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/32/slayer-of-the-wicked?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 3/2
**Oracle text**:
```
When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
```

**Status**: PASS

### Code issues
No issues found.

- "**you may** destroy **target** Vampire, Werewolf, or Zombie" — same locked-target
  plus optional-decision shape as Geistcatcher's Rig.
- Destroys through `PendingEffect::Destroy`, so indestructible and regeneration
  apply; the oracle says destroy, not exile or sacrifice.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/32/slayer-of-the-wicked?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 3/2
**Oracle text**:
```
When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, **destroy target Vampire, Werewolf, or Zombie**" —
  all three subtypes, and the trigger declares its target so hexproof is
  filtered by the engine: PASS
- `has_subtype` covers granted subtypes, so a creature Olivia Voldaren turned
  into a Vampire is a legal target: PASS
- `try_destroy`, so indestructible survives: PASS
- No legal target means the trigger is not put on the stack: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The subtype filter and hexproof: `hexproof_filter.rs:an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/32/slayer-of-the-wicked?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 3/2
**Oracle text**:
```
When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
```

**Rulings fetched**:
- [2011-09-22] If you control the only Vampire, Werewolf, or Zombie, you must target it with Slayer of the Wicked’s ability. You choose whether or not to destroy the target when the ability resolves.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/32/slayer-of-the-wicked
**Oracle text**: When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
**Type line**: Creature — Human Soldier
**Mana cost**: {3}{W} — **P/T**: 3/2
**Rulings** (1, 2011-09-22): "If you control the only Vampire, Werewolf, or Zombie, you must target it with Slayer of the Wicked's ability. You choose whether or not to destroy the target when the ability resolves."

**Status**: ISSUE (fixed) — the card code is correct; its ruling had no test.

### Card data
Matches the fetched text: `{3}{W}`, `card_types: [Creature]`,
`subtypes: ["Human", "Soldier"]` (both), 3/2, oracle text verbatim in the
current "When this creature enters" errata wording, no keywords. One
`TriggeredAbilityDef` of kind `EntersBattlefield` matching the one implemented
hook, with `has_etb_handler()` returning true.

### How it meets its ruling
The ruling has the same two moments as Geistcatcher's Rig, and the card is built
the same way: `target_requirement: CreatureWithFilter(SubtypeOrCardType {
subtypes: ["Vampire", "Werewolf", "Zombie"], card_types: [] })` on the
`TriggeredAbilityDef`, so the engine locks the target when the trigger goes on
the stack (CR 603.3d) — and with only one legal target there is nothing to
choose, which is the ruling's "you must target it". `on_enter_battlefield` then
offers only the may-decision from the locked target.

The filter carries no "you don't control": the printed text has no such
restriction, and the ruling's whole point is that your own creature is a legal
target.

### Code issues

No issue in `slayer_of_the_wicked.rs`. The ruling had no test, in either half;
both of these mutations passed the entire workspace:

1. **"you must target it" — your own creature is targetable**
   - Ruling says: `If you control the only Vampire, Werewolf, or Zombie, you must target it`
   - Replacing the subtype filter with `TargetFilter::YouDontControl` produced
     zero failures. Every existing test puts the victim on the opponent's side:
     `slayer_of_the_wicked_destroys_zombie` (P1's Walking Corpse),
     `slayer_of_the_wicked_sees_instance_vampire` (P1's granted Vampire),
     `bug_at_slayer_of_the_wicked_targets_vampire_token` (P1's token),
     `an_etb_trigger_does_not_offer_an_opponents_hexproof_creature` (P1's).

2. **"You choose whether or not to destroy" — declining**
   - Ruling says: `You choose whether or not to destroy the target when the ability resolves`
   - Adding an unconditional `try_destroy` *before* the prompt was even raised
     produced zero failures: the existing destroy test answers yes, so nothing
     distinguished "destroyed because you said yes" from "destroyed regardless".
   - (The related mutation — making the choice mandatory rather than optional —
     *is* caught, by `slayer_of_the_wicked_destroys_zombie`'s
     `awaiting_action.is_some()` assertion, because a mandatory choice with one
     target auto-applies and never prompts.)

Added `slayer_of_the_wicked_must_target_your_own_zombie_and_may_spare_it`,
which covers both: the only Vampire/Werewolf/Zombie is P0's own Walking Corpse,
with an opponent's plain 3/3 standing by so "the only one" is a claim about the
filter and not about an empty board. It reads the offered options and asserts
they are exactly `[your own Zombie]`, then declines, and the Zombie lives.

### Tricky interactions checked
- Your own creature is a legal target (the ruling): PASS — new test.
- Declining destroys nothing: PASS — new test.
- Answering yes destroys it: PASS — `cards_death_triggers_and_tokens.rs:138`.
- A creature that is none of the three is not offered: PASS — the new test's
  `options == [mine]` assertion; and dropping "Zombie" from the filter fails
  two tests.
- Runtime-granted subtype (Olivia's "becomes a Vampire") is a legal target:
  PASS — `cards_shortcuts_taken.rs:327`.
- A Vampire **token** is a legal target: PASS — `subtype.rs:207`.
- An opponent's **hexproof** creature is not offered: PASS —
  `hexproof_filter.rs:638`.
- "destroy", so indestructible survives: `PendingEffect::Destroy` routes to
  `destruction::try_destroy` (`engine/effects.rs:92`), the destroy pipeline —
  not exile, not sacrifice. Structural; the indestructible rule is covered by
  the destruction tests rather than per card.
- No legal target at all: the trigger is not put on the stack (CR 603.3c), the
  engine's; `chosen_targets.first()` returning `None` is a second guard.
- Target becomes illegal between trigger and resolution: CR 608.2b, generic.
- The Slayer itself is a Human Soldier, so it can never be its own target.

### UI presentation
Trigger description: "destroy target Vampire, Werewolf, or Zombie". Prompt:
"Slayer of the Wicked: you may destroy the targeted creature". Both name the
source and match the printed text.

### Test coverage
- Destroys the chosen target: `cards_death_triggers_and_tokens.rs:138`
  (`slayer_of_the_wicked_destroys_zombie`).
- The ruling, both halves:
  (`slayer_of_the_wicked_must_target_your_own_zombie_and_may_spare_it`) —
  **added this audit**.
- Runtime-granted Vampire: `cards_shortcuts_taken.rs:327`.
- Vampire token: `subtype.rs:207`.
- Hexproof creature not offered: `hexproof_filter.rs:638`.

### Mutations run
| mutation | result |
| --- | --- |
| filter restricted to `YouDontControl` | fails the new test (before: **nothing at all**) |
| destroy the target before the "you may" prompt | fails the new test (before: **nothing at all**) |
| mandatory instead of `you may` | fails `slayer_of_the_wicked_destroys_zombie` |
| drop "Zombie" from the filter | fails that test and the hexproof sweep |
| `TargetRequirement::Creature` instead of the filter | fails three tests |

Suite after: 1455 passing, exit 0, zero warnings.

