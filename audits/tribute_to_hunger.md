## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/119/tribute-to-hunger?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target **opponent** sacrifices a creature **of their choice**" — the choice
  is the opponent's, not the caster's, and `is_valid_target` rejects the caster:
  PASS
- Sacrifice, not destroy, so indestructible does not save it: PASS
- Ruling: "Use the sacrificed creature's toughness **as it last existed on the
  battlefield** to determine how much life to gain" — last known information
  (CR 608.2g): PASS
- The life gain goes through `gain_life`: PASS
- An opponent with no creatures sacrifices nothing and you gain nothing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The opponent's choice and the life gain: `sacrifice_choice.rs`, `cards_removal.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/119/tribute-to-hunger?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
Target opponent sacrifices a creature of their choice. You gain life equal to that creature's toughness.
```

**Rulings fetched**:
- [2024-11-08] Use the sacrificed creature's toughness as it last existed on the battlefield to determine how much life to gain.

**Status**: ISSUE (1 in this card; 26 across the shapes the sweep turned up)

### Code issues found and fixed

**In this card: one dead fallback that invents a target.**

```rust
let opponent = match targets.first() {
    Some(Target::Player(pid)) => *pid,
    _ => state.opponent(controller),
};
```

- Oracle text says: `Target opponent sacrifices a creature of their choice.`
- Code does: if no target is present, substitutes an opponent the caster never
  declared.

The rule for a target that stopped being legal is CR 608.2b — the spell does
not resolve at all — and `stack::resolve_spell` applies it before `on_resolve`
is ever called, so the branch is unreachable *and* states the wrong rule. It is
now `let Some(&Target::Player(opponent)) = targets.first() else { return };`.
Everything else about the card was correct.

### What the sweep for that fallback turned up

`state.opponent(controller)` appears in five other cards. Three are documented
defaults that read the real source first (Grimgrin, Kessig Cagebreakers) or are
the engine's two-player win bookkeeping (Laboratory Maniac). One was not:

**Geist of Saint Traft** — "Whenever Geist of Saint Traft attacks, create a 4/4
white Angel creature token with flying that's tapped and attacking." Its
`on_attacks` hook takes an `AttackInfo` carrying `defending_player`, ignored it
as `_attack`, and wrote `state.opponent(controller)` instead. With two players
those agree; with three, the Angel attacks the wrong player. Fixed to use the
information the trigger already carries.

Its existing test `the_angel_token_attacks_whoever_geist_is_attacking` could
not have caught this — in a two-player game both readings give the same answer.
The new test builds a three-player game where the next player and Geist's
defender are different, and fails against the old code.

### And the cluster underneath that

Geist's hook also opened with `Some(o) => (o.controller, o.card_id), None =>
return`, and a scan found **twenty-five** sites across the card set reading
`o.controller` off their own source while resolving. Two rules say not to, and
they agree: CR 608.2g (an ability resolving after its source has left uses the
source's *last known* controller — and leaving the battlefield resets
`controller` to `owner`, so the field is not that), and CR 602.2a for activated
abilities (the controller is the activator).

Most of them carried a comment stating exactly the rule they were breaking:

- `abattoir_ghoul.rs`: "CR 603.6d: triggered ability resolves even if source
  has left the battlefield (e.g. simultaneous death in combat)"
- `moldgraf_monstrosity.rs`: "CR 603.10c: 'your' means last-known controller,
  not owner" — and it is a *dies* trigger, so its source is always in the
  graveyard by the time it reads the field. "Return two creature cards at
  random from your graveyard" looked in the owner's.
- `woodland_sleuth.rs`: "We still know who controlled it when it entered"
- `burning_vengeance.rs`: "CR 113.7a: the trigger is independent of Burning
  Vengeance once on the stack, so destroying it in response still deals the 2
  damage" — above a `caster != controller` check against the reset field, which
  bails after the Vengeance is destroyed.

Every one also paired the read with `None => return`, throwing the effect away
if the source had gone — which CR 113.7a forbids outright.

All converted: `helpers::controller_of` for triggers, `helpers::ability_controller`
for the two loyalty abilities (Garruk Relentless, Liliana of the Veil).

The guard added during the Skirsdag High Priest audit only covered
`resolve_activated_ability`; it is now
`no_card_reads_its_controller_off_its_own_source_while_resolving` and covers
every hook, exempting the ones that answer "what is true of this permanent
right now" — a static or replacement effect, a trigger *condition*, and the
dual lands' enters-tapped check all run while the source is on the battlefield
(CR 113.6), where the two answers coincide.

### Why fifteen existing tests did not catch any of it

`trigger_source_independence.rs` exists for precisely this rule and has a
helper that stacks a trigger, kills the source, and resolves. It gave the
source the same owner and controller — and the reset sets `controller = owner`,
so the raw read and `controller_of` returned the same player. Every test in the
file passed against the broken code.

The helper now gives the source a different owner. Two tests need the old
behaviour and say why: a card goes to its *owner's* graveyard (CR 404.3), so
Kessig Cagebreakers counting itself among the creature cards in its
controller's graveyard, and Curse of the Bloody Tome's count of the cursed
player's graveyard, are about which graveyard the source lands in. They use
`resolve_after_source_dies_into_its_controllers_graveyard`.

Six assertions in the file counted tokens by name without asking who controls
them, which is the same blindness one level up; they now use
`count_tokens_named_by`.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{2}{B}` | `Generic(2), Colored(Black)` OK |
| type | `Instant` | `[CardType::Instant]`, no P/T OK |
| oracle text | verbatim match | OK |
| targeting | "target opponent" | `TargetRequirement::PlayerOnly` plus an `is_valid_target` that rejects the caster OK |

