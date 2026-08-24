---
id: splinterfright-01
status: fixed
card: Splinterfright
audit_run_id: 2026-04-19-splinterfright-audit
audit_model: sonnet
audit_tokens: 27529
audit_duration: 462
fixed_sha: 0b001c0
fixed_at: 2026-08-24T00:53:35Z
test_file: mtg-engine/tests/token_is_not_a_card.rs
fix_note: Oracle text saying "card" now excludes tokens (CR 109.1); added GameState::is_card.
---

## Audit Finding

**Oracle text:**
> Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard.

**Code:**
> .filter(|o| o.power.is_some())

**Description:**
The oracle text uses the word 'cards', which per CR 109.1 excludes tokens. The graveyard filter uses `o.power.is_some()` as a creature proxy but never checks `!o.is_token`. Per CR 704.5d, tokens in non-battlefield zones are removed by SBA — but that removal happens in a discrete SBA pass, not atomically at the moment the token enters the graveyard. Any evaluation of `effective_power` that occurs between a token entering the graveyard and the next SBA pass (for example, during trample overflow calculation mid-combat-damage-step, or when Corpse Lunge reads the exiled creature's power immediately after exile) would count the token and inflate Splinterfright's P/T. Moorland Haunt (moorland_haunt.rs:55) uses `o.power.is_some() && !o.is_token` for an identical 'creature card in graveyard' query, establishing the correct pattern.

**Engine path:** mtg-engine/src/cards/isd/splinterfright.rs:47

**Required check:** 8d

**Affected cards:**
- Boneyard Wurm

## Tests

### cda_does_not_count_tokens_in_graveyard
Scenario: A creature token is placed directly in Zone::Graveyard (simulating the window before SBA cleanup); verify that Splinterfright's effective_power does not count the token, returning 0 rather than 1.

