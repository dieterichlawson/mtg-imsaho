## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/177/elder-of-laurels?utm_source=api
**Type line**: `Creature — Human Advisor` — {2}{G}, 2/3
**Oracle text**:
```
{3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The number of creatures you control is counted as **the ability
  resolves**." Counting it in the activation hook counted at announcement
  instead — a creature that died in response was still counted, and one that
  arrived was not. Fixed by the CR 602.2a conversion: PASS
- Ruling: "Once the ability has resolved, the bonus won't change if the number
  of creatures you control changes later in the turn" — the count is baked into
  a `ModifyPT` value, not re-evaluated: PASS
- "Target **creature**", any creature, not just your own: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump lands and is a fixed number: `cards_activated_abilities.rs`
- Protection makes the target illegal: `ability_target_protection.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/177/elder-of-laurels?utm_source=api
**Type line**: `Creature — Human Advisor` — {2}{G}, 2/3
**Oracle text**:
```
{3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
```

**Rulings fetched**:
- [2011-09-22] The number of creatures you control is counted as the ability resolves.
- [2011-09-22] Once the ability has resolved, the bonus won’t change if the number of creatures you control changes later in the turn.

**Status**: PASS

### Code issues

No issues found.

### Checked against each ruling

- `The number of creatures you control is counted as the ability resolves.` — PASS. The count is taken in `resolve_activated_ability`, which is the resolution hook; there is no announcement-time capture. `test_suite_guards::no_card_or_test_names_the_removed_activation_hook` exists because this card once counted at announcement, and it keeps that hook from coming back.
- `Once the ability has resolved, the bonus won't change if the number of creatures you control changes later in the turn.` — PASS. What is pushed is `TemporaryEffect::ModifyPT { power_mod: creature_count, toughness_mod: creature_count }` — a number fixed at resolution, not a live count.

### Checked and correct

- Cost `{2}{G}`, `Creature — Human Advisor`, 2/3, oracle text verbatim.
- Ability cost `{3}{G}`, no tap, no sorcery-speed restriction, no once-per-turn — matching an ability the card states with none of those.
- `TargetRequirement::Creature` with no filter: "target creature", not "target creature you control". An opponent's creature is a legal target.
- The Elder counts itself among "creatures you control" while it is still there, and stops when it is not.
- "you" is `helpers::controller_of`, the last known controller (CR 608.2g), so an Elder destroyed in response still resolves its ability for the right player.
- `is_creature` reads the active face plus runtime grants, so an animated noncreature permanent is counted while it is a creature.
- The until-end-of-turn bonus is dropped if the target leaves the battlefield and returns the same turn (`move_object`, CR 400.7), so a reanimated creature does not inherit it.
- CR 608.2b is enforced by the engine for activated abilities (`stack.rs`), which re-checks both targetability and the card's own `is_valid_target` before resolving; the card's own battlefield check is a second line rather than the only one.

### Tricky interactions checked

- Creature count changes between activation and resolution: X is the resolution count. PASS.
- Creature count changes after resolution: the bonus stays. PASS.
- Elder destroyed in response: ability resolves, X excludes the Elder. PASS.
- Targeting an opponent's creature: legal, and offered.
- Target gains protection/hexproof after being targeted: the engine fizzles the ability (CR 608.2b), covered set-wide by `ability_target_protection.rs`, which uses this card as its `TargetRequirement::Creature` case.

### Test coverage

- pumps by the creature count: `cards_activated_abilities.rs:22`
- protection stops it being targeted: `ability_target_protection.rs:66`
- autotap pays `{3}{G}`: `equipment_autotap.rs:261`
- effect happens at resolution, not activation (guard): `test_suite_guards.rs:849`
- X counted at resolution, not announcement: `cards_activated_abilities.rs` `elder_of_laurels_counts_creatures_when_the_ability_resolves` (NEW, mutation-checked)
- bonus does not follow the count afterwards: `cards_activated_abilities.rs` `elder_of_laurels_bonus_does_not_follow_the_creature_count` (NEW, mutation-checked for non-vacuity by an off-by-one; the fixed-versus-live property is structural, since the effect stores an `i32`)
- Elder killed in response no longer counts itself: `cards_activated_abilities.rs` `elder_of_laurels_killed_in_response_no_longer_counts_itself` (NEW, mutation-checked)

