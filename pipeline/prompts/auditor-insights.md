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

No new generalizable insights discovered — the inline damage pattern is already documented in auditor-insights.md.

No new generalizable insights discovered. The entering modifier pattern (`entering_modifier_zones` + `modify_creature_entering_counters`) is already well-documented in the codebase and handles the graveyard-zone replacement effect correctly.

No new generalizable insights discovered. The entering modifier pattern (`entering_modifier_zones` + `modify_creature_entering_counters`) is already documented in auditor-insights.md. The card's implementation correctly follows this pattern.

No new insights to add — the DFC name/zone-change pattern is already documented in auditor-insights.md (discovered during a previous audit of this exact card).

No new generalizable insights discovered — all patterns found (inline damage, zone-change subtype persistence, "for as long as" LTB-only implementation, obj-only subtype check vs registry) are already documented in auditor-insights.md or covered by existing required checks (8a, 8d, 8e, 8h).

No new generalizable insights discovered. The "enters with" replacement-effect bypass is specific to X-cost entering counters (the non-X "enters with" cards already use `entering_with_counters`). The counter-removal cost pattern is already documented in auditor-insights.md.

No new generalizable insights discovered — the tapped-token creation pattern and flashback cost-reduction bypass are both engine-wide issues already visible from the required checks (8g, 8i). The existing insight on zone-change cleanup already covers the general category of "API lacks a parameter for correct state."

No new generalizable insights discovered. The DFC zone-change stale-name pattern is already documented in auditor-insights.md.

No new generalizable insights discovered. The DFC name/zone-change pattern is already documented in auditor-insights.md. The intervening-if trigger-time check gap is engine-wide but specific to the trigger dispatch architecture rather than a code pattern future auditors would miss — it's visible from the 8b check procedure.

No new generalizable insights discovered. The sequential "each player" discard pattern is already documented in auditor-insights.md. The `abilities_activated_this_turn` zone-change issue is a specific instance of the already-documented zone-change cleanup gap (auditor-insights.md § "Zone-change cleanup does not reset characteristic modifications").

No new generalizable insights discovered. The patterns checked (subtype dual-source lookup, upkeep trigger dispatch, token creation) are already covered by existing required checks and insights.

No new generalizable insights discovered. The exile-cost prompt flow (ChooseExileFromGraveyard) and cast-from-graveyard vs flashback distinction are already well-covered by the required check 8i and the existing auditor-insights entries.

### Triggered ability resolution skips target legality check (CR 608.2b)
The `resolve_next_trigger` function in `triggers.rs` dispatches each trigger variant directly to its handler without re-checking target legality. Per CR 608.2b, a triggered ability with targets must verify that all targets are still legal when it tries to resolve; if all targets are illegal, the ability is removed from the stack. The target validity infrastructure exists for spells (`resolve_spell` in `stack.rs` calls `is_target_legal` + `is_valid_target`), but the parallel check is missing from the trigger resolution path. Any targeted triggered ability — ETB, death, spell-cast, upkeep, etc. — will resolve even when its targets have become illegal (moved zones, gained hexproof, etc.). To audit: check whether a card's trigger targets could become illegal between the trigger going on the stack and resolving, and whether the handler assumes the target is valid without checking.
Discovered auditing: Snapcaster Mage

No new generalizable insights to add — the counter-removal cost pattern and controller-after-move_object pattern are already documented in auditor-insights.md (both discovered during previous audits of this same card). The zone-change cleanup gap for subtypes/colors is also already documented.

No new generalizable insights discovered. The zone-change cleanup issue (TemporaryEffect entries surviving target zone changes) is a specific instance of the already-documented "zone-change cleanup does not reset characteristic modifications" insight, extended to the `until_end_of_turn` tracking system rather than object fields.

No new generalizable insights to add. The DFC name/zone-change pattern (Finding 1) and the intervening-if trigger dispatch gap (Finding 2) are already covered by existing insights and required checks. The "your" vs "each" step-trigger scoping issue (Finding 3) is a specific instance of the trigger dispatch system not supporting per-player trigger scopes, which is closely related to the intervening-if pattern already documented.

No new generalizable insights discovered. The damage prevention replacement effect pattern (`PreventDamageRemoveCounter` checked in both combat and non-combat damage paths) is specific to this card and already well-integrated into the engine's dual damage handlers.

No new generalizable insights discovered. The `dynamic_pt` pattern for equipment/aura P/T modification is well-established in the engine, and the graveyard card-type counting pattern is straightforward.

No new generalizable insights discovered. The watcher-trigger resolution battlefield gate (Finding 1) is an engine-wide pattern already visible from the 8b check procedure. The unconditional zone-move pattern (Finding 2) is card-specific.

No new generalizable insights discovered. The attack-trigger zone-check pattern (Finding 1) is a specific instance of the broader engine behavior where trigger resolution checks source presence, which is already partially documented in the "Triggered ability resolution skips target legality check" insight. The root cause — blanket zone-gating in `resolve_next_trigger` — affects multiple trigger kinds beyond just attacks, but the pattern is visible from the existing 8b check procedure.

