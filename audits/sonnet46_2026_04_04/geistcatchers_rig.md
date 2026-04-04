## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, you may have it deal 4 damage to target creature with flying.
**Type line**: Artifact Creature — Construct
**Status**: ISSUE

### Code issues

- **Target selection deferred to resolution instead of stack-placement** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` lines 40–59, `mtg-engine/src/triggers.rs` lines 344–364)
  - Oracle text says: `When this creature enters, you may have it deal 4 damage to target creature with flying.`
  - Scryfall ruling says: `The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability resolves.`
  - Code does: Target selection happens entirely inside `on_enter_battlefield`, which is called at trigger resolution (via `resolve_next_trigger` → `behavior.on_enter_battlefield`). The `collect_triggers` path stores no target in `PendingTrigger::EnteredBattlefield`; the struct has no target field. The complete combined "pick a target or decline" choice is presented at resolution, not at stack-placement.

- **`optional: true` conflates target selection with the "you may" decision** (`mtg-engine/src/cards/isd/geistcatchers_rig.rs` line 56)
  - Oracle text says (per ruling): target is mandatory at stack time (if legal targets exist); `You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability resolves.`
  - Code does: `ResolutionChoiceKind::ChooseTarget { ..., optional: true, ... }` — presents a single merged choice at resolution that lets the player pick a target OR pick `None` (skip). This allows declining the target at resolution time, which conflates two distinct steps: (1) mandatory target selection at stack placement, and (2) the "you may" damage decision at resolution. In correct rules the player must choose a target when the trigger goes on the stack (assuming legal targets exist), and separately decides yes/no on damage at resolution.

- **ETB trigger silently suppressed if source leaves battlefield before resolution** (`mtg-engine/src/triggers.rs` lines 893–899)
  - Oracle text says: `When this creature enters, you may have it deal 4 damage to target creature with flying.`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — if Geistcatcher's Rig leaves the battlefield after the trigger is on the stack but before resolution (e.g., killed in response), the trigger body is skipped entirely and no choice is presented. Per MTG rules, ETB triggers do not require the source to remain on the battlefield to resolve; the trigger was already captured and goes to resolution regardless.

### Tricky interactions checked

- **Target chosen at stack time vs. resolution time**: FAIL — ruling explicitly states target is chosen when trigger goes on the stack; code defers it to `on_enter_battlefield` at resolution.
- **"you may" decision point**: FAIL — ruling says the yes/no choice happens at resolution after target is locked in; code merges target selection and yes/no into a single `optional: true` ChooseTarget at resolution.
- **No valid flying targets present**: PASS functionally — `targets.is_empty()` causes the code to do nothing; per correct rules the trigger would go on the stack without a valid target and be countered at resolution; the outcome (no damage) is identical.
- **Source leaves battlefield between trigger placement and resolution**: FAIL — `resolve_next_trigger` guards on `o.zone == Zone::Battlefield`, suppressing the trigger; correct rules require the trigger to resolve regardless.
- **Can target opponent's flying creatures**: PASS — the filter does not restrict by controller; any creature with flying on the battlefield (other than the Rig itself) is a legal target.
- **NonCombatDamageDealt vs CombatDamageDealt event**: PASS — `PendingEffect::DealDamage` emits `GameEvent::NonCombatDamageDealt` in `apply_pending_effect`, which is correct for a triggered ability.
- **Mana cost {6}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(6)])` matches oracle.
- **P/T 4/5**: PASS — `power: Some(4), toughness: Some(5)` matches oracle.
- **Card types Artifact Creature — Construct**: PASS — `card_types: vec![CardType::Artifact, CardType::Creature], subtypes: vec!["Construct".into()]` matches oracle.
- **No keywords**: PASS — `keywords: vec![]` matches oracle (no keyword abilities listed).
- **has_keyword(Flying) checks both object keywords and registry**: PASS — `state.has_keyword` checks `obj.keywords`, card data, continuous effects, and until-EOT grants; tokens with Flying stored on the object are covered.
- **Damage amount is 4**: PASS — `PendingEffect::DealDamage { amount: 4, ... }` matches oracle.

### Test coverage

- **Target chosen at stack-placement time (ruling)**: NOT TESTED
- **"you may" declines damage at resolution**: NOT TESTED
- **No flying targets present — trigger does nothing**: NOT TESTED
- **Source leaves battlefield before trigger resolves**: NOT TESTED
- **Can target opponent's flying creature**: NOT TESTED
- **Correctly deals exactly 4 non-combat damage**: NOT TESTED
- **Trigger goes on stack (TriggeredAbilityDef present)**: NOT TESTED

There are zero tests for Geistcatcher's Rig in `mtg-engine/tests/`.
