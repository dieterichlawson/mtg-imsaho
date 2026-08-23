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

## Spell cleanup must be deferred to PendingEffect handler when awaiting_action is set

Cards that present player choices via `awaiting_action` during `on_resolve` must NOT call `move_spell_after_resolve` in `on_resolve` before setting `awaiting_action`. The comment in `stack.rs` at the `resolve_top` function (around line 166) makes this explicit: 'If the card set an awaiting_action, it\'s mid-resolution. Don\'t clean up yet — the ResolveChoice handler in submit_action will do that.' The correct pattern: (1) do NOT call `move_spell_after_resolve` in `on_resolve`; (2) pass `spell_id: ObjectId` inside the `PendingEffect` enum variant; (3) call `state.move_spell_after_resolve(spell_id, registry)` at the END of the effect handler in `engine.rs`, after all per-effect actions are complete. `tribute_to_hunger.rs` + `SacrificeAndGainLife` (engine.rs:3662) is the canonical reference. Cards that violate this pattern move the spell to the graveyard (or exile it, for flashback) while player choices are still pending, reversing the CR 608.2 resolution order. Per the rules-strict audit standard, this is a bug even when no trigger fires on the premature zone change in the current engine.

_Discovered auditing: Divine Reckoning_

## Transient-guard flags set during ETB must be cleared after the guarded effect resolves

