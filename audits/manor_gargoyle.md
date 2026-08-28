## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/228/manor-gargoyle?utm_source=api
**Type line**: `Artifact Creature — Gargoyle` — {5}, 4/4
**Oracle text**:
```
Defender
This creature has indestructible as long as it has defender.
{1}: Until end of turn, this creature loses defender and gains flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}: This creature loses defender and gains flying until end of turn" — both
  halves, and losing defender is a keyword removal rather than a P/T change:
  PASS
- Indestructible is printed and is not affected by the ability: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Losing defender and gaining flying: `cards_activated_abilities.rs:manor_gargoyle_loses_defender_and_gains_flying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/228/manor-gargoyle?utm_source=api
**Type line**: `Artifact Creature — Gargoyle` — {5}, 4/4
**Oracle text**:
```
Defender
This creature has indestructible as long as it has defender.
{1}: Until end of turn, this creature loses defender and gains flying.
```

**Rulings fetched**:
- [2013-07-01] Lethal damage dealt to Manor Gargoyle while it has indestructible will stay marked on it that turn. If Manor Gargoyle loses indestructible after having been dealt lethal damage earlier in the turn, it will be destroyed.

**Status**: ISSUE


One ruling: "Lethal damage dealt to Manor Gargoyle while it has indestructible
will stay marked on it that turn. If Manor Gargoyle loses indestructible after
having been dealt lethal damage earlier in the turn, it will be destroyed."

### Code issues
No behavioural bug. Card data matches exactly — {5}, Artifact Creature —
Gargoyle, 4/4, Defender — and the conditional static is modelled the right way
round: `ContinuousEffect::when(SelfHasKeyword(Defender), GrantKeyword(
Indestructible, OnSelf))` is "as long as it has defender", recomputed rather
than latched, so the {1} ability costs it indestructible in the same breath as
defender. The activated ability is {1}, no tap, instant speed, repeatable, all
correct.

Two pieces of dead or duplicated code removed:

**1. `is_valid_target` on a card that takes no targets.** The card defined:

```rust
fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
    match target {
        Target::Object(id) => state.get_object(*id)
            .is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry)),
        ...
```

Manor Gargoyle's only ability is `target_requirement: None` and it has no
targeted spell, so nothing can reach this. It is not merely unused, it is
misleading: it reads as "any creature on the battlefield is a legal target",
which is not a restriction this card has and would have been the wrong answer
the moment it gained a targeted ability. Deleted, with a guard —
`card_data_invariants.rs::no_card_defines_is_valid_target_without_taking_a_target`
— so it cannot come back. Manor Gargoyle was the only card in the set doing
this.

**2. `EffectCondition::SelfHasKeyword` re-implemented `has_keyword`'s first
step.** It scanned `until_end_of_turn` for a `RemoveKeyword` on the keyword
and returned false, then called `has_keyword` — which opens by doing exactly
that scan. Two copies of "was this keyword removed this turn", free to
disagree. I found it by mutation: deleting the duplicate changed nothing, and
the tests only moved when the real check in `has_keyword` was touched. Now it
just asks.

The duplicate is not a recursion guard, which was the plausible reason to keep
it — it only short-circuits when the keyword is on the removal list, and in the
ordinary case still calls straight through. The recursion hazard is real
(`when(SelfHasKeyword(X), GrantKeyword(X))` overflows the stack, which I
reproduced accidentally while mutating) but it is handled by `walk_effects`
testing `want` before the condition, not by this block.

### Tricky interactions checked
- Indestructible while it has defender: pass
- Activating removes defender *and* with it indestructible: pass
- Both come back at end of turn: pass
- The ruling — lethal damage stays marked through indestructibility (CR 120.3)
  and kills once indestructible goes (CR 704.5g): pass
- The condition asks about a keyword without `has_keyword` re-entering itself:
  pass (`continuous_effects.rs:73`)
- Flying is actually granted, not merely queued as an effect: pass

### Test coverage
- The condition terminates rather than recursing: `continuous_effects.rs:73`
- Removed keywords restored at cleanup: `turn_structure.rs:386`
- Activation removes defender and grants flying:
  `cards_complex_creatures.rs:733`
- **NEW** losing defender loses indestructible, and both return next turn:
  `cards_complex_creatures.rs:764`
- **NEW** the ruling — dies to damage marked while it was indestructible:
  `cards_complex_creatures.rs:797`
- **NEW** guard: no card defines `is_valid_target` without taking a target:
  `card_data_invariants.rs:1180`

### What was untested before
The two halves of the card were tested separately — that it has indestructible
while it has defender, and that activating removes defender — but never
chained, and the chain is the card. The one ruling had no test at all, which
matters because it is the card's trap: the {1} ability is a way to kill your
own Gargoyle if it is already carrying lethal damage.

One existing assertion was also weakened by construction: it confirmed flying
by finding the `until_end_of_turn` entry rather than asking `has_keyword`. The
entry existing and the engine honouring it are two different claims, and only
the second is what the card promises. Now asked through the accessor.