No new generalizable insights discovered. The `power.is_some()` creature-proxy pattern is already partially addressed by the existing 8d required check (subtype/type checks), and the zone-change cleanup insight already documents the persistence of `card_types` through zone changes.

No new generalizable insights discovered. The trigger-queue targeting timing issue (Finding 1) is card-specific to implementations that use `target_requirement: None` for targeted triggers — the fix is straightforward (use the existing `TargetRequirement` enum). The "your upkeep" vs "each upkeep" engine limitation (Finding 2) is already partially visible from the existing 8b check procedure and affects the trigger dispatch architecture broadly.

### Activated ability targeting omits protection-from-source check
`generate_ability_targets` (engine.rs:1988) receives `source_id` but passes `None` to `can_be_targeted_by` via the wrapper `can_be_targeted` (engine.rs:2018). This means protection from the ability's source permanent is never checked when enumerating valid targets for activated abilities. The spell-targeting path (`valid_targets_for_req` at engine.rs:1761) correctly passes `Some(spell_id)`. Any card with a targeted activated ability — fight abilities, tap-to-damage, tap-to-exile — can illegally target creatures with protection from the source's color, type, or other quality. To check: verify whether the activated ability target enumeration threads `source_id` through to the protection check.
Discovered auditing: Daybreak Ranger

No new generalizable insights discovered. The card is a straightforward instant with flashback; all patterns checked are already covered by existing required checks and insights.

No new generalizable insights discovered. The sequential mass-destruction pattern (no indestructible snapshot in spell-based destroy loops) is specific to the interaction between `KeepOneDestroyRest` and conditional indestructible, and is already partially addressed by the SBA snapshot at sba.rs:107-110. The fix pattern (snapshot before processing) is visible from the existing SBA code.

No new generalizable insights discovered. The `attached_to_player` zone-change gap is a specific instance of the already-documented "Zone-change cleanup does not reset characteristic modifications" insight. The resolution-time targeting pattern for triggered abilities is card-specific (most other targeted triggers in the codebase correctly use `target_requirement`).

### Activated ability targeting omits protection-from-source check
`generate_ability_targets` (engine.rs:1988) receives `source_id` but passes `None` to `can_be_targeted_by` via the wrapper `can_be_targeted`. This means protection from the ability's source permanent is never checked when enumerating valid targets for activated abilities. The spell-targeting path (`valid_targets_for_req`) correctly passes `Some(spell_id)`. Any card with a targeted activated ability — fight abilities, tap-to-damage, tap-to-exile, etc. — can illegally target creatures with protection from its color, type, or other relevant quality. To check: compare `generate_ability_targets` calls to `can_be_targeted` vs `can_be_targeted_by` and verify the source_id is threaded through.
Discovered auditing: Daybreak Ranger

### Non-creature death-watchers missed in simultaneous destruction
The `simultaneously_dead` list in `triggers.rs` (used to include permanents destroyed in the same event batch as potential death-watch trigger sources) only tracks `CreatureDied` events. Non-creature permanents — enchantments, artifacts, planeswalkers — that have `AnyCreatureDies` triggered abilities are excluded when destroyed simultaneously with creatures. Their death-watch triggers are silently dropped. Any card that is NOT a creature but watches for creature deaths should be checked for correct trigger creation when the watcher is destroyed in the same batch as the watched creatures.
Discovered auditing: Gutter Grime

No new generalizable insights discovered. The card is a straightforward combat-damage-to-player trigger with counter placement; all patterns checked are already covered by existing required checks and insights.

No new generalizable insights discovered. The upkeep trigger dispatch scoping issue (Finding 1) and the trigger resolution zone gate (Finding 2) are engine-wide patterns already visible from the 8b check procedure and previously noted in other audit reports. The target legality re-check gap (Finding 3) is already documented in auditor-insights.md.

No new generalizable insights discovered. The upkeep trigger battlefield gate (Finding 1) is an instance of the engine-wide blanket zone-gating pattern visible from the 8b check procedure. The TemporaryEffect zone-change persistence (Finding 2) is already documented in auditor-insights.md as a known extension of the zone-change cleanup gap.

No new generalizable insights discovered. The token-copy-of-DFC transform issue (Finding 4) is specific to the interaction between `create_token_copy` and `apply_transform` — it's a targeted bug rather than a pattern that would surface from a different audit angle. The X-cost-in-activated-ability issue (Finding 1) is specific to cards that proxy another card's mana cost into their own ability cost, which is rare. The activated-ability-bypasses-stack issue (Finding 2) is already well-known as an engine architectural limitation.

No new generalizable insights discovered. The inline damage pattern (Finding 1) is already documented in auditor-insights.md ("Inline damage bypasses engine protections"). The flashback cost-reduction bypass (Finding 2) is a specific instance of the casting atomicity check (8i) — the pattern is narrow (flashback cost path vs normal cost path) and already visible from the 8i procedure.

