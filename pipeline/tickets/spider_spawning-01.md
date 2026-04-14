---
id: spider_spawning-01
status: new
card: Spider Spawning
card_file: mtg-engine/src/cards/isd/spider_spawning.rs
created: 2026-04-14T21:14:04Z
audit_run_id: 2026-04-14-spider_spawning-audit
audit_model: opus
audit_tokens: 15129
audit_duration: 441
---

## Audit Finding

**Oracle text:**
> Create a 1/2 green Spider creature token with reach for each **creature card** in your graveyard.

**Code:**
> `let creature_count = state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some() && o.id != object_id).count();` (spider_spawning.rs:37-39)

**Description:**
The code identifies creature cards in the graveyard by checking `o.power.is_some()` instead of checking `o.card_types.contains(&CardType::Creature)`. Per CR 302.1, a "creature card" is a card with the creature card type. Using `power.is_some()` as a proxy is semantically incorrect: it would count non-creature cards that have power and toughness (such as Vehicles, which are Artifact cards with P/T but no Creature type until crewed) and could miss any creature card that somehow lacked a power value. In the current Innistrad-only cardpool there are no Vehicles so this does not manifest, but the check is wrong per the rules definition. This is an engine-wide convention — `power.is_some()` is used as a creature proxy in ~30 places across the codebase — but some graveyard-counting cards (Grimoire of the Dead at grimoire_of_the_dead.rs:145, Graveyard Shovel at graveyard_shovel.rs:81) already use the more correct dual check: `o.power.is_some() || o.card_types.contains(&CardType::Creature)`. The correct fix is `o.card_types.contains(&CardType::Creature)` (or the dual check for defense-in-depth).

**Engine path:**
- mtg-engine/src/cards/isd/spider_spawning.rs:37-39

**Required check:** 8d

**Affected cards:**
- Spider Spawning
- Gnaw to the Bone (gnaw_to_the_bone.rs:35 — same `power.is_some()` pattern)
- Wreath of Geists (wreath_of_geists.rs:37 — same pattern)
- Any future card that counts "creature cards" in a zone