Some cards set a boolean flag on the entering permanent during `on_enter_battlefield` to suppress state-based actions while a pending choice is resolved (e.g., `entering_copy_source = true` protects Evil Twin's 0/0 from immediate SBA death while the copy-choice prompt is outstanding). The engine has two guards in the SBA toughness/damage loop: a behavior-level check (`enters_as_copy()`) that uses the object's `card_id` to look up the trait, and an object-level flag check (`entering_copy_source`). If the copy choice succeeds and the `card_id` changes to the copied creature, the behavior-level guard becomes false — but the object-level flag is never cleared in the `CopyCreature` handler, leaving it permanently `true`. Any card that sets a transient boolean flag in `on_enter_battlefield` to gate SBA must explicitly clear that flag in the corresponding pending-effect handler (or wherever the guarded state is resolved). Otherwise, the guard persists indefinitely and suppresses SBA checks for the permanent's remaining lifetime.

_Discovered auditing: Evil Twin_

## SpellCast trigger dispatch fires unconditionally for all spell casts

The `GameEvent::SpellCast` handler in `collect_triggers` (triggers.rs lines ~923-950) creates a `SpellCastWatch` trigger for EVERY battlefield permanent that declares a `TriggerKind::SpellCast` ability, regardless of who cast the spell or what type the spell was. The comment 'Individual card handlers can filter by spell type if needed' reflects a deliberate design choice, but it is rules-incorrect: per CR 603.2, a 'Whenever [player] casts [type]' ability should only go on the stack when the full trigger condition is satisfied. Instead, the trigger is pushed onto the stack for all spell casts and the condition is only evaluated at resolution inside `on_spell_cast`. Players therefore see spurious stack entries — for example, Charmbreaker Devils' 'Whenever you cast an instant or sorcery spell' trigger appears when the opponent casts a creature. Any card with a conditional SpellCast trigger (restricting by caster identity or by spell type) is affected. Compare with the `CreatureCardMilled` dispatch in the same function, which explicitly filters watchers by `watcher_controller != milled_player` before creating the trigger, showing that event-time filtering is possible and is the right pattern.

_Discovered auditing: Charmbreaker Devils_

## `AnyTarget` permanent filter lacks registry fallback for planeswalker type

The `AnyTarget` branch in `generate_ability_targets` (engine.rs:2055) filters battlefield permanents by `o.power.is_some() || o.card_types.contains(&CardType::Planeswalker)`. Since non-token permanents have empty `card_types` on the battlefield object (per the 'Non-token permanents' insight), the planeswalker check silently returns false for all non-token planeswalkers. The `PlayerOrPlaneswalker` branch (engine.rs:2043–2046) was already patched with a registry fallback (`registry.card_data(obj.card_id).is_some_and(|d| d.card_types.contains(&CardType::Planeswalker))`), but `AnyTarget` was not. When auditing any card with `TargetRequirement::AnyTarget`, verify the engine's AnyTarget case has the same registry fallback; without it, the card cannot target non-token planeswalkers despite oracle text saying 'any target.'

_Discovered auditing: Heretic's Punishment_

## `combat::fight()` partially handles damage modifiers but still inlines `damage_marked`

The `deal_fight_damage` helper in `combat.rs` manually checks protection-from-source (via `has_protection_from_creature`) and handles lifelink and deathtouch, giving the impression that fight damage is fully rules-compliant. However, it still writes `obj.damage_marked += amount` directly instead of routing through `apply_pending_effect(PendingEffect::DealDamage)`. The central handler contains at least one damage replacement check that `deal_fight_damage` omits: `PreventDamageRemoveCounter` (the effect Unbreathing Horde uses to prevent damage by removing a +1/+1 counter, CR 614.1a). An auditor checking a fight card under rule 8e should not stop at 'fight uses a shared utility' — trace into `deal_fight_damage` itself and compare its checks against the full central handler. Currently only Nightfall Predator and Prey Upon use this path.

_Discovered auditing: Daybreak Ranger_

## Equipment/Aura `dynamic_pt` bleeds into `effective_power` self-check

The `effective_power` and `effective_toughness` functions in state.rs (lines 1090–1096 and 1151–1158) call `behavior.dynamic_pt(self, id)` on every battlefield permanent as the first step — the comment labels this the 'self-check for CDAs (e.g., Geist-Honored Monk)'. The branch fires whenever `registry.get(obj.card_id)` succeeds, which is true for every registered card including Equipment and Auras. Equipment and Auras that implement `dynamic_pt` to contribute variable P/T to the attached object (e.g., Runechanter's Pike, Wreath of Geists) return `Some(...)` even when queried with the source's own ObjectId, because their zone check (`zone != Battlefield -> None`) is satisfied while on the battlefield. As a result, `effective_power(source_id)` returns a non-None integer and `effective_toughness(source_id)` returns Some(0), causing those objects to appear to have P/T in `PermanentView.effective_power/effective_toughness`. The fix is to add `if obj.power.is_some()` before the `dynamic_pt` call in both self-check branches so that Equipment (power: None) and Auras (power: None) are skipped, while CDA creatures (which use `power: Some(0)` as a sentinel) continue to have their CDA evaluated. Check any card implementing `dynamic_pt` that is not a creature (no base P/T) for this pattern.

_Discovered auditing: Runechanter's Pike_

## Death-watch dispatch checks `card_data()` (front face only) without consulting `is_transformed`

The death-watch trigger dispatch in `triggers.rs` (lines 662-664) calls `b.card_data().triggered_abilities` to determine whether a watcher permanent has an `AnyCreatureDies` trigger. `card_data()` always returns front-face data, regardless of whether the object is currently transformed. Any DFC whose front face has `AnyCreatureDies` but whose back face does not will create a spurious `DeathWatch` trigger on the stack whenever a creature dies while the DFC is showing its back face. Per CR 712.8d, a DFC's characteristics are solely those of its active face. The fix: extend the watchers collection at lines 654-658 to also capture `o.is_transformed`, then branch at line 662-664 to check `b.back_face_data().triggered_abilities` when `is_transformed` is true. The symmetric bug (back face has `AnyCreatureDies`, front face does not) would suppress triggers that should fire — though no such card exists in the current set.

_Discovered auditing: Thraben Sentry_

## Equipment activated-ability enumeration does not check equipment-to-creature controller parity

The legal-action loop in engine.rs (lines 682-688) iterates over ALL attached permanents and adds their activated abilities to the object's ability list without verifying that the attached permanent's controller matches the object's controller. Per CR 602.2a and 701.13, a player can only activate an ability if they can pay all its costs — and a player cannot sacrifice a permanent they don't control. If any effect causes equipment E (controlled by player A) to attach to creature C (controlled by player B), player B's legal actions incorrectly include E's activated abilities, and any sacrifice cost in on_activate_ability runs without an ownership check. The fix is to add an `attached.controller == player` guard in the enumeration loop (and a matching guard before any sacrifice in on_activate_ability). Any equipment whose damage/sacrifice ability is granted to the equipped creature is affected.

_Discovered auditing: Blazing Torch_

## `matches_target_filter` `HasCardType` branch lacks registry fallback, breaking non-creature permanent targeting

`matches_target_filter` in engine.rs handles `TargetFilter::HasCardType` with a bare `obj.card_types.contains(t)` check and no registry fallback. Because `create_object` initialises `card_types: Vec::new()` for every game object, non-token permanents on the battlefield always have an empty `card_types` list, so `HasCardType([Artifact])`, `HasCardType([Enchantment])`, and `HasCardType([Land])` all silently return `false` for non-token permanents of those types. Tokens are the exception: `create_token_with_subtypes` explicitly sets `card_types`. The `HasSubtype` branch in the same function already applies the correct two-step pattern (`obj.subtypes` first, then `registry.card_data(obj.card_id)`). The fix for `HasCardType` is identical: fall back to `registry.card_data(obj.card_id).is_some_and(|d| types.iter().any(|t| d.card_types.contains(t)))` when the object-level field is empty. Any card whose targeting requirement uses `PermanentWithFilter(HasCardType([Artifact|Enchantment|Land|...]))` — whether cast as a spell or activated as an ability — is affected and will offer zero valid targets against all non-token permanents of those types.

_Discovered auditing: Ghost Quarter_

## Blocks/BecomesBlocked trigger dispatch fires unconditionally regardless of creature-type conditions

The `BlockersDeclared` handler in `collect_triggers` (triggers.rs) creates `BlocksTrigger` and `BecomesBlockedTrigger` entries for every piece of equipment that declares those trigger kinds — it checks only whether the card has a non-empty description for `TriggerKind::Blocks` / `TriggerKind::BecomesBlocked`, with no facility for the card to pre-filter based on the type of the other creature. Cards whose oracle text says 'whenever equipped creature blocks **a [Type]**' or 'becomes blocked by **a [Type]**' have that type condition as part of the trigger condition itself (not an intervening-if), meaning per CR 603.2 the trigger should only go on the stack when the type condition is satisfied. Instead, the trigger fires for every block and the type check happens at resolution in `on_blocks` / `on_becomes_blocked`, creating spurious stack entries that give players incorrect priority windows. Compare with the analogous SpellCast dispatch issue (documented separately). Any future card with a conditional Blocks or BecomesBlocked trigger that restricts on the other creature's type should be checked for this pattern; a card-behavior hook analogous to `should_queue_upkeep_trigger` would be the correct fix.

_Discovered auditing: Wooden Stake_

## ETB trigger dispatch always defers target selection to resolution

The `collect_triggers` handler for `GameEvent::EnteredBattlefield` (triggers.rs ~line 573) hardcodes `chosen_targets: Vec::new()` when creating every `PendingTrigger::EnteredBattlefield`. The `target_requirement` field declared in `TriggeredAbilityDef` is not consulted during ETB dispatch. As a result, ALL cards with ETB targeted abilities defer target selection to `on_enter_battlefield` at resolution time, ignoring the `chosen_targets` parameter (typically prefixed `_chosen_targets`). Per CR 603.3b, targets for triggered abilities must be chosen when the trigger is put on the stack. The current design grants the controlling player an information advantage (they choose targets after opponents have responded) and allows targeting creatures that entered between trigger creation and resolution. Any card whose ETB ability has a target (e.g., 'When ~ enters, exile target creature') should be checked: if `on_enter_battlefield` ignores `_chosen_targets` and gathers fresh targets via helpers, the card has this defect.

_Discovered auditing: Fiend Hunter_

## `PermanentWithFilter` targeting has two code paths with inconsistent pre-filtering

The `PermanentWithFilter` branch appears twice in the targeting logic with different filter pipelines. The action-generation path inside `legal_actions` (engine.rs:1655-1669) deliberately skips `matches_target_filter` and delegates entirely to `is_valid_target` (comment: 'Actual filtering is done by the card\'s is_valid_target'). The `build_cast_target_spec` → `valid_targets_for_req` path (engine.rs:1777-1784, called at 1931) applies `matches_target_filter` as a pre-filter before `is_valid_target`. This means a card whose `is_valid_target` is correct but whose `target_requirement` uses a filter type that `matches_target_filter` handles incorrectly (e.g., `HasCardType`) will produce consistent results for human/random players but broken results for AI/LLM players. When auditing any card with `PermanentWithFilter`, check both code paths: if the card has a custom `is_valid_target`, confirm that the pre-filter in `valid_targets_for_req` does not over-restrict candidates before `is_valid_target` can correct them.

_Discovered auditing: Maw of the Mire_

## Equipment `generate_ability_targets` filters out currently-attached creature, blocking re-equip to same host

In `generate_ability_targets` (engine.rs), the `TargetRequirement::CreatureWithFilter` branch computes `already_attached = source.attached_to` for Equipment sources and then excludes that creature from the target list via `.filter(|o| already_attached != Some(o.id))`. Per CR 702.6a, equip abilities say "Attach this permanent to target creature you control" with no restriction against targeting the currently-attached creature. The filter was added with the comment "For equipment equip abilities, exclude the creature already attached to this equipment" — a defensible UX simplification (re-equipping to the same creature is usually pointless) but rules-incorrect. It blocks the strategically valid action of sacrificing a different creature as the equip cost while preserving the current attachment (useful when the sacrifice triggers other abilities). Any Equipment card using `TargetRequirement::CreatureWithFilter` as its equip target requirement is affected. Fix: remove the `already_attached` filter from `generate_ability_targets`; the downstream (sacrifice != target) guard at line 823–826 already prevents the degenerate case where the sacrifice and equip target are the same creature.

_Discovered auditing: Demonmail Hauberk_

## `PermanentWithFilter` inside `TwoTargets` applies `matches_target_filter`; top-level `PermanentWithFilter` in a spell does not

In `generate_cast_actions_with_targets` (engine.rs), the top-level `PermanentWithFilter(_)` branch (around line 1655) skips `matches_target_filter` entirely and delegates all filtering to `is_valid_target` — with a comment 'Actual filtering is done by the card's is_valid_target.' However, when `PermanentWithFilter` appears as a sub-requirement inside `TwoTargets`, the `TwoTargets` branch (line 1687) calls `valid_targets_for_req`, which DOES apply `matches_target_filter` as a mandatory pre-filter before consulting `is_valid_target`. As a result, a spell with a top-level `PermanentWithFilter(HasCardType([Land]))` correctly finds non-token lands (because `is_valid_target` uses the registry), but the same requirement nested inside `TwoTargets` silently returns zero land targets (because `matches_target_filter`'s `HasCardType` branch has no registry fallback). The existing `HasCardType` insight notes that 'whether cast as a spell or activated as an ability' is affected — for spells, this is only true when `PermanentWithFilter` is nested inside `TwoTargets` or `UpToTargets`. Any multi-target spell where one slot is `PermanentWithFilter(HasCardType([Artifact|Enchantment|Land|...]))` should be checked for this pattern.

_Discovered auditing: Into the Maw of Hell_

## Upkeep-triggered targeted abilities require `target_requirement: Some(...)` — unlike ETB triggers, the dispatch correctly handles this

The `process_pending_trigger_pushes` function in triggers.rs reads each pending upkeep (and end-step) trigger's `target_requirement` field from `TriggeredAbilityDef` and, when it is `Some(req)`, calls `valid_targets_for_req` to enumerate legal targets, auto-picks a single target, or prompts the controller — all before pushing the trigger onto the stack (CR 603.3b). When `target_requirement` is `None`, the trigger is classified as untargeted and pushed immediately with no target selection. A card that needs to target a player in its upkeep trigger must declare `target_requirement: Some(TargetRequirement::PlayerOnly)` (or the appropriate requirement) rather than handling target selection manually inside `on_upkeep`. Manually re-selecting targets in the handler violates CR 603.3b (targets chosen at resolution, not stack-placement), bypasses CR 603.3c (trigger should be removed if no legal targets exist), and bypasses the CR 608.2b legality re-check the engine performs at resolution. This is distinct from the ETB trigger pattern, where the engine itself always hardcodes `chosen_targets: Vec::new()` regardless of the card's `target_requirement`; for upkeep and end-step triggers, the engine already does the right thing — the card just needs to declare the requirement correctly.

_Discovered auditing: Bloodgift Demon_

## TriggerScope cannot express enchanted-player's upkeep

The TriggerScope enum (cards/mod.rs) has only Your (fires when controller == active_player) and Each (fires every upkeep). Cards with oracle text 'at the beginning of enchanted player's upkeep' need a third case: fire when the permanent's attached_to_player == Some(active_player). Because no such variant exists, these cards are forced to use TriggerScope::Each combined with a runtime guard in on_upkeep (e.g. 'if state.active_player != cursed_player { return; }'). The guard correctly suppresses the effect but does NOT suppress the trigger from going on the stack, producing a spurious stack entry and an incorrect priority window during every non-enchanted player's upkeep. Any enchantment Aura Curse whose trigger is worded 'at the beginning of enchanted player's upkeep' is affected. The correct fix is a new TriggerScope::AttachedPlayer variant whose dispatch guard checks o.attached_to_player == Some(active_player) before creating the PendingTrigger, or an equivalent should_queue_step_trigger hook on the CardBehavior trait.

_Discovered auditing: Curse of Oblivion_

## `attached_to_player` is not cleared in `move_object` leave-battlefield cleanup

`GameObject` has two attachment fields: `attached_to: Option<ObjectId>` for equipment/creature-targeted Auras, and `attached_to_player: Option<PlayerId>` for Curse Auras. The `move_object` cleanup block (state.rs:599–621) correctly clears `obj.attached_to = None` when a permanent leaves the battlefield, but has no corresponding `obj.attached_to_player = None`. Curses attached to players via `AttachCurseToPlayer` (engine.rs) or `resolve_curse` (helpers.rs) will therefore carry a stale `attached_to_player` value in all non-battlefield zones, violating CR 400.7. Current code is incidentally protected because every consumer of `attached_to_player` guards with `o.zone == Zone::Battlefield`, so no observable bug exists today. The fix is to add `obj.attached_to_player = None` to the leave-battlefield cleanup block, symmetric with `obj.attached_to = None`. Any future card or effect that moves an enchantment to the battlefield without explicitly setting `attached_to_player` (e.g., an enchantment-reanimation effect) will trigger this latent bug.

_Discovered auditing: Bitterheart Witch_

## `AttachedHasSubtype` / `AttachedLacksSubtype` conditions treat `obj.subtypes` as authoritative when non-empty, missing registry native types

The `check_condition` branch for `EffectCondition::AttachedHasSubtype` (state.rs ~1492) uses a two-branch check: if `target_obj.subtypes.is_empty()`, fall back to the registry; otherwise, only check `target_obj.subtypes`. This diverges from the canonical `matches_filter::HasSubtype` pattern (state.rs:881), which always checks the registry first and treats `obj.subtypes` as an additive fallback. For non-token, non-DFC cards, `obj.subtypes` starts empty and only receives runtime pushes: Olivia Voldaren pushes "Vampire", Grimoire of the Dead pushes "Zombie". After either push, `obj.subtypes` is non-empty but does not contain the creature's native types from the registry. Cards whose continuous effects condition on `AttachedHasSubtype("Human")` — Butcher's Cleaver, Bonds of Faith, Sharpened Pitchfork, Silver Inlaid Dagger — will therefore incorrectly drop their Human-conditional effects when the equipped/enchanted creature has had a subtype added at runtime. `AttachedLacksSubtype` delegates to `AttachedHasSubtype` and inherits the inverted form of the same bug. The fix is to apply the same two-step check as `matches_filter::HasSubtype`: always consult the registry (using the appropriate face when `is_transformed` is true) and also check `obj.subtypes`, returning true if either confirms the subtype.

_Discovered auditing: Butcher's Cleaver_

## Curse-aura upkeep triggers have no engine scope for 'enchanted player's upkeep'

Curse auras trigger 'at the beginning of enchanted player's upkeep' — the relevant player is the ENCHANTED player (an opponent), not the Curse's controller. The engine's `TriggerScope` enum only has `Your` (controller's upkeep only) and `Each` (every upkeep). There is no scope that fires exclusively on a specific other player's upkeep. Curse auras therefore default to `TriggerScope::Each` and must place an in-handler early-return guard (`if state.active_player != cursed_player { return; }`) to suppress the damage effect. This guard is correct for the effect but too late for the trigger: the trigger is already on the stack before `on_upkeep` is called, giving players a spurious priority window on every non-enchanted-player upkeep. The fix requires a new card-behavior hook — e.g., `should_queue_upkeep_trigger(state, id, registry) -> bool` defaulting to `true` — that the dispatch loop calls before creating the `PendingTrigger::UpkeepTrigger`, so curse auras can check `state.active_player == state.get_object(id)?.attached_to_player`. Any aura that enchants a player and has an 'at the beginning of enchanted player's upkeep' trigger shares this defect.

