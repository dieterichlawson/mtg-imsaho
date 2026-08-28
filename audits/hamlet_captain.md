## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/187/hamlet-captain?utm_source=api
**Type line**: `Creature — Human Warrior` — {1}{G}, 2/2
**Oracle text**:
```
Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever this creature attacks **or blocks**" — two declared triggers and two
  hooks, so both directions fire: PASS
- "**other** Humans you control" — the Captain excludes itself: PASS
- The set of Humans is fixed when the trigger resolves (CR 611.2c), so one
  arriving later gets nothing: PASS
- `has_subtype` reads granted subtypes, so a Human token counts: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both triggers and the self-exclusion: `combat_rules.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/187/hamlet-captain?utm_source=api
**Type line**: `Creature — Human Warrior` — {1}{G}, 2/2
**Oracle text**:
```
Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (1 in this card; 3 across the shape it shares)

### Code issues found and fixed

**One bug in this card, and it turned out to be a shape shared by two others.**

`buff_humans` opened with:

```rust
let controller = match state.get_object(self_id) {
    Some(o) if o.zone == Zone::Battlefield => o.controller,
    _ => return,
};
```

- Oracle text says: `Whenever this creature attacks or blocks, other Humans you
  control get +1/+1 until end of turn.`
- Code does: returns without doing anything if Hamlet Captain is no longer on
  the battlefield when the trigger resolves.

Nothing in that sentence is about the Captain. CR 113.7a: once the trigger is
on the stack it exists independently of its source, so killing the Captain in
response does not counter it. CR 608.2g: "you" is then the source's last known
controller — and `o.controller` is reset to `o.owner` on leaving the
battlefield, so even the value being read is wrong in the case the guard was
supposedly handling. Removal in response to the attack trigger cancelled the
pump for the entire rest of the team.

Fixed: `helpers::controller_of(state, self_id)`, no bail.

### The shape, and the two other cards with it

The expression reads as "who controls my source" and behaves as "do nothing if
my source is gone". Those are two different questions, and conflating them is
only harmless when the ability's effect is about the source anyway. A scan of
`src/cards` found 14 battlefield-guarded match arms; three were harmful:

| card | oracle | what the guard cost |
|---|---|---|
| Hamlet Captain | "other Humans you control get +1/+1 until end of turn" | the whole pump |
| Ghoulraiser | "When this creature enters, return a Zombie card at random from your graveyard to your hand" | removal in response ate the card advantage as well as the body |
| Rakish Heir | "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it" | read the reset `controller`, so after trading in the same damage step it compared the attacking Vampire against the wrong player |

Rakish Heir is worth singling out: its own comment already said "CR 113.7a: the
Heir trading with a blocker in the same combat damage step does not counter
this", and the code beneath it read `o.controller` off an object that had by
then had `controller` reset to `owner`. Its existing test in
`trigger_source_independence.rs` did not catch that because it gave the Heir
the same owner and controller, so both readings gave the same answer; the test
now owns it to P1 and controls it with P0, and fails against the old code.

Ghoulraiser has a pleasant consequence once it is fixed: a Ghoulraiser killed
in response to its own enters trigger is itself a Zombie card in that
graveyard by the time the trigger resolves, so it is one of the candidates to
be returned at random.

The other eleven sites were correct, for two different reasons, both now said
out loud rather than left to the reader:

- **The effect is about the source** (Champion of the Parish, Unruly Mob,
  Cloistered Youth, Delver of Secrets, Screeching Bat x2, Thraben Sentry,
  Civilized Scholar, Tree of Redemption, Mirror-Mad Phantasm). Rewritten as
  `if !helpers::still_on_battlefield(..) { return }` followed by
  `helpers::controller_of(..)` — same behaviour, two questions asked
  separately. Most did not even need the guard: `add_counters` and
  `apply_transform` already decline on a permanent that is not there.
- **The guard is on something other than the source**: Ghost Quarter checks
  its *target land* (CR 608.2b), and Undead Alchemist's is inside
  `replace_event`, where a battlefield check is genuinely required because a
  replacement effect functions only while its source is on the battlefield
  (CR 113.6) — unlike an ability already on the stack.

New helper `helpers::still_on_battlefield`, and a new guard
`card_data_invariants.rs::no_card_conflates_its_controller_with_still_being_on_the_battlefield`,
which fails the build on a battlefield-guarded match arm yielding the source's
controller. It exempts the hooks that answer "what is true of this permanent
right now" — `replace_event`, `activated_abilities`, `continuous_effects`,
`is_valid_target`, `dynamic_pt`, the `should_*` predicates — since for those
the battlefield check *is* the right question (CR 113.6). It also ignores a
guard on an object other than the source, and a `controller ==` comparison
between two objects.

The earlier `no_card_reads_its_sources_controller_by_hand` guard missed all of
this because it only looked for the closure form (`|o| o.controller` with a
`PlayerId(0)` or `.unwrap()` fallback); the match form went straight past it.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{1}{G}` | `Generic(1), Colored(Green)` OK |
| type | `Creature - Human Warrior` | `Creature`, `["Human", "Warrior"]` OK |
| P/T | 2/2 | `Some(2)/Some(2)` OK |
| keywords | none | none OK |
| oracle text | verbatim match | OK |
| triggers | attacks, blocks | `TriggerKind::Attacks` + `TriggerKind::Blocks`, both with hooks OK |

