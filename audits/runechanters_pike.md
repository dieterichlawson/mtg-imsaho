## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/231/runechanters-pike?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.
Equip {2}
```

**Status**: ISSUE

### Code issues
See below.


### Tricky interactions checked
- Equipment enters unattached and stays on the battlefield when what it equipped
  leaves (CR 704.5n), rather than going to the graveyard as an unattached Aura
  would (CR 704.5m): PASS — and this is the one that was wrong. Being an
  Equipment was a per-object `is_equipment` bool that eleven cards set in an
  `on_resolve` override which otherwise only repeated the trait default's "move
  a permanent to the battlefield". An Equipment that reached the battlefield any
  other way left the flag false and was then read as an Aura. Now derived from
  the Equipment subtype (CR 301.5) through the characteristics layer, and the
  eleven dead overrides are gone.
- "Equip only as a sorcery" — `sorcery_speed_only: true`: PASS
- "Attach to target creature **you control**" — `TargetFilter::YouControl` and
  the card's own `is_valid_target`: PASS
- The equip ability is offered on the Equipment, not duplicated onto the
  creature it is attached to: PASS
- The attach happens on resolution, not on activation (CR 602.2a): PASS
- Ruling: "The value of X is constantly updated as instant cards and sorcery
  cards are put into or removed from your graveyard" — `dynamic_pt`, recomputed
  every time P/T is asked for, not a snapshot: PASS
- "**your** graveyard" is the Pike's controller's, not the equipped creature's.
  `dynamic_pt` is called with the Pike's own object id, so `obj.controller` is
  the right player even when the Pike is on an opponent's creature: PASS
- "instant and sorcery **cards**" — CR 109.1, `state.is_card`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Equip cost, bonus, and reattachment: `cards_equipment_costs.rs`, `equipment_autotap.rs`
- Detaching rather than dying: `cards_equipment_costs.rs:an_equipment_that_did_not_resolve_as_a_spell_still_detaches_rather_than_dying`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/231/runechanters-pike?utm_source=api
**Type line**: `Artifact — Equipment` — {2}
**Oracle text**:
```
Equipped creature has first strike and gets +X/+0, where X is the number of instant and sorcery cards in your graveyard.
Equip {2}
```

**Rulings fetched**:
- [2011-09-22] The value of X is constantly updated as instant cards and sorcery cards are put into or removed from your graveyard.

**Status**: PASS (behaviour correct; equip deduplicated from eleven copies to one, and four gaps in coverage closed)

### Code issues

**The card's behaviour is correct.** Every field matches the fetched text and
the one ruling is satisfied. What I found is structural: equip is one rules
action and it was written out eleven times.

**Eleven copies of CR 702.6b.** Every Equipment in the set ended its equip
resolution with the same four lines —

```rust
if let Some(Target::Object(creature_id)) = targets.first() {
    if let Some(obj) = state.get_object_mut(object_id) {
        obj.attached_to = Some(*creature_id);
    }
}
```

— and ten of them carried a byte-identical `is_valid_target` above it. Auras
have had `helpers::resolve_aura` since the beginning; equip had no counterpart,
so there was nowhere for the two rules that are *not* in those four lines to
live. Both now do, in `helpers::resolve_equip` and
`helpers::equip_target_is_legal`, and a guard
(`card_data_invariants.rs::no_equipment_attaches_itself_by_hand`) fails the
build on a card that sets `attached_to` by hand.

Being exact about what that did and did not fix, since I mutation-tested each
piece separately:

- The **predicate** is load-bearing. The engine's CR 608.2b re-check runs
  `is_target_legal` plus the card's `is_valid_target`, and for a
  `CreatureWithFilter` requirement the former only re-runs the *filter* — it
  accepts a target in the Stack zone and asks nothing about creature-ness. So
  "still a creature, still on the battlefield, still yours" is the card's to
  answer. Dropping the battlefield half fails the new test.
- The **same check inside `resolve_equip`** is belt-and-braces on today's
  paths: disabling it alone changes nothing, because the engine has already
  fizzled the ability by then. I have kept it — it is where the attachment
  actually happens — but it is not closing a live hole and I am not claiming it
  is.
- The **CR 301.5c line** (an Equipment that is also a creature does not become
  attached) is unreachable in this set: nothing animates an Equipment, and no
  test fails without it. It is one line in one place now instead of eleven
  cards that would each need it.

Blazing Torch keeps its own `is_valid_target`, which covers a second ability
that targets a creature or a player; only its attach went to the helper.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{2}` | `Generic(2)` OK |
| type | `Artifact - Equipment` | `Artifact`, `["Equipment"]` OK |
| P/T | none | none OK |
| static ability | "Equipped creature has first strike" | `ContinuousEffect::GrantKeyword { FirstStrike, scope: Attached }` OK |
| dynamic bonus | "+X/+0, where X is the number of instant and sorcery cards in your graveyard" | `dynamic_pt` returning `(count, 0)` OK |
| equip | `Equip {2}` | `{2}`, `sorcery_speed_only`, `CreatureWithFilter(YouControl)` OK |
| oracle text | verbatim match | OK |