_Discovered auditing: Curse of the Pierced Heart_

## `on_any_creature_dies` zone-check suppresses simultaneous-death triggers for creature deathwatchers

Several `on_any_creature_dies` handlers guard with `o.zone == Zone::Battlefield` before applying their effect (e.g., Selhoff Occultist, Lumberknot, Unruly Mob, Village Cannibals, Rage Thrower). The engine's `simultaneously_dead` logic in `collect_triggers` (triggers.rs:647–653) correctly includes creature watchers that died in the same event batch, so their DeathWatch triggers ARE created and placed on the stack. However, the card-level zone check then suppresses the effect at resolution because the watcher is now in the graveyard. The engine comment at line 1341 explicitly documents that 'death triggers fire even if the watcher died simultaneously.' For cards whose effect does not modify the watcher itself (e.g., mill a player, deal damage to a player), the zone check is simply wrong. For cards that add counters to themselves (Lumberknot, Unruly Mob, Village Cannibals), the zone check also prevents the trigger from firing, though placing a counter on a graveyard object would itself be wrong (per the 'Counters placed on non-battlefield objects persist' insight). In either case, the correct fix is to read the watcher's controller using last-known information from the object's current zone without asserting Zone::Battlefield, and to NOT guard the effect on the watcher's continued presence on the battlefield.

