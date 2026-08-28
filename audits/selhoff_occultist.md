## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature or another** creature dies" — both kinds declared,
  same as Falkenrath Noble.
- "target player mills a card" — targeted, so the target is locked when the
  trigger goes on the stack (CR 603.3d).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **this creature or another creature** dies" — declared as **two**
  trigger kinds, `SelfDies` *and* `AnyCreatureDies`, so it fires on its own death
  as well as on others'. Murder of Crows, whose text says "whenever **another**
  creature dies", declares only the second — the distinction is in the card data,
  not buried in a handler: PASS
- "**target player** mills a card" — targeted, so it can be pointed at yourself
  or an opponent: PASS
- The mill goes through the pipeline, so a creature card emits
  `CreatureCardMilled` and an opponent's Undead Alchemist sees it: PASS
- CR 113.7a: its own death does not counter the trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both trigger kinds and the mill: `cards_morbid_and_ltb.rs`, `multi_target_and_mill.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE


No rulings are cached for this card and none surfaced.

### Code issues
No behavioural bug. Card data matches exactly — {2}{U}, Creature — Human Rogue
(both subtypes), 2/3, oracle text verbatim.

Same shape as Falkenrath Noble: "this creature or another creature dies" is two
`TriggeredAbilityDef`s, `SelfDies` and `AnyCreatureDies`, both with
`PlayerOnly`. That does not double-trigger on its own death because the
collector's death-watch arm excludes the dead creature from watching itself.
The mill goes through `engine::mill_cards`, and `on_any_creature_dies`
deliberately carries no battlefield guard on the Occultist — a board wipe that
kills it alongside the creature it is watching is the case the trigger exists
for.

### Wrong rule citations, corrected across the tree
The card cited **CR 603.3b** three times for "the target is chosen when the
trigger goes on the stack". The engine uses that number for something else:
`triggers/collect/mod.rs` cites 603.3b for APNAP ordering of simultaneous
triggers, twice and in detail, while `state.rs` and `engine/effects.rs` — the
machinery that actually attaches a chosen target to a pending trigger — cite
**603.3d**. One number cannot mean both, and the engine core plus the majority
of cards agree on 603.3d for target choice.

Corrected at nine sites: this card (×3), Elder Cathar (×2), Bloodgift Demon
(×2), and four test comments. Every surviving 603.3b in the tree is now the
ordering claim — `collect/mod.rs`, `apnap.rs`, and `trigger_dispatch.rs:604`,
which is explicitly about the active player choosing stack order.

### A stale to-do that had already been done
`trigger_dispatch.rs` carried a note saying the `o.zone == Zone::Battlefield`
early-return gate "exists in Murder of Crows, Rage Thrower, and Selhoff
Occultist... the other three need the same one-line fix". None of the four has
that gate any more, and each of the other three now has its own regression test
in `trigger_source_independence.rs`. Rewritten to say so and to point at those
tests. A comment directing work that is already finished sends the next reader
looking for a guard that is not there.

### A test I wrote wrong, and corrected
I first wrote the Undead Alchemist test as proof that the Occultist mills
"through the shared pipeline rather than moving the card by hand", and named it
accordingly. Mutation showed otherwise: replacing `mill_cards` with a hand-
rolled library-to-graveyard move left the test passing.

The reason is that `state.rs::move_object` emits `CreatureCardMilled` for *any*
library-to-graveyard move of a creature card, on purpose — its comment says
"being one is a property of the zone change, not of the caller having
remembered a helper". So no test can distinguish the two paths through that
event, and mine never did.

Rewritten to claim only what it shows: the cross-card interaction, and that the
*targeted* player mills rather than the Occultist's controller. A second
creature card in P0's library makes that second claim real rather than the only
card that could have moved.

### Tricky interactions checked
- An ally dying mills the target one card: pass
- The Occultist's own death mills once, not twice: pass
- It mills after dying alongside the creature it watched (CR 113.7a): pass
  (`trigger_source_independence.rs:600`)
- The target is the player chosen, not the controller: pass
- A player with hexproof is not offered as a target: pass
  (`hexproof_filter.rs:298`)
- A creature card it mills is visible to Undead Alchemist: pass

### Test coverage
- Mills after dying alongside the creature: `trigger_source_independence.rs:600`
- Hexproof target filtering: `hexproof_filter.rs:298`
- Shared zone-gate regression: `trigger_dispatch.rs:169`
- **NEW** one card per death, both arms of "this creature or another":
  `cards_death_triggers_and_tokens.rs:566`
- **NEW** the milled card reaches Undead Alchemist, and the target milled:
  `cards_death_triggers_and_tokens.rs:592`

Mutation-checked: letting the dead creature watch its own death, and milling
the controller instead of the target, each fail the tests they should.