### Tricky interactions checked

- Killed in response to its own attack trigger: **was broken, now fixed**.
- "other" excludes the Captain itself: **pass** (`o.id != self_id`).
- "you control" excludes an opponent's Humans: **pass**, and now tested.
- A transformed werewolf is no longer a Human and must not be pumped
  (CR 712.8d): **pass**, `has_subtype` reads the active face.
- Human tokens are pumped: **pass** — `has_subtype` falls back to the object's
  own subtypes for a faceless token.
- CR 611.2c, the affected set is fixed at resolution: **pass** — a Human that
  arrives afterwards gets nothing. Now tested.
- "until end of turn" ends at cleanup (CR 514.2): **pass**, now tested.
- `state.is_creature` filter alongside the Human check: redundant rather than
  wrong. Human is a creature type (CR 205.3m), so a Human that is not a
  creature does not arise in this pool. **Not a finding.**
- Blocking two attackers would fire the trigger twice, since
  `triggers/collect/combat.rs::blockers_declared` emits per (blocker, attacker)
  assignment. **Not reachable**: nothing in this card pool lets a creature
  block an additional creature, and I could not establish an authoritative
  ruling on the correct count, so I have left it alone rather than guess.

### Test coverage

- attacks -> other Humans pumped, self and non-Humans not:
  `cards_spells_and_enchantments.rs::hamlet_captain_buffs_humans_on_attack`
- blocks -> other Humans pumped:
  `cards_spells_and_enchantments.rs::hamlet_captain_buffs_humans_on_block`
- a transformed werewolf is not pumped:
  `characteristics_card_sweep.rs::hamlet_captain_does_not_buff_transformed_werewolves`
- killed in response still pumps the team:
  `trigger_source_independence.rs::hamlet_captain_pumps_the_team_after_being_killed_in_response` (new)
- an opponent's Humans are not pumped:
  `cards_spells_and_enchantments.rs::hamlet_captain_does_not_pump_an_opponents_humans` (new)
- affected set fixed at resolution, and expires at end of turn:
  `cards_spells_and_enchantments.rs::hamlet_captains_pump_covers_who_was_there_and_lasts_one_turn` (new)
- Ghoulraiser killed in response:
  `trigger_source_independence.rs::ghoulraiser_returns_its_zombie_after_being_killed_in_response` (new)
- Rakish Heir after trading in combat:
  `trigger_source_independence.rs::rakish_heir_gives_its_counter_after_trading_in_combat`
  (existing, rewritten to distinguish controller from owner)

All three card fixes and the new guard mutation-checked: reverting each one
fails the test named for it.