_Discovered auditing: Selhoff Occultist_

## Activated ability handlers that read source stats need an explicit zone == Battlefield guard

The engine (`stack.rs`) calls `behavior.resolve_activated_ability()` unconditionally when popping the ability from the stack — there is no engine-level check that the source is still on the battlefield. Cards whose ability resolution reads characteristics of the source object (toughness, power, controller, counters) and uses them to affect the game must include `if obj.zone != Zone::Battlefield { return; }` at the top of `resolve_activated_ability`. The `None => return` guard that many handlers have only protects against a completely absent object; it does not protect against an object that has moved to the graveyard, hand, or exile. This matters most for exchange, copy, or comparison effects where the source's current stats are the operands: if the source has left the battlefield, those stats may be stale or undefined. Check any activated ability handler whose resolution behavior depends on the source object's zone-sensitive characteristics.

_Discovered auditing: Tree of Redemption_

## Custom mill implementations must emit `GameEvent::CreatureCardMilled`

The centralized `mill_cards()` helper (engine.rs:4313–4325) emits `GameEvent::CreatureCardMilled` whenever a creature card is placed into a player's graveyard from their library. Cards that implement bespoke mill logic — because they mill from the bottom (Cellar Door), need the milled card's mana value (Mindshrieker), or process multiple cards with intervening calculations (Heretic's Punishment) — bypass `mill_cards()` and therefore never fire this event. Each such card must manually push `state.events.push(GameEvent::CreatureCardMilled { object: milled_id, milled_player: player_id })` after confirming the milled card is a creature, or `TriggerKind::CreatureCardMilled` watchers (currently Undead Alchemist) silently miss those mills. When auditing any card that removes an object from a player's `library_order` and moves it to `Zone::Graveyard` outside of `mill_cards()`, verify it emits this event for creature cards.

_Discovered auditing: Cellar Door_

## Inline milling in card handlers skips `GameEvent::CreatureCardMilled`

The `mill_cards()` helper in engine.rs (lines 4302-4331) checks whether each milled card is a creature (via the registry) and emits `GameEvent::CreatureCardMilled { object, milled_player }` before moving on. Card handlers that re-implement milling inline — `library_order.remove(0)` followed by `state.move_object(obj_id, Zone::Graveyard, registry)` — silently skip this event emission. Any card watching for `TriggerKind::CreatureCardMilled` (e.g. Undead Alchemist) will therefore fail to trigger when a creature is milled by one of these inline paths. When auditing any card whose oracle text says 'mills N cards' (or 'puts the top N cards of your/their library into the graveyard'), verify that the implementation calls `mill_cards()` rather than inlining the library manipulation.

_Discovered auditing: Mindshrieker_

## Mid-resolution `awaiting_action` separates ETB and death triggers into different `collect_triggers` rounds, fixing their stack ordering

When an activated ability (1) creates a token or otherwise fires an ETB event and (2) then requires a player choice via `present_target_choice` / `awaiting_action` before completing its effect (e.g., a mandatory sacrifice), the engine collects ETB-watch triggers in one round of `collect_triggers` and death-watch triggers (from the deferred sacrifice) in a later round. Both sets land in `pending_trigger_pushes` but in insertion order: ETB triggers first, death triggers second. When `process_pending_trigger_pushes` runs after `awaiting_action` is cleared, it pushes ETB triggers onto the stack first and death triggers second — death triggers end up on top and resolve first, ETB triggers resolve last. Per CR 603.3b, when multiple triggered abilities would go on the stack at the same time, the active player (and NAP) should order them; the engine denies this choice by fixing the order based on collection round. Note that the `pending_trigger_pushes` mechanism correctly prevents ETB triggers from appearing on the visible stack while the sacrifice prompt is outstanding — so intermediate state is not exposed — but the ordering between simultaneous ETB and death triggers is determined by the engine rather than the player. This affects any card that creates a token (or ETB-triggering permanent) as part of an ability effect and then requires a sacrifice choice with 2+ valid targets.

_Discovered auditing: Stitcher's Apprentice_

## Werewolf `should_transform` adds `!is_first_turn` guard not present in oracle text

Every Innistrad front-face werewolf's `should_transform` implementation (villagers_of_estwald.rs, daybreak_ranger.rs, reckless_waif.rs, and others) uses the pattern `total_spells_cast_last_turn == 0 && !state.is_first_turn`. The `!state.is_first_turn` portion does not appear in any of these cards' oracle text — the oracle text simply says 'if no spells were cast last turn.' At game start, `num_spells_cast_last_turn` is an empty HashMap, so `values().sum() == 0` is vacuously true, meeting the oracle condition. The `!state.is_first_turn` guard suppresses transformation on the first upkeep of the game despite the condition being satisfied. Any card whose oracle text checks a 'last turn' condition should be audited for whether an implicit `!is_first_turn` guard has been added beyond what the oracle text requires. Verify against the official ruling (the Innistrad FAQ addresses this case specifically).

_Discovered auditing: Villagers of Estwald_

## `on_dies` does not receive the pre-captured last-known-information controller

The `PendingTrigger::SelfDies` struct captures `controller: dead_controller` (the creature's controller at the moment of death, per CR 603.10c) when the trigger is created. However, the dispatch in `triggers.rs` (lines 1334–1337) passes only `dead_id` and `chosen_targets` to `behavior.on_dies(state, dead_id, &chosen_targets, registry)` — the captured `controller` is silently dropped via `..`. Cards whose death trigger logic depends on 'your' resources (e.g., 'your graveyard', 'you gain life') must re-derive the controller by calling `state.get_object(object_id).map(|o| o.controller)`. This re-derivation is incorrect in edge cases: if the dead creature has been returned to the battlefield (by another simultaneous-death trigger resolving first), `object_id` now refers to a battlefield permanent whose current controller may differ from the LKI controller. The structurally correct fix is to add a `controller: PlayerId` parameter to `on_dies`, threaded from the captured `dead_controller` in the `SelfDies` dispatch. Any card with a 'when this creature dies' trigger that references 'your [zone/life/resource]' is potentially affected.

_Discovered auditing: Moldgraf Monstrosity_

## ETB trigger dispatch does not check intervening-if conditions at trigger-creation time

The `GameEvent::EnteredBattlefield` handler in `collect_triggers` (triggers.rs lines 565-580) creates a `PendingTrigger::EnteredBattlefield` for every card that returns `true` from `has_etb_handler()`, with no facility to evaluate an intervening-if condition. Per CR 603.4, a triggered ability phrased as 'When [event], if [condition], [effect]' must evaluate the condition when the trigger event occurs — if the condition is false, the trigger must not go on the stack at all. Cards with morbid ETB abilities (Woodland Sleuth, Hollowhenge Scavenger, Morkrut Banshee) all guard with `if !state.creature_died_this_turn { return; }` inside `on_enter_battlefield`, satisfying the second check at resolution but not the first. When the morbid condition is false, the trigger still appears on the stack and players incorrectly receive priority. A `should_queue_etb_trigger(state, object_id, registry) -> bool` hook defaulting to `true` on the `CardBehavior` trait would allow cards to pre-filter at dispatch time, analogous to the `should_queue_upkeep_trigger` pattern recommended for upkeep triggers with intervening-if conditions. Any card whose ETB trigger is conditioned on a runtime game-state check (morbid, threshold, storm count, etc.) should be checked for this pattern.

_Discovered auditing: Woodland Sleuth_

## "Enters tapped unless" on check lands is a replacement effect, not a triggered ability

The Innistrad cycle of check lands (Woodland Cemetery, Clifftop Retreat, Hinterland Harbor, Isolated Chapel, Sulfur Falls) each have oracle text reading "This land enters tapped unless you control a [Type] or a [Type]". Per CR 614.1d, this is a static replacement effect that modifies the entering event before it occurs — no stack entry is created. The engine has no `ReplacementEffect::EntersTapped` variant, so all five cards work around this with a `TriggerKind::EntersBattlefield` triggered ability whose `on_enter_battlefield` handler taps the land at resolution. This creates a spurious stack entry (visible priority window with the land untapped) and moves the condition check to trigger-resolution time instead of entry time. The correct fix is a new `CardBehavior` hook — e.g., `entering_tapped(state, id, registry) -> bool` — called BEFORE `EnteredBattlefield` is emitted in `move_object`, analogous to how `entering_with_counters` handles CR 614.1c. Any future card with "enters tapped unless" or "enters tapped if" wording should be checked for this pattern.

_Discovered auditing: Woodland Cemetery_

## `TemporaryEffect::GrantKeyword { target }` and `ModifyPT { target }` lack a zone guard in `has_keyword` / `effective_power`

The `GrantKeywordAll` and `ModifyPTAll` variants in `until_end_of_turn` both check `obj.zone == Zone::Battlefield` before applying the effect. The single-target counterparts — `GrantKeyword { target, keyword }` (state.rs:1293) and `ModifyPT { target, power_mod, toughness_mod }` (state.rs:1112) — do not. Both simply match on the `ObjectId` regardless of which zone that object currently occupies. As a result, if a creature is granted a keyword (e.g., trample) or a P/T bonus via a targeted 'until end of turn' effect and then leaves the battlefield, `state.has_keyword(dead_id, Trample)` and `state.effective_power(dead_id)` continue to return positive values until the cleanup step clears `until_end_of_turn`. Any card whose triggered ability or replacement effect checks the keywords or power of a creature that just left the battlefield — for example, a death-watch trigger with an 'if this creature had trample' condition — will see stale data. The fix is to add `if self.get_object(creature_id).is_some_and(|o| o.zone == Zone::Battlefield)` guards to the `GrantKeyword` and `ModifyPT` arms, symmetric with the `GrantKeywordAll` guard.

_Discovered auditing: Kessig Wolf Run_

## Dynamic flashback priority silently drops intrinsic flashback option

The legal action generator in `engine.rs` (lines 1231–1245) resolves flashback cost via a priority `match`: dynamic flashback (from `GrantFlashback` in `until_end_of_turn`) wins over intrinsic flashback (`data.flashback_cost`). Only ONE `CastSpell` action is generated per graveyard card regardless of how many flashback sources exist. Per the ruling 'If a card has multiple instances of flashback, you may choose any of its flashback costs to pay,' the engine must generate a separate `CastSpell` action for each available flashback cost when a card has more than one (dynamic + intrinsic). The impact is not merely a missing choice — in cross-color situations (e.g., Bump in the Night: mana cost {B} vs printed flashback {5}{R}), the engine may present zero affordable flashback options where one exists. Any card that grants dynamic flashback (Past in Flames, Snapcaster Mage, future cards using `GrantFlashback`) to cards that already have printed flashback triggers this defect.

_Discovered auditing: Past in Flames_

## `UpToTargets` nested inside `TwoTargets` silently collapses the cast action list to empty

When a spell's `target_requirement` is `TwoTargets(X, UpToTargets(N, Y))`, the `TwoTargets` handler in `generate_cast_actions_with_targets` (engine.rs:1687) calls `valid_targets_for_req` on each slot. `valid_targets_for_req` has explicit cases for concrete requirement types but no case for `UpToTargets` — it falls through to `_ => vec![]` at engine.rs:1893. The empty result for the second slot makes the Cartesian product empty, so no `CastSpell` actions are generated and the card cannot be cast. The `UpToTargets` case is handled correctly at the *top level* of both `generate_cast_actions_with_targets` (lines 1706-1733) and `build_cast_target_spec` (lines 1916-1918), but those paths are bypassed when `UpToTargets` appears inside `TwoTargets`. The fix requires adding `TargetRequirement::UpToTargets(_, inner) => valid_targets_for_req(state, caster, spell_id, inner, behavior, registry)` to `valid_targets_for_req`, and updating the `TwoTargets` Cartesian product handler to treat an empty second-slot result as a single empty-selection option so that 'mandatory first target + zero optional second targets' remains castable. Any card using `TwoTargets(A, UpToTargets(N, B))` as its `target_requirement` is fully uncastable until this is fixed.

_Discovered auditing: Memory's Journey_

## `AnyDamageToPlayer` and `AnyCombatDamageToPlayer` dispatch fire unconditionally for all player damage events

The `GameEvent::CombatDamageDealt` and `GameEvent::NonCombatDamageDealt` handlers in `collect_triggers` (triggers.rs lines 794–811 and 818–848) create `DamageToPlayerWatch` (and `CombatDamageWatch`) triggers for EVERY battlefield permanent that declares the matching `TriggerKind`, regardless of the source of damage or the identity of the damaged player. Cards whose oracle text restricts the trigger condition by source ('enchanted creature', 'a Vampire you control') or by target ('an opponent') have those conditions evaluated at resolution inside `on_any_damage_to_player` / `on_any_combat_damage_to_player`, not at dispatch time. This produces spurious stack entries and incorrect priority windows whenever unrelated player damage events occur, violating CR 603.2. The pattern is identical to the `SpellCast` dispatch issue already documented above. The fix is a card-behavior hook (e.g. `should_queue_damage_to_player_trigger(state, watcher_id, source_id, damaged_player, registry) -> bool`) called before the trigger is pushed, analogous to `should_queue_upkeep_trigger`. Any card with a `TriggerKind::AnyDamageToPlayer` or `TriggerKind::AnyCombatDamageToPlayer` ability whose oracle text restricts the source or target is affected.

_Discovered auditing: Curiosity_

## Targeted death triggers using `target_requirement: None` defer target selection to resolution

Several cards with 'target player/creature [effect]' SelfDies or AnyCreatureDies triggers declare `target_requirement: None` in their `TriggeredAbilityDef` and instead build the target list inline inside `on_dies` / `on_any_creature_dies`. Because `process_pending_trigger_pushes` treats `None` as untargeted, it pushes the trigger to the stack with no chosen targets, violating CR 603.3b (targets must be chosen at stack-placement), CR 603.3c (trigger must be removed if no legal targets exist), and CR 608.2b (legality re-check at resolution only fires when `chosen_targets` is non-empty). The inline target list often also omits `can_be_targeted_by`, leaving shroud creatures reachable. The correct pattern — used correctly by Pitchburn Devils and Falkenrath Noble — is to declare `target_requirement: Some(req)` and read `chosen_targets` in the resolution handler. Any card whose death trigger description says 'target [something]' but whose `TriggeredAbilityDef` has `target_requirement: None` should be checked for this pattern.

_Discovered auditing: Elder Cathar_

## `is_valid_target` redundancy guards can illegally restrict trigger targets

Some cards add "don't retarget what we already affected" guards in `is_valid_target` to prevent redundant or confusing prompts when the same effect has already been applied. For example, Snapcaster Mage filters out cards that already have a `GrantFlashback` entry in `until_end_of_turn`. These guards are not oracle-sanctioned: if the oracle text places no restriction on targeting an already-affected object, the guard is wrong and can silently cause a trigger to be removed from the stack (CR 603.3c) when no other legal targets exist. Any card that adds a custom exclusion to `is_valid_target` beyond the oracle's stated restrictions should be treated as a suspected bug.

_Discovered auditing: Snapcaster Mage_

## `EndStepTrigger` dispatch in `resolve_next_trigger` silently drops triggers when source has left the battlefield

The `resolve_next_trigger` match in triggers.rs wraps the `EndStepTrigger` arm in `if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield)`, meaning any end-step trigger whose source permanent has left the battlefield between stack-placement and resolution is silently discarded. Per CR 112.7a, triggered abilities exist on the stack independently of their source; destruction of the source after the trigger is placed does not counter it. The `UpkeepTrigger` arm in the same match has no zone guard, and the ETB arm explicitly notes 'ETB triggers resolve even if the source has left the battlefield.' The asymmetry is bugs for any card whose end-step trigger targets something other than itself — those effects are valid even without the source. Card-level `on_end_step` handlers that read self characteristics (transform state, controller) still need their own zone checks, but the engine-level guard must be removed so that triggers with independent effects (like Reaper from the Abyss destroying a target creature) are not incorrectly suppressed.

_Discovered auditing: Reaper from the Abyss_

## Oracle word 'card' excludes tokens — `!o.is_token` guard required

In oracle text, the word 'card' is a precise rules term (CR 109.1): a 'card' is a physical game object, not a token or copy. Whenever oracle text says '[Type] card' (e.g., 'Zombie card', 'creature card'), the engine must filter with `!o.is_token`. When oracle text says just '[Type]' or '[Type] you control' without 'card', tokens are included. The `is_zombie` / `matches_filter` helpers in this codebase check the registry and `obj.subtypes` for type membership but have no built-in notion of 'card vs token' — that distinction must be applied at the call site. Cards that count, return, or interact with 'X cards' in a specific zone (graveyard, hand, library) should be checked to ensure the `is_token: false` guard is present wherever the oracle uses the word 'card.'

_Discovered auditing: Unbreathing Horde_

## Source of `requires_tap` activated ability is not excluded from the auto-tap mana plan

The `legal_actions` function in `engine.rs` (lines 722–729) only excludes the source permanent from the auto-tap mana-source pool when the ability has `SacrificeCost::SacrificeThis`. For any `requires_tap: true` ability with any other sacrifice cost (including `SacrificeCost::None`), the source is included in `ability_sources` via the `else` branch at line 728 and can be selected by `compute_autotap` as a mana source for that same ability's mana cost. If the source also has a mana ability (e.g., a land with `{T}: Add {C}`), the engine generates and executes an `ActivateAbility` action where the source appears in both the mana tap plan and the activation tap target — a single tap used twice, violating CR 602.2h. The practical effect: the ability is offered as legal when the player lacks sufficient mana from other sources, and the source's own mana production is illegally credited toward its activation cost. Fix: extend the exclusion condition at `engine.rs:722` from `if ability_has_sac_this` to `if ability_has_sac_this || ab.requires_tap`. Any card with both a mana ability and a tap-cost non-sacrifice activated ability is affected.

_Discovered auditing: Gavony Township_
