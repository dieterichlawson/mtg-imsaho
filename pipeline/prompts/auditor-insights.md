# Auditor Insights — Discovered by Previous Audits

This file contains generalizable insights discovered by audit agents during
their work. Each insight describes a pattern that could cause bugs in OTHER
cards, not just the card that was being audited.

**Do NOT add card-specific findings here.** Only add patterns that would
help future auditors find bugs they might otherwise miss.

<!-- Auditors: append new insights below this line -->

## Inline damage bypasses engine protections
Cards that apply damage by directly writing `obj.damage_marked += N` instead of
using the central `PendingEffect::DealDamage` path bypass the damage-processing
results enumerated in CR 120.3 and the replacement-effect layer (CR 614):
protection (702.16), hexproof/ward (702.11/702.21), lifelink (702.15),
planeswalker loyalty removal (120.3c / 306.8), battle defense removal
(120.3d), damage prevention, and any "whenever damage is dealt" triggers.
Always check whether a card's damage code goes through the central handler
in `resolve_single_effect()` (engine.rs).

## Zone-change cleanup does not reset characteristic modifications
The cleanup block in `move_object()` (state.rs) does NOT clear `subtypes`,
`keywords`, `colors`, `power`, `toughness`, `card_types`, or `name` when an
object leaves the battlefield. Any card that modifies these fields at runtime
(e.g., adding a subtype, granting a keyword) will have those modifications
persist incorrectly through zone changes, violating MTG rule 400.7. Check
whether the card modifies any of these fields and whether that modification
would survive a zone change.

## "For as long as you control [source]" requires continuous re-evaluation
Per CR 611.2b, a "for as long as" duration ends as soon as its condition
becomes false — not just when the source leaves the battlefield. Effects
with "for as long as you control [source]" must end when the source's
controller changes even if the source stays on the battlefield (e.g., Act
of Treason takes temporary control without a zone change). An implementation
that only uses `on_leave_battlefield` will miss this case. Check whether the
card has any mechanism to detect controller changes on the source permanent.

### "Each player" choice effects require simultaneous resolution
Per CR 101.4 (APNAP), when multiple players must make choices or take
actions at the same time, the active player chooses first, then each other
player in turn order — but then the actions happen SIMULTANEOUSLY. A card
like "each player discards a card" resolves with all choices locked in
before any discards occur, so no player's choice can affect another's
chosen state. The engine's chained-choice pattern (resolve one player's
choice, then set up the next player's) forces sequential execution,
letting earlier players' resolved choices trigger abilities or alter
game state before later players choose. Any card with "each player
discards/sacrifices/chooses" should be checked for whether simultaneity
matters per its rulings.
Discovered auditing: Liliana of the Veil

### DFC face-dependent `is_valid_target` breaks with multiple copies
The `CardBehavior::is_valid_target` trait method receives `caster: PlayerId` but
not the activating object's `ObjectId`. Double-faced cards that need different
targeting rules per face (e.g., front face targets "creature with flying", back
face targets "any creature") must work around this by searching for a matching
object via `state.objects.values().find(...)`. This search is non-deterministic
(HashMap iteration order) and returns an arbitrary copy when the player controls
multiple instances in different transform states. Check any DFC with face-dependent
targeting logic in `is_valid_target` for this pattern.
Discovered auditing: Daybreak Ranger

### DFC transform and zone-change cleanup: `obj.name` has no registry fallback
When a DFC transforms, `obj.name` is updated to the back-face name (either
manually or via `helpers::apply_transform`). When the permanent later leaves
the battlefield, `move_object` clears `is_transformed` but not `name`. The
engine has registry-based fallbacks for keywords and subtypes (checking
`is_transformed` + `back_face_data()`), which mask stale object-level values
for those fields. However, `obj_name()` reads `obj.name` directly with NO
registry fallback. This makes `name` the most impactful stale field for DFCs
after zone changes — the card retains its back-face name in non-battlefield
zones, violating CR 712.8a ("While a double-faced card is outside the game
or in a zone other than the battlefield or stack, it has only the
characteristics of its front face"). Also check whether a DFC card uses
`helpers::apply_transform` (the correct path) vs manual transform.
Discovered auditing: Bloodline Keeper // Lord of Lineage

### Counter-removal activation costs are not supported by the engine
`ActivatedAbilityDef` only supports mana costs, tap requirements, and sacrifice
costs. It has no field for "remove N counters" as an activation cost. Cards that
require counter removal as a cost (text like "Remove three study counters from ~")
must handle it manually in `on_activate_ability`. Per CR 601.2h / 602.2b, costs
are paid in any order within the non-random group, but each cost must actually
be paid — the stated action must occur (118.11). When such a card also
sacrifices itself as part of the same cost, the counter removal is sometimes
skipped entirely: the sacrifice moves the object to the graveyard, which clears
all counters via `move_object` in one step, so the "remove N counters" action
never actually runs. The counter count is wrong at the moment of sacrifice (the
object has its full counter count instead of N minus the cost). Check any card
with "Remove N counters" in its activation cost to verify the counters are
removed first (or independently of the sacrifice), and that the count is
correct at the time of any simultaneous sacrifice or zone change.
Discovered auditing: Grimoire of the Dead

### Controller update after move_object causes stale EnteredBattlefield events
Cards that move objects to the battlefield "under your control" (reanimation,
steal effects, etc.) often call `state.move_object(id, Zone::Battlefield, registry)`
first, then set `obj.controller = controller` afterward. The `move_object` function
emits an `EnteredBattlefield` event using the object's controller at move time —
which is the previous controller (typically the owner), not the intended new
controller. While the trigger dispatch system currently re-reads controller from
state, the event itself stores the wrong value. This also affects the `EnterWatch`
trigger's `entered_controller` field. Check any card that changes controller after
`move_object` — the controller should be set BEFORE the move, or `move_object`
should accept a controller parameter.
Discovered auditing: Grimoire of the Dead