### Tricky interactions checked

- **"of their choice"** — the opponent chooses, not the caster. **Pass**; the
  choice is presented to `opponent`, and it is mandatory. Was only tested with
  a single creature, where `present_target_choice` auto-applies and the prompt
  never appears, so nothing checked *whose* choice it is. Now tested.
- **Ruling 2024-11-08: "Use the sacrificed creature's toughness as it last
  existed on the battlefield."** **Pass** — `effective_toughness` is read
  before `sacrifice`, so counters and pumps count. Now tested.
- **Sacrifice, not destroy** — indestructible does not save the creature and it
  cannot regenerate (CR 701.17a). **Pass**, now tested.
- **The creature is chosen, not targeted** — hexproof does not protect it.
  **Pass**, now tested. `creatures_controlled_by` correctly does no hexproof
  filtering.
- **The opponent has no creatures** — the spell still resolves (its target, the
  opponent, is legal) and does as much as it can, which is nothing, and no life
  is gained. **Pass**, tested.
- **The spell does not clean itself off the stack.** **Pass** — no
  `move_object` or `move_spell_after_resolve`; the engine finishes it once the
  choice chain completes (CR 608.2m).
- **"You gain life"** — `controller_of` on the spell, which is right for a
  spell: there is no activator to record.
- **Toughness 0 or less.** Guarded by `if toughness > 0`, and unreachable — a
  creature with toughness 0 is already dead by state-based action.

### Test coverage

- the opponent sacrifices and the caster gains life:
  `cards_sacrifice_and_additional_costs.rs::tribute_to_hunger_opponent_sacs_and_gain_life`
- no creatures, nothing happens: `::tribute_to_hunger_no_creatures_does_nothing`
- "target opponent" is not "target player":
  `characteristics_targeting.rs:102`
- **the opponent picks which creature, and the caster does not**:
  `::tribute_to_hunger_lets_the_opponent_pick_which_creature` (new)
- **the ruling — toughness as it last was on the battlefield**:
  `::tribute_to_hunger_gains_life_for_the_toughness_it_last_had` (new)
- **indestructible does not stop a sacrifice**:
  `::tribute_to_hunger_takes_an_indestructible_creature` (new)
- **hexproof does not protect a chosen creature**:
  `::tribute_to_hunger_can_take_a_hexproof_creature` (new)
- Geist's Angel attacks Geist's defender, on a board where that is not the next
  player: `geist_of_saint_traft.rs::the_angel_attacks_geists_defender_and_not_just_the_next_player` (new)

Everything mutation-checked. The three Tribute mutations (caster chooses, read
toughness after the sacrifice, `try_destroy` instead of `sacrifice`) each fail
the test named for it; reverting Geist's defender fails the three-player test;
and reverting the controller read on Sturmgeist, Undead Alchemist, Gutter
Grime, Endless Ranks of the Dead or Geist now fails its source-independence
test, which it would not have before the helper change. Kessig Cagebreakers'
remains non-discriminating, unavoidably: its scenario needs owner and
controller to agree.
