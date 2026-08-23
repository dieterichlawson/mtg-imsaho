---
id: grimoire_of_the_dead-01
status: new
card: Grimoire of the Dead
audit_run_id: 2026-04-19-grimoire_of_the_dead-audit
audit_model: sonnet
audit_tokens: 22452
audit_duration: 439
---

## Audit Finding

**Oracle text:**
> They're black Zombies in addition to their other colors and types.

**Code:**
> if !obj.subtypes.contains(&"Zombie".into()) {
                            obj.subtypes.push("Zombie".into());
                        }
                        if !obj.colors.contains(&Color::Black) {
                            obj.colors.push(Color::Black);
                        }

**Description:**
The reanimation effect directly mutates obj.subtypes and obj.colors on the reanimated creatures. The move_object cleanup block in state.rs (lines 586-608) does not clear subtypes (except when is_transformed is set) or colors when a permanent leaves the battlefield. Per CR 400.7, when a creature later leaves the battlefield it becomes a new object with no memory of previous effects — it should lose the Zombie subtype and the Black color addition. Instead, those mutations persist on the object in whatever zone it moves to. A creature reanimated by Grimoire that subsequently dies would remain a 'Black Zombie' in the graveyard, corrupting zone-based checks on those objects and causing incorrect behavior if the same object is later returned to the battlefield by a different effect.

**Engine path:** mtg-engine/src/state.rs:586

**Required check:** 8a

## Tests

### zombie_subtype_cleared_when_reanimated_creature_dies
Scenario: A non-Zombie creature is placed into a graveyard, Grimoire's ability reanimates it (it becomes a Zombie on the battlefield), it is then destroyed — verify that the object in the graveyard does NOT have 'Zombie' in its subtypes.

### black_color_cleared_when_reanimated_creature_dies
Scenario: A non-black creature is reanimated by Grimoire (gains Black on the battlefield), then dies — verify that the object in the graveyard does NOT have Color::Black in its colors.

