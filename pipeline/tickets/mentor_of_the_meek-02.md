---
id: mentor_of_the_meek-02
status: new
card: Mentor of the Meek
card_file: mtg-engine/src/cards/isd/mentor_of_the_meek.rs
created: 2026-04-15T03:46:51Z
audit_run_id: 2026-04-14-mentor_of_the_meek-audit
audit_model: opus
audit_tokens: 19660
audit_duration: 465
---

## Audit Finding

**Oracle text:**
> you may pay {1}. If you do, draw a card.

**Code:**
> `mentor_of_the_meek.rs:74-88`: `on_yes_no_choice` checks `player.mana_pool.get(ManaType::Colorless) >= 1` and iterates colored mana types — only floating mana in the pool.

**Description:**
When the player chooses "yes" to pay {1}, the code checks only the current mana pool (floating mana). Per CR 605.3a, a player may activate mana abilities during the resolution of a spell or ability whenever they are asked to pay mana. The player should be able to tap lands to generate mana for this payment. This is the same as Bug Y from AUDIT_BUGS.md — Screeching Bat was fixed to use `engine::plan_autotap_for_cost` (screeching_bat.rs), but Mentor of the Meek still uses the old pool-only check. If the player has untapped lands but no floating mana, the payment silently fails and no card is drawn.

**Engine path:**
- mentor_of_the_meek.rs:74-88 — pool-only mana check

**Required check:** 8j (Bug Y reference in audit_xcost_mana_family.rs:159)

**Affected cards:**
- Mentor of the Meek
- Any other card using the same pool-only pay pattern (Frightful Delusion per Bug Y)

## Tests

### mentor_autotap_for_payment
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Mentor of the Meek on the battlefield. Give the controller an untapped Plains but no floating mana. Enter a 1/1 creature and resolve the trigger. Player chooses "yes" on the YesNo prompt. Assert that the Plains is tapped, 1 mana is spent, and a card is drawn. Currently fails because `on_yes_no_choice` only checks floating mana.

