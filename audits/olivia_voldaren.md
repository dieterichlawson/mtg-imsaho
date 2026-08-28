## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/215/olivia-voldaren?utm_source=api
**Type line**: `Legendary Creature — Vampire` — {2}{B}{R}, 3/3
**Oracle text**:
```
Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Olivia Voldaren deals lethal damage to a creature with its first
  activated ability, that creature will become a Vampire before dying." Damage
  is marked and the subtype added inside one resolution; state-based actions run
  afterwards: PASS
- "**another** target creature" — `TargetFilter::Another`: PASS
- The Vampire subtype is an object-level grant, so the second ability recognises
  both printed Vampires and ones Olivia made: PASS
- Ruling: "If you activate Olivia Voldaren's last ability, and before that
  ability resolves you lose control of Olivia Voldaren, the ability will resolve
  with no effect." The control effect's duration is the engine's, keyed on
  "for as long as you control Olivia" — it ends both when she leaves and when
  someone else takes control of her (CR 611.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage/subtype/counter ability: `olivia_voldaren.rs`
- The control duration ends on a control change, not only on leaving: `control_durations.rs`, `control_change.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/215/olivia-voldaren?utm_source=api
**Type line**: `Legendary Creature — Vampire` — {2}{B}{R}, 3/3
**Oracle text**:
```
Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
```

**Rulings fetched**:
- [2017-03-14] If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying.
- [2017-03-14] If you activate Olivia Voldaren’s last ability, and before that ability resolves you lose control of Olivia Voldaren, the ability will resolve with no effect. You won’t gain control of the targeted Vampire.

**Status**: ISSUE (fixed)

### Code issues

Three found, all fixed.

1. **The steal went to whoever held Olivia at resolution, not to the player who activated it.** `olivia_voldaren.rs:130` (before the fix)
   - Oracle text says: `{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.`
   - Ruling says: `If you activate Olivia Voldaren's last ability, and before that ability resolves you lose control of Olivia Voldaren, the ability will resolve with no effect. You won't gain control of the targeted Vampire.`
   - Code did: `state.gain_control_while_source_controlled(*target_id, object_id, registry);`, and that helper reads `source_obj.controller` — Olivia's controller *now*. So taking Olivia in response did not stop the ability; it redirected it. Targeting your own Vampire makes it plain: the old code handed it to the thief.
   - Now the ability checks that its **activator** still controls Olivia, and does nothing otherwise (CR 611.2b: the duration is already over).

2. **A declared trigger with no handler behind it.** `olivia_voldaren.rs:29` (before the fix)
   - Code declared: `TriggeredAbilityDef { kind: TriggerKind::LeavesBattlefield, description: "return stolen creatures to their owners" }`
   - There is no `on_leave_battlefield` on this card — the comment in ability 1 records that the hand-rolled unwind was replaced by the engine's control-effect duration. The declaration outlived it, so every time Olivia left the battlefield an ability went on the stack, did nothing, and gave both players a priority window for it. "For as long as you control Olivia Voldaren" is a duration (CR 611.2b), not a triggered ability, and `expire_control_effects` ends it as a state-based action.

3. **The first ability re-checked protection and returned early.** `olivia_voldaren.rs:101` (before the fix)
   - Oracle text says: `Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.`
   - Code did: `if state.has_protection_from(*target_id, object_id, registry) { return; }`
   - The type change and the counter are unconditional and protection touches neither, so that early return skipped two things the card does. Protection's two real effects are handled where they belong: it stops the targeting, which the engine enforces at announcement and again at resolution (CR 608.2b), and it prevents the damage, which `damage::deal_damage` does.

**Engine-level follow-up.** Issue 1 is not Olivia's alone. `StackEntry::Ability` has recorded an `activator` since it was written, and resolution ignored it — `stack.rs` re-read the source's current `controller` for the CR 608.2b target re-check, and no card had any way to ask who activated the ability. Take the source in response and the ability became the thief's: its targets were re-checked for legality against them, and any card reading the controller answered with them. Resolution now uses `activator`, and publishes it as `state.resolving_ability_activator` for the duration of the call — the same shape as `resolving_trigger_from_back_face`.

### Checked against each ruling

- `If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying.` — PASS. State-based actions do not run mid-resolution (CR 117.5), so the damage, the type change and the counter all land while the creature is still on the battlefield; it dies at the next SBA check. Now tested at each of those three moments.
- `If you activate Olivia Voldaren's last ability, and before that ability resolves you lose control of Olivia Voldaren, the ability will resolve with no effect.` — the ruling issue 1 is about; now PASS.

### Checked and correct

- Cost `{2}{B}{R}`, `Legendary Creature — Vampire`, 3/3, `Flying`, `Supertype::Legendary`.
- Ability 0 is `CreatureWithFilter(Another)` — "another target creature", so Olivia cannot point it at herself; the resolution guard `if *target_id == object_id { return; }` is a second line behind that.
- Ability 1 is `CreatureWithFilter(HasSubtype("Vampire"))`, and the resolution check uses `state.has_subtype`, which sees both printed Vampires and creatures Olivia's other ability turned into Vampires.
- "Target Vampire" has no controller restriction: your own Vampire is a legal target, which is what the new test relies on.
- The Vampire type is written to `obj.subtypes` — the runtime-grant vector — which is the right home for "becomes a Vampire **in addition to** its other types", and it is cleared when the creature changes zones (CR 400.7).
- Damage goes through `apply_pending_effect` → `damage::deal_damage` with Olivia as the source.
- Neither ability is once-per-turn or sorcery-speed, and neither taps.

### Tricky interactions checked

- Lethal damage: Vampire first, death after. PASS.
- Lose Olivia in response to the steal: no effect. PASS.
- Keep Olivia: the steal works. PASS.
- Olivia leaves: stolen creatures go home, and nothing goes on the stack. PASS.
- Olivia stolen by the opponent: stolen creatures go home (the duration checks the *controller*, not just the zone). PASS — already covered by `expire_control_effects`.
- Olivia's bite surviving a Moonmist transform: PASS (`subtype.rs`, fixed under Instigator Gang).
- Stealing a creature Olivia bit earlier: PASS.

### Test coverage

- damage + Vampire type + counter: `olivia_voldaren.rs:14`
- cannot target herself: `olivia_voldaren.rs:41`
- steals a Vampire: `olivia_voldaren.rs:57`
- will not steal a non-Vampire: `olivia_voldaren.rs:75`
- stolen creatures return when she leaves: `olivia_voldaren.rs:93`
- ability 1's target filter is Vampire: `olivia_voldaren.rs:125`
- can target a creature she bit: `subtype.rs:258`
- the bite survives Moonmist: `subtype.rs:365`
- lost Olivia in response — no effect: `olivia_voldaren.rs` `olivia_steal_does_nothing_if_you_lost_olivia_in_response` (NEW, mutation-checked)
- kept Olivia — steal works: `olivia_voldaren.rs` `olivia_steal_still_works_while_you_keep_her` (NEW)
- Vampire before dying: `olivia_voldaren.rs` `olivia_makes_a_creature_a_vampire_before_it_dies` (NEW)
- nothing on the stack when she leaves: `olivia_voldaren.rs` `olivia_puts_nothing_on_the_stack_when_she_leaves` (NEW, mutation-checked)

