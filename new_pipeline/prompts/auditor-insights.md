# Auditor insights — patterns discovered by previous audits

Read this file before starting your audit. It contains generalizable
patterns previous auditors found while working through the codebase.
Each entry describes a *code or engine pattern* that has caused bugs
in multiple cards, not a specific card-level finding.

When you finish your audit, if you discover a *new* generalizable
pattern (not just a card-specific bug), report it in your JSON output's
`insights` array. The pipeline will append it to this file for the
next auditor. **Do not add card-specific findings here** — those go in
your normal `findings` array.

<!-- New insights are appended below this line -->

## Inline damage bypasses engine protections

Cards that apply damage by directly writing `obj.damage_marked += N`
instead of using the central `PendingEffect::DealDamage` path bypass
the damage-processing results enumerated in CR 120.3 and the
replacement-effect layer (CR 614): protection (702.16),
hexproof/ward (702.11/702.21), lifelink (702.15), planeswalker
loyalty removal (120.3c / 306.8), battle defense removal (120.3d),
damage prevention, and any "whenever damage is dealt" triggers.
Always check whether a card's damage code goes through the central
handler in `resolve_single_effect()` (engine.rs).

## Zone-change cleanup does not reset characteristic modifications

The cleanup block in `move_object()` (state.rs) does NOT clear
`subtypes`, `keywords`, `colors`, `power`, `toughness`, `card_types`,
or `name` when an object leaves the battlefield. Any card that
modifies these fields at runtime (e.g., adding a subtype, granting
a keyword) will have those modifications persist incorrectly through
zone changes, violating CR 400.7. Check whether the card modifies
any of these fields and whether that modification would survive a
zone change.

## "For as long as you control [source]" requires continuous re-evaluation

Per CR 611.2b, a "for as long as" duration ends as soon as its
condition becomes false — not just when the source leaves the
battlefield. Effects with "for as long as you control [source]"
must end when the source's controller changes even if the source
stays on the battlefield (e.g., Act of Treason takes temporary
control without a zone change). An implementation that only uses
`on_leave_battlefield` will miss this case. Check whether the card
has any mechanism to detect controller changes on the source
permanent.

## "Each player" choice effects require simultaneous resolution

