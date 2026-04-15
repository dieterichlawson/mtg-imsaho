---
id: mentor_of_the_meek-03
status: closed-duplicate
card: Mentor of the Meek
card_file: mtg-engine/src/cards/isd/mentor_of_the_meek.rs
created: 2026-04-15T03:46:51Z
audit_run_id: 2026-04-14-mentor_of_the_meek-audit
audit_model: opus
audit_tokens: 19660
audit_duration: 465
duplicate_of: merged-trigger-source-zone-gate-02
---

## Audit Finding

**Oracle text:**
> Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.

**Code:**
> `triggers.rs:1266-1268`:
> ```rust
> let watcher_zone = state.get_object(watcher_id).map(|o| o.zone);
> if matches!(watcher_zone, Some(Zone::Battlefield | Zone::Graveyard)) {
> ```
> `mentor_of_the_meek.rs:41-44`:
> ```rust
> let controller = match state.get_object(self_id) {
>     Some(o) if o.zone == Zone::Battlefield => o.controller,
>     _ => return,
> };
> ```

**Description:**
Per CR 113.7a, once a triggered ability is on the stack, it exists independently of its source. If Mentor of the Meek's EnterWatch trigger is created and placed on the trigger queue, then Mentor is destroyed before the trigger resolves, the trigger should still resolve — the "you may pay {1}, draw a card" effect does not reference the source permanent. However, both the engine dispatch (triggers.rs:1267) and the card's own handler (mentor_of_the_meek.rs:42-43) gate on the watcher still being on the battlefield, silently dropping the trigger. This is an engine-wide pattern: the `resolve_next_trigger` dispatch for `EnterWatch` always checks the watcher's current zone.

**Engine path:**
- triggers.rs:1266-1268 — dispatch zone check silences trigger if watcher left battlefield
- mentor_of_the_meek.rs:41-44 — card handler also checks zone

**Required check:** 8b

**Affected cards:**
- Mentor of the Meek
- Champion of the Parish (also uses AnyCreatureEnters via EnterWatch)
- Any card with EnterWatch triggers whose effects don't reference the source

## Tests

### mentor_trigger_resolves_after_removal
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Mentor of the Meek on the battlefield. Give the controller {1} floating mana and a library card. Enter a 1/1 creature (creates EnterWatch trigger). Before resolving the trigger, destroy Mentor (move to graveyard). Resolve the trigger. Assert that the YesNo pay choice IS presented — per CR 113.7a, the trigger exists independently on the stack and should resolve even though Mentor is gone.
