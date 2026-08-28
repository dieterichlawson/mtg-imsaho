## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/243/kessig-wolf-run?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{X}{R}{G}, {T}: Target creature gets **+X/+0**" — X is the amount funded, and
  CR 107.3e means X is 0 in a cost paid other than by casting only for costs
  that are not announced; here X is announced, so the funding prompt is correct:
  PASS
- X = 0 is a legal activation: trample with no pump: PASS
- Trample until end of turn: PASS
- The pump lands on resolution, not on activation (CR 602.2a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both X values and the trample: `cards_lands_and_mana_sources.rs:x_equals_0_gives_trample_only`, `:x_equals_3_gives_plus_3`, `cards_rule_modifiers.rs:kessig_wolf_run_grants_power_and_trample`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/243/kessig-wolf-run?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE


No rulings are cached for this card and none surfaced.

### Code issues
No behavioural bug. Card data matches exactly: Land with no mana cost, both
abilities present, `{T}: Add {C}` as a `ManaAbilityDef` producing colorless,
and `{X}{R}{G}, {T}` as an activated ability targeting a creature at instant
speed. The pump reads `last_activated_x_value`, applies `+X/+0` and Trample,
both until end of turn.

**One structural change**, the same one as Heretic's Punishment: the target's
zone legality was checked inside `resolve_activated_ability` rather than in
`is_valid_target`, which is the hook `stack.rs`'s ability arm actually calls
before deciding whether to counter the ability. Moved. Confirmed
load-bearing first — neutralizing the new `is_valid_target` lets the ability
resolve against a creature in the graveyard.

### A correction to the previous audit
The Heretic's Punishment entry I wrote immediately before this one said that
card was "the only card in the set" guarding its target from inside its
resolution handler. **That is wrong.** Elder of Laurels, Kessig Wolf Run and
Silverchase Fox do the same and likewise define no `is_valid_target`.

I have amended `audits/heretics_punishment.md` with a marked correction rather
than editing the claim away, and corrected the code comment that repeated it.
The refactor there was still the right change; it just fixed one instance of a
shared pattern rather than a lone outlier.

Two cards matched my first grep for this pattern and do not belong on the list:
Mindshrieker's `Zone::Battlefield` check is on its own source, not its target,
and Ghost Quarter's is in its search-destination and filtering code. That is
why the grep said seven and the real answer is four.

Elder of Laurels and Silverchase Fox are left for their own audits — I have not
fetched their oracle text this session, and the procedure is explicit that I
must not judge a card against text I have not fetched.

### Tricky interactions checked
- X = 3 gives +3/+0 and trample; X = 0 gives trample only: pass
- Activatable with just {R}{G} (X = 0), not with {R} alone: pass
- CR 602.2h, one tap pays one cost — the land's own `{T}: Add {C}` cannot fund
  its `{T}` ability: pass (`tap_cost_legality.rs:210`, the five-utility-land
  sweep)
- The pump does not apply until the ability resolves from the stack
  (CR 602.2a): pass (`activated_no_stack.rs:81`)
- X is not auto-picked for the player: pass (`auto_pick.rs:543`)
- "Target creature" with no "you control" — an opponent's creature is offered:
  pass
- CR 608.2b, target gone by resolution: nothing applied: pass

### Test coverage
- X arithmetic and mana requirements: `cards_lands_and_mana_sources.rs:699`,
  `:721`, `:743`, `:786`
- Ability uses the stack: `activated_no_stack.rs:81`
- X funding is a player choice: `auto_pick.rs:543`
- One tap pays one cost: `tap_cost_legality.rs:210`
- **NEW** an opponent's creature is a legal target:
  `cards_lands_and_mana_sources.rs:820`
- **NEW** target gone by resolution, nothing applied:
  `cards_lands_and_mana_sources.rs:840`

### Two assertions strengthened
Both existing trample checks confirmed the grant by finding the
`until_end_of_turn` entry rather than asking `has_keyword`. The entry existing
and the engine honouring it are two different claims, and only the second is
what the card promises — the same weakening I corrected on Manor Gargoyle.
Mutation-checked: granting a different keyword now fails both.

