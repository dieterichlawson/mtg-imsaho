---
id: manor_gargoyle-01
status: new
card: Manor Gargoyle
card_file: mtg-engine/src/cards/isd/manor_gargoyle.rs
created: 2026-04-14T21:30:34Z
audit_run_id: 2026-04-14-manor_gargoyle-audit
audit_model: opus
audit_tokens: 17084
audit_duration: 338
---

## Audit Finding

**Oracle text:**
> {1}: Until end of turn, this creature loses defender and gains flying.

**Code:**
> `state.until_end_of_turn.push(TemporaryEffect::RemoveKeyword { target: object_id, keyword: Keyword::Defender });` — manor_gargoyle.rs:67
> `move_object` cleanup block (state.rs:572-583) does not purge `until_end_of_turn` entries targeting the departing object.

**Description:**
If Manor Gargoyle activates its ability (pushing RemoveKeyword(Defender) and GrantKeyword(Flying) into `until_end_of_turn`), then leaves the battlefield and re-enters in the same turn, the stale temporary effects still apply to the returned permanent. The returned Gargoyle incorrectly lacks Defender (due to leftover RemoveKeyword) and has Flying (due to leftover GrantKeyword), and consequently lacks Indestructible (since the ConditionalKeyword checks SelfHasKeyword(Defender) which sees the stale RemoveKeyword). Per CR 400.7, an object that changes zones becomes a new object with no memory of its previous existence — the until-end-of-turn effects should not carry over.

**Engine path:**
- state.rs:572-583 (move_object LTB cleanup — does not purge until_end_of_turn)
- state.rs:1204-1210 (has_keyword RemoveKeyword check — matches by ObjectId without zone_change_count validation)
- state.rs:1253-1255 (has_keyword GrantKeyword check — same ObjectId match without zone_change_count)

**Required check:** 8a

**Affected cards:**
- Manor Gargoyle
- Any card that uses TemporaryEffect entries (RemoveKeyword, GrantKeyword, ModifyPT, CantBlock, etc.) and could leave/re-enter the battlefield in the same turn — this is an engine-wide issue