Per CR 101.4 (APNAP), when multiple players must make choices or
take actions at the same time, the active player chooses first,
then each other player in turn order — but then the actions happen
SIMULTANEOUSLY. A card like "each player discards a card" resolves
with all choices locked in before any discards occur, so no
player's choice can affect another's chosen state. The engine's
chained-choice pattern (resolve one player's choice, then set up
the next player's) forces sequential execution, letting earlier
players' resolved choices trigger abilities or alter game state
before later players choose. Any card with "each player
discards/sacrifices/chooses" should be checked for whether
simultaneity matters per its rulings.

## DFC face-dependent `is_valid_target` breaks with multiple copies

The `CardBehavior::is_valid_target` trait method receives `caster:
PlayerId` but not the activating object's `ObjectId`. Double-faced
cards that need different targeting rules per face (e.g., front
face targets "creature with flying", back face targets "any
creature") must work around this by searching for a matching
object via `state.objects.values().find(...)`. This search is
non-deterministic (HashMap iteration order) and returns an
arbitrary copy when the player controls multiple instances in
different transform states. Check any DFC with face-dependent
targeting logic in `is_valid_target` for this pattern.

## DFC transform and zone-change cleanup: `obj.name` has no registry fallback

When a DFC transforms, `obj.name` is updated to the back-face name.
When the permanent later leaves the battlefield, `move_object`
clears `is_transformed` but not `name`. The engine has
registry-based fallbacks for keywords and subtypes (checking
`is_transformed` + `back_face_data()`), which mask stale
object-level values for those fields. However, `obj_name()` reads
`obj.name` directly with NO registry fallback. This makes `name`
the most impactful stale field for DFCs after zone changes — the
card retains its back-face name in non-battlefield zones,
violating CR 712.8a. Also check whether a DFC card uses
`helpers::apply_transform` (the correct path) vs manual transform.

## Counter-removal activation costs are not supported by the engine

`ActivatedAbilityDef` only supports mana costs, tap requirements,
and sacrifice costs. It has no field for "remove N counters" as an
activation cost. Cards that require counter removal as a cost (text
like "Remove three study counters from ~") must handle it manually
in `on_activate_ability`. Per CR 601.2h / 602.2b, costs are paid in
any order within the non-random group, but each cost must actually
be paid (118.11). When such a card also sacrifices itself as part
of the same cost, the counter removal is sometimes skipped: the
sacrifice moves the object to the graveyard, which clears all
counters via `move_object` in one step, so the "remove N counters"
action never actually runs. Check any card with "Remove N counters"
in its activation cost to verify the counters are removed first
and that the count is correct at the time of any simultaneous
sacrifice or zone change.

## Controller update after move_object causes stale EnteredBattlefield events

Cards that move objects to the battlefield "under your control"
(reanimation, steal effects) often call
`state.move_object(id, Zone::Battlefield, registry)` first, then
set `obj.controller = controller` afterward. The `move_object`
function emits an `EnteredBattlefield` event using the object's
controller at move time — which is the previous controller
(typically the owner), not the intended new controller. While the
trigger dispatch system currently re-reads controller from state,
the event itself stores the wrong value. This also affects the
`EnterWatch` trigger's `entered_controller` field. Check any card
that changes controller after `move_object` — the controller
should be set BEFORE the move, or `move_object` should accept a
controller parameter.

## Triggered ability resolution skips target legality check (CR 608.2b)

The `resolve_next_trigger` function in `triggers.rs` dispatches
each trigger variant directly to its handler without re-checking
target legality. Per CR 608.2b, a triggered ability with targets
must verify that all targets are still legal when it tries to
resolve; if all targets are illegal, the ability is removed from
the stack. The target validity infrastructure exists for spells
(`resolve_spell` in `stack.rs` calls `is_target_legal` +
`is_valid_target`), but the parallel check is missing from the
trigger resolution path. Any targeted triggered ability — ETB,
death, spell-cast, upkeep — will resolve even when its targets
have become illegal (moved zones, gained hexproof, etc.).

## Activated ability targeting omits protection-from-source check

`generate_ability_targets` (engine.rs) receives `source_id` but
passes `None` to `can_be_targeted_by` via the wrapper
`can_be_targeted`. This means protection from the ability's source
permanent is never checked when enumerating valid targets for
activated abilities. The spell-targeting path
(`valid_targets_for_req`) correctly passes `Some(spell_id)`. Any
card with a targeted activated ability — fight abilities,
tap-to-damage, tap-to-exile — can illegally target creatures with
protection from the source's color, type, or other quality. Check:
verify whether the activated ability target enumeration threads
`source_id` through to the protection check.

## Non-creature death-watchers missed in simultaneous destruction

The `simultaneously_dead` list in `triggers.rs` only tracks
`CreatureDied` events. Non-creature permanents — enchantments,
artifacts, planeswalkers — that have `AnyCreatureDies` triggered
abilities are excluded when destroyed simultaneously with
creatures. Their death-watch triggers are silently dropped. Any
card that is NOT a creature but watches for creature deaths
(e.g., enchantments like Gutter Grime) should be checked for
correct trigger creation when the watcher is destroyed in the
same batch as the watched creatures.

## Token copy of a non-registry token loses characteristics

`create_token_copy` reads keywords, subtypes, card_types, and
colors from `registry.card_data(card_id)`. Tokens created by
`create_token_with_subtypes` (generic tokens like "2/2 black
Zombie") have `card_id = CardId(0)`, a sentinel not in the
registry. The lookup returns `None`, and `.unwrap_or_default()`
silently drops all four fields to empty vectors. Only name and
P/T (read from the object) survive. Any card that creates token
copies should be checked for whether the source could be a
generic token — if so, the copy loses creature types, subtypes,
keywords, and colors.

## Controller field not reset to owner on zone change affects CDAs

`move_object` resets battlefield-specific state (tapped,
summoning_sick, damage_marked, counters) when a permanent leaves
the battlefield, but does not reset `controller` to `owner`. Per
CR 112.8, a card not on the stack or battlefield is controlled by
its owner. Any card that reads `obj.controller` off-battlefield —
particularly CDA creatures using `dynamic_pt` — will use the last
battlefield controller instead of the owner after a
control-change-then-zone-change sequence. The "Zone-change cleanup
does not reset characteristic modifications" insight covers fields
like subtypes and keywords; `controller` is a distinct field
requiring a different fix (set to `owner`, not clear to default).

## Equipment/aura triggers derive equipped creature from current state

Equipment and aura cards with attack/block triggered abilities
(e.g., "Whenever equipped creature attacks") typically derive the
equipped creature's identity by reading `equip.attached_to` at
trigger resolution time. If the equipment becomes detached between
trigger creation and resolution — because the creature was
destroyed (SBA detaches equipment), or a re-equip effect moved the
equipment — the handler reads stale or wrong state. Per CR 603.3c,
triggered abilities should use information from the triggering
event. The root cause is that `PendingTrigger::AttacksTrigger`
stores only `object_id` (the equipment) but not the attacking
creature's ID or the defending player. Any equipment or aura with
attack/block triggers should be checked for whether the handler
derives the creature from current `attached_to` rather than a
trigger-time capture.

## "If you do" draw conditionals cannot verify draw success

`draw_cards` (engine.rs) returns `void` — it tracks `drawn` count
internally but does not expose it to the caller. Any card with
oracle text "draw a card. If you do, [X]" needs to verify that a
card was actually drawn before performing the conditional action.
Without a return value, callers cannot distinguish between "drew
successfully" and "library was empty, draw failed." Cards using
this pattern that unconditionally proceed to the conditional
action after `draw_cards` will incorrectly perform the action
even when the draw failed (e.g., empty library).

## Non-token permanents have empty `card_types` on the object; use `o.power.is_some()` for creature detection

The `create_object` function initialises `card_types: Vec::new()`. The default `on_resolve` for permanents does not populate `card_types` on the object — it only calls `state.move_object`. Tokens are the exception: `create_token_with_subtypes` explicitly sets the `card_types` argument. As a result, `o.card_types.contains(&CardType::Creature)` silently returns `false` for most non-token creatures on the battlefield. The canonical engine pattern for 'is this a creature on the battlefield?' is `o.power.is_some()` (e.g., `sba.rs:54`). Card code that builds a list of battlefield creatures for buffing, sacrificing, or similar effects must use `o.power.is_some()` (or the two-branch registry fallback `if o.card_types.is_empty() { registry.card_data(...).is_some_and(...) } else { o.card_types.contains(...) }`) rather than relying on the object's `card_types` field alone.

_Discovered auditing: Garruk Relentless_

## Per-turn state flags in card_state are not cleared at end of turn unless the clearing code runs unconditionally

Cards that store "happened this turn" boolean flags in obj.card_state (e.g., `card_state.insert("attacked_this_turn", ...)`) must ensure those flags are cleared at the end of every turn, regardless of game state. The Cleanup step handler in engine.rs does NOT clear card_state; it only resets damage_marked, until_end_of_turn effects, and regeneration shields. If the flag-clearing logic lives inside a triggered ability handler that only fires under specific conditions (such as "only when the creature is on the back face at end step"), the flag can persist across turns when those conditions are not met. Any card that writes a per-turn tracking flag to card_state should clear that flag in a step/phase that is guaranteed to run every turn — for example, during the Cleanup step or the Untap step — or should use a global event hook rather than a conditional trigger handler.

_Discovered auditing: Civilized Scholar_

## Step-trigger dispatch never checks intervening-if conditions at trigger-creation time

The `GameEvent::StepStarted` handler in `triggers.rs` (around line 844) creates a `PendingTrigger` for every battlefield permanent whose current face has a non-empty description for the relevant `TriggerKind`. The only guard is the `step_trigger_scope` check (Your vs Each). Per CR 603.4, a triggered ability phrased as 'At/When/Whenever [event], if [condition], [effect]' must evaluate the condition *when the trigger event occurs* — the trigger only goes on the stack if the condition is true at that moment. The engine skips this check at creation and only evaluates the condition at resolution (inside `on_upkeep`, `on_end_step`, etc.). This causes triggers to appear on the stack when they shouldn't, incorrectly granting priority and exposing observable game state. All cards with conditional step triggers are affected: werewolves ('if no spells were cast last turn', 'if a player cast 2+ spells'), Screeching Bat ('if you have no cards in hand'), and any future card with an 'At the beginning of [your/each] upkeep, if [condition]' pattern. The fix requires a new `CardBehavior` trait method — e.g. `should_step_trigger(state, id, kind, registry) -> bool` defaulting to `true` — that the dispatch loop calls before creating the trigger.

_Discovered auditing: Village Ironsmith_

## Intervening-if upkeep triggers queued unconditionally

The `collect_triggers` handler for `GameEvent::StepStarted { step: Upkeep }` (triggers.rs) calls `face_trigger_description` to decide whether a card has an upkeep trigger, then unconditionally queues the trigger if the description is non-empty. `face_trigger_description` only checks whether the card declares an upkeep trigger — it does not evaluate the trigger's intervening-if condition. Per CR 603.4, a triggered ability reading 'At [event], if [condition], [effect]' may only be placed on the stack if the condition is true at the time the event occurs. Cards whose upkeep trigger description is a static string (e.g. werewolf transform triggers) will therefore appear on the stack on every upkeep, even when the condition is false, giving players a spurious opportunity to respond. The fix requires a new `CardBehavior` hook (e.g. `should_queue_upkeep_trigger(state, is_transformed) -> bool`) that the dispatch calls before queuing, so cards with intervening-if conditions can pre-filter. Any card whose upkeep trigger has an 'if [dynamic condition]' clause should be checked for this pattern.

_Discovered auditing: Kruin Outlaw_

## In-card subtype-counting loops may not handle DFC transformation

Some cards implement local helper functions that count permanents of a given type (e.g., `count_vampires`) rather than routing through the engine's canonical `matches_filter` / `HasSubtype` path. These local loops commonly check `registry.card_data(o.card_id).subtypes` to identify non-token card types, but `registry.card_data()` always returns the **front-face** data regardless of `o.is_transformed`. The engine's canonical `matches_filter` (state.rs) correctly handles this by branching on `creature.is_transformed` and consulting `back_face_data().subtypes` when true. Any card with a custom type-counting activation condition should be checked to see whether its loop mirrors that branch. If a DFC exists whose front face has the relevant subtype but back face does not (or vice versa), the in-card loop will produce wrong counts, incorrectly enabling or suppressing the activation restriction.

_Discovered auditing: Bloodline Keeper_

## `ChooseFromLibrary` omits `move_spell_after_resolve` and cannot support post-search card logic

`ResolutionChoiceKind::ChooseFromLibrary` was designed for *activated abilities* (Garruk, Traveler's Amulet) where there is no spell on the stack to clean up after the search. Its handler in engine.rs always moves the chosen card to `Zone::Hand` and shuffles, but never calls `move_spell_after_resolve`. When a *resolving sorcery* uses this mechanism for the multi-match case, the spell remains in `Zone::Stack` after the choice resolves and will be re-entered on the next priority pass. Additionally, the handler offers no hook for card-specific post-search logic (e.g., a conditional destination choice like Caravan Vigil's morbid option). Any sorcery or instant that needs to (a) let the player choose among multiple library matches and (b) do anything beyond moving the chosen card to hand must NOT use `ChooseFromLibrary`; it needs a dedicated card-specific selection path that routes through the card's own resolution logic after the player picks.

_Discovered auditing: Caravan Vigil_

## Counters placed on non-battlefield objects persist through reanimation

When a triggered ability places a counter on its source (e.g., 'put a +1/+1 counter on [card name]') and the source has left the battlefield before the trigger resolves, `add_counters` in state.rs adds the counter to the graveyard (or exile) object without checking zone. `move_object` clears counters only when an object *leaves* the battlefield (`from == Zone::Battlefield && to != Zone::Battlefield`), not when it *enters* the battlefield from a non-battlefield zone. Therefore a counter added to a graveyard object by a stale trigger survives the graveyard → battlefield transition and incorrectly appears on the reanimated permanent, violating CR 400.7. Any effect helper or triggered-ability handler that calls `add_counters(source_id, ...)` should first verify that `source_id` refers to an object whose `zone == Zone::Battlefield`.

_Discovered auditing: Grimgrin, Corpse-Born_

## Player-choice mana abilities implemented as `ActivatedAbilityDef` are excluded from auto-tap plans

`ManaAbilityDef` has a static `produced: Vec<(ManaType, u32)>` field, which cannot express "add one mana of any color" without enumerating one entry per color. When a card has a mana ability requiring a player choice ("add one mana of any color," "add one mana of a color a land you control could produce," etc.), developers may implement it as multiple `ActivatedAbilityDef` entries instead of `ManaAbilityDef` entries. This workaround silently excludes the ability from `gather_mana_sources` (engine.rs, line 76), which only calls `behavior.mana_abilities()`. The ability therefore never appears in any `CastSpell` or `ActivateAbility` tap plan. AI players lose all awareness of the color-fixing the land provides, and the tap-plan optimizer cannot fund colored spells through it. The fix is to add one `ManaAbilityDef` per color choice so the optimizer can select the correct color during planning. Any land or artifact with text like "Add one mana of any color" or "Add one mana of any type that a land you control could produce" should be checked for this pattern.

_Discovered auditing: Shimmering Grotto_