### Token copy of a non-registry token loses characteristics via sentinel card_id
`create_token_copy` (state.rs) reads keywords, subtypes, card_types, and colors from `registry.card_data(card_id)`. Tokens created by `create_token_with_subtypes` (generic tokens like "2/2 black Zombie") have `card_id = CardId(0)`, a sentinel not in the registry. The lookup returns `None`, and `.unwrap_or_default()` silently drops all four fields to empty vectors. Only name and P/T (read from the object) survive. Any card that creates token copies should be checked for whether the source could be a generic token — if so, the copy loses creature types, subtypes, keywords, and colors. The fix is to read these fields from the object when the registry lookup fails, or to always read from the object for these fields.
Discovered auditing: Cackling Counterpart

### Token copy of a non-registry token loses characteristics via sentinel card_id
`create_token_copy` reads keywords, subtypes, card_types, and colors from `registry.card_data(card_id)`. Tokens created by `create_token_with_subtypes` (generic tokens like "2/2 black Zombie") have `card_id = CardId(0)`, a sentinel that is not in the registry. The lookup returns `None`, and `.unwrap_or_default()` silently drops all four fields to empty vectors. Only name and P/T (read from the object) survive. Any card that creates token copies should be checked for whether the source could be a generic token — if so, the copy loses creature types, subtypes, keywords, and colors. The fix is to read these fields from the object (which has the correct values set at token creation) rather than from the registry, or to fall back to the object when the registry lookup fails.
Discovered auditing: Cackling Counterpart

### Non-creature death-watchers missed in simultaneous destruction
The `simultaneously_dead` list in `triggers.rs` (used to include permanents destroyed in the same event batch as potential death-watch trigger sources) only tracks `CreatureDied` events. Non-creature permanents — enchantments, artifacts, planeswalkers — that have `AnyCreatureDies` triggered abilities are excluded from this list when destroyed simultaneously with creatures. This means their death-watch triggers are silently dropped. Any card that is NOT a creature but watches for creature deaths (e.g., enchantments like Gutter Grime, artifacts like Skullclamp-style effects) should be checked for correct trigger creation when the watcher is destroyed in the same batch as the creatures it watches.
Discovered auditing: Gutter Grime

No new generalizable insights discovered. The back-face missing trigger (Finding 1) is card-specific. The static-as-triggered pattern (Finding 2) is already tracked as Bug BK. The DFC name/zone-change pattern (Finding 3) is already documented in auditor-insights.md.

No new generalizable insights discovered. The inline damage pattern is already documented in auditor-insights.md.

No new generalizable insights discovered. The attack-trigger battlefield gate (Finding 2) is already partially documented in auditor-insights.md under the existing insight about trigger resolution zone-gating. The delayed-trigger-as-turn-based-action pattern (Finding 1) is specific to the `end_of_combat_exiles` mechanism and not yet a generalizable pattern seen in other cards.

No new generalizable insights discovered. The flashback cost-reduction bypass is specific to the engine's flashback cast path and is already partially visible from the 8i check procedure. The cast-from-graveyard affordability mismatch is a specific instance of the same code-path divergence.

No new generalizable insights discovered. Finding 1 (TemporaryEffect zone-change persistence) is already documented in auditor-insights.md. Finding 2 (activated ability protection-from-source omission) is already documented in auditor-insights.md. Finding 3 (activated abilities bypass stack) is noted in auditor-insights.md as a known engine architectural limitation.

No new generalizable insights discovered. The activated-ability-bypasses-stack pattern is already documented as a known engine architectural limitation in previous audit reports.

No new generalizable insights discovered. The `power.is_some()` creature-proxy pattern (Finding 1) is already noted in auditor-insights.md as partially addressed by check 8d. The activation cost atomicity issue (Finding 2) is a specific instance of the engine's `ActivatedAbilityDef` lacking support for non-mana, non-sacrifice, non-tap costs, which is already visible from the check 8i procedure and documented in the counter-removal cost insight.

No new generalizable insights discovered. The "as enters" vs ETB trigger distinction is specific to the engine's lack of a replacement-effect entry point for name/mode choices during permanent entry, which is already partially visible from the `entering_modifier_zones` pattern used by Dearly Departed. The registry-only name restriction is an inherent engine limitation, not a code pattern that would surprise future auditors.

No new generalizable insights discovered. Finding 1 (zone-change toughness persistence) is already documented in auditor-insights.md under "Zone-change cleanup does not reset characteristic modifications." Finding 2 (life gain/loss event gap) is engine-wide but specific to the event system architecture rather than a code pattern auditors would miss. Finding 3 (activated abilities bypass stack) is already noted in auditor-insights.md as a known engine architectural limitation.

No new generalizable insights discovered. Both findings are already documented in auditor-insights.md: "Triggered ability resolution skips target legality check (CR 608.2b)" and the TemporaryEffect zone-change persistence pattern is noted as an extension of "Zone-change cleanup does not reset characteristic modifications."

No new generalizable insights discovered. The flashback cost-reduction bypass is already noted in multiple previous audit reports and is visible from the 8i check procedure.

No new generalizable insights discovered. The phantom-trigger dispatch scoping issue ("your upkeep" vs "each upkeep") and the trigger-resolution battlefield gate are both engine-wide patterns already visible from the existing 8b check procedure and previously noted in other audit reports.
