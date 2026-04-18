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