Scryfall lists `Equip` under keywords. No Equipment in this set declares it as
a `Keyword`; equip is implemented as an activated ability, which is what
CR 702.6b says it is. Consistent across all eleven, so not a finding.

### Tricky interactions checked

- **Ruling 2011-09-22: "The value of X is constantly updated as instant cards
  and sorcery cards are put into or removed from your graveyard."** **Pass** —
  `dynamic_pt` is computed on demand rather than stamped. The existing test
  covered cards going *in*; coming out was untested and now is.
- **"your graveyard" is the Pike's controller's, not the equipped creature's.**
  **Pass**, and non-obviously so: `state.rs` calls `dynamic_pt` with the
  *Equipment's* id (`behavior.dynamic_pt(self, source.id, registry)` while
  scanning things attached to the creature), so `obj.controller` is the Pike's.
  An opponent stealing the equipped creature does not redirect the count. Was
  untested; now is.
- **"instant and sorcery cards"** — a creature card in the graveyard adds
  nothing, and neither does a token, which is not a card (CR 109.1) and sits in
  a graveyard until the next state-based action pass. Both untested; now
  tested, and dropping the `is_card` filter fails it.
- **The Equipment's own P/T.** The Pike is not a creature, so its `dynamic_pt`
  is never consulted for itself. **Pass**, guarded by a unit test in `state.rs`.
- **Equip targeting a creature that dies in response.** **Pass** — the ability
  fizzles under CR 608.2b and the Pike stays unattached. Untested; now tested.
- **Equip is sorcery-speed only** (CR 702.6b). **Pass**, declared.
- **An animated Equipment cannot equip** (CR 301.5c). Now stated once, in the
  helper. Unreachable in this set.

### Test coverage

- first strike and the +X/+0, counting cards as they arrive:
  `cards_equipment_and_artifacts.rs::runechanters_pike_grants_first_strike_and_power_bonus`
- the Equipment's dynamic P/T does not become its own:
  `state.rs::equipment_dynamic_pt_does_not_leak_into_own_effective_pt`
- **what counts and what does not — creature cards, tokens, an opponent's
  graveyard — and X falling as a card leaves**:
  `::runechanters_pike_counts_only_instants_and_sorceries_in_its_own_graveyard` (new)
- **the count follows the Pike's controller, not the stolen creature's**:
  `::runechanters_pike_counts_its_own_controllers_graveyard_after_the_creature_is_stolen` (new)
- **equip does nothing if its target left in response**:
  `::equip_does_not_attach_to_a_creature_that_left_in_response` (new)

Mutation-checked: counting the opponent's graveyard fails all three counting
tests, dropping `is_card` fails the token assertion, and dropping the
battlefield half of the equip predicate fails the response test. The two
belt-and-braces pieces noted above do not fail anything, and I have said so
rather than counted them.
