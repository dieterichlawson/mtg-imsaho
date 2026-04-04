## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flash
Deathtouch
**Type line**: Creature — Snake
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flash grants instant-speed casting: correctly handled — `engine.rs` line 501 reads `data.keywords.contains(&Keyword::Flash)` and sets `can_cast_timing = true` unconditionally for Flash, allowing the card to be cast any time the player has priority (pass)
- Deathtouch marks damage correctly: `combat.rs` line 456 checks `has_keyword(source, Keyword::Deathtouch, registry)` and sets `obj.dealt_deathtouch_damage = true` when any damage is applied to a target (pass)
- SBA destroys creature dealt deathtouch damage: `sba.rs` line 76 condition `(deathtouch && damage > 0)` triggers `try_destroy` even when damage is less than toughness (pass)
- Deathtouch with trample assigns minimum 1 per blocker: `combat.rs` lines 239-240 set `lethal = 1` when attacker has deathtouch, so remaining power correctly tramples through (pass)
- Indestructible survives deathtouch: SBA sends deathtouch-killed creatures through `try_destroy`, which respects Indestructible — no special case needed (pass)
- Summoning sickness applies when Ambush Viper ETBs: `state.rs` `move_object` sets `summoning_sick = true` when entering the battlefield, but Haste would override. Ambush Viper has no Haste, so it can't attack the turn it enters — correct (pass)
- Flash usable from graveyard if flashback granted: `engine.rs` line 697 applies the same Flash check for graveyard casts, so a dynamically granted flashback + Flash still yields instant-speed — no issue for Ambush Viper itself since it has no flashback cost (pass)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flash and Deathtouch keywords present on card data: `tests/innistrad_cards.rs:87` (`ambush_viper_has_flash_and_deathtouch`)
- Flash allows casting during opponent's turn: `tests/keywords.rs:412` (`flash_creature_castable_at_instant_speed`) using Ambush Viper specifically
- Normal creature (no Flash) cannot be cast during opponent's turn: `tests/keywords.rs:430` (`normal_creature_not_castable_on_opponent_turn`)
- Deathtouch kills with 1 damage (SBA): `tests/keywords.rs:246` (`deathtouch_kills_with_one_damage`) using Typhoid Rats
- Deathtouch + trample assigns minimum lethal: `tests/keywords.rs:271` (`deathtouch_trample_assigns_minimum`)
- Indestructible survives deathtouch: `tests/card_mechanics.rs:942` (`indestructible_survives_deathtouch`)
- Regeneration saves from deathtouch: `tests/card_mechanics.rs:1006` (`regeneration_saves_from_deathtouch`)
- Ambush Viper dealing deathtouch damage in combat specifically: NOT TESTED (only Typhoid Rats is used in the deathtouch combat test; the behavior is identical but no Ambush Viper-specific combat test exists)
