---
id: moldgraf_monstrosity-01
status: fixed
card: Moldgraf Monstrosity
audit_run_id: 2026-04-19-moldgraf_monstrosity-audit
audit_model: sonnet
audit_tokens: 28519
audit_duration: 480
fixed_sha: 64f2e6f
fixed_at: 2026-08-24T01:12:48Z
test_file: mtg-engine/tests/resolution_time_checks.rs
fix_note: Exile applies only to the card in the graveyard; the two-creature return happens either way.
---

## Audit Finding

**Oracle text:**
> When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield. [Ruling 2011-09-22] If Moldgraf Monstrosity's ability can't exile it (perhaps because it's not still in the graveyard when the ability resolves), the two creature cards are still returned to the battlefield. [Ruling 2011-09-22] If two Moldgraf Monstrosities die simultaneously, the first ability to resolve could return the other Moldgraf Monstrosity to the battlefield. If it does, the second Moldgraf Monstrosity's ability won't exile it but it will return two more creature cards to the battlefield.

**Code:**
> let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Exile Moldgraf Monstrosity.
        state.move_object(object_id, Zone::Exile, registry);

**Description:**
Two related violations of the rulings. First (ruling 2): `state.move_object(object_id, Zone::Exile, registry)` on line 52 is called unconditionally regardless of the object's current zone. In the simultaneous-death scenario described by ruling 2 — where the first Monstrosity's trigger resolves and returns the second Monstrosity to the battlefield — the second trigger's `on_dies` is called with `object_id` referring to a battlefield permanent. The code reads its controller fine, then blindly calls `move_object(object_id, Zone::Exile, ...)`, exiling the live creature from the battlefield. Ruling 2 explicitly says the second trigger 'won't exile it.' Second (ruling 1): the `None => return` at line 48 aborts the entire ability — both the exile step and the two-creature-return — if `get_object(object_id)` returns `None`. Per ruling 1, the creature return must happen even when the exile fails. Although `None` is unreachable for Moldgraf in the current engine (non-token objects are never removed from the HashMap), the guard structure inverts the intent: a missing-object guard that swallows the main effect is structurally wrong against these rulings. The correct shape is: attempt exile only if the object is currently in Zone::Graveyard (skip silently otherwise), then always proceed to return two creatures. The LKI controller needed for 'your graveyard' should be derived from the graveyard object when present, or from the pre-captured `controller` field stored in `PendingTrigger::SelfDies` — but `on_dies` does not receive that value as a parameter (see insight).

**Engine path:** mtg-engine/src/cards/isd/moldgraf_monstrosity.rs:48

**Required check:** 8j

## Tests

### moldgraf_simultaneous_death_second_trigger_does_not_exile_live_creature
Scenario: Two Moldgraf Monstrosities owned by P0 die simultaneously; the first trigger resolves and returns the second Monstrosity to the battlefield; when the second trigger resolves, the second Monstrosity should remain on the battlefield (not be exiled), and two more creature cards from the graveyard should be returned.

### moldgraf_exile_skipped_when_already_exiled_still_returns_creatures
Scenario: Moldgraf Monstrosity dies and its trigger is placed on the stack; before the trigger resolves, the Monstrosity is exiled from the graveyard by another effect; the trigger resolves and should return two creature cards to the battlefield without attempting a second zone change on the already-exiled object.

