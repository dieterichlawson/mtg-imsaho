# Audit: Ambush Viper

## Reference (Scryfall/API)
- **Name:** Ambush Viper
- **Mana Cost:** {1}{G}
- **Type:** Creature — Snake
- **Oracle:** Flash, Deathtouch
- **P/T:** 2/1

## Implementation: `ambush_viper.rs`
- **Name:** Ambush Viper -- CORRECT
- **Mana Cost:** {1}{G} -- CORRECT
- **Type:** Creature — Snake -- CORRECT
- **P/T:** 2/1 -- CORRECT
- **Keywords:** Flash, Deathtouch -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flash\nDeathtouch
**Type line**: Creature — Snake
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Ambush Viper", cost {1}{G}, 2/1, type Creature — Snake, keywords [Flash, Deathtouch]. Vanilla creature with keywords only, no behavior needed beyond card_data.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/169/ambush-viper
**Oracle text**: Flash\nDeathtouch
**Type line**: Creature — Snake
**Mana cost**: {1}{G}
**P/T**: 2/1
**Keywords**: Flash, Deathtouch
**Status**: PASS

### Code issues
No issues found.

### Checklist
- [x] Mana cost matches: Oracle `{1}{G}` = Code `Generic(1), Colored(Green)`
- [x] Card types correct: Oracle `Creature` = Code `CardType::Creature`
- [x] Supertypes correct: none expected, none in code
- [x] Subtypes correct: Oracle `Snake` = Code `"Snake"`
- [x] Power/toughness correct: Oracle `2/1` = Code `power: Some(2), toughness: Some(1)`
- [x] Keywords correct and complete: Oracle `Flash, Deathtouch` = Code `Keyword::Flash, Keyword::Deathtouch`
- [x] Oracle text field matches: `"Flash\nDeathtouch"`
- [x] No triggered abilities, flashback, or continuous effects needed
- [x] No anti-patterns (vanilla keyword creature, no behavior code)

### Tricky interactions checked
- Flash casting at instant speed: PASS (engine keyword behavior, tested)
- Deathtouch lethal damage: PASS (engine keyword behavior)

### Test coverage
- Flash + Deathtouch keywords present: `mtg-engine/tests/innistrad_cards.rs:87` (ambush_viper_has_flash_and_deathtouch)
- Flash allows instant-speed casting: `mtg-engine/tests/keywords.rs:412` (flash_creature_castable_at_instant_speed)
- Deathtouch combat behavior: tested at engine level (not card-specific)

### UI presentation
- LLM card knowledge: present in `mtg-player/src/llm.rs` — accurate description

## Audit — 2026-04-02 20:28

**Oracle text source**: Scryfall API (cached 2026-04-01) — https://scryfall.com/card/isd/169/ambush-viper
**Oracle text**: Flash\nDeathtouch
**Type line**: Creature — Snake
**Status**: PASS

### Code issues
No issues found.

All card data verified against oracle:
- Name: "Ambush Viper" — matches
- Mana cost: `Generic(1), Colored(Green)` — matches oracle `{1}{G}`
- Card types: `CardType::Creature` — matches oracle `Creature`
- Supertypes: none — correct (none on oracle)
- Subtypes: `"Snake"` — matches oracle `Snake`
- P/T: `power: Some(2), toughness: Some(1)` — matches oracle `2/1`
- Keywords: `Keyword::Flash, Keyword::Deathtouch` — matches oracle `Flash, Deathtouch`
- Oracle text field: `"Flash\nDeathtouch"` — matches
- No triggered abilities, flashback, continuous effects, or additional costs — correct for a vanilla keyword creature

### Tricky interactions checked
- Flash instant-speed casting: PASS — engine checks `Keyword::Flash` in `engine.rs:501` to allow casting anytime the player has priority, verified in test
- Deathtouch lethal damage via SBA: PASS — `sba.rs:76` correctly destroys creatures with `dealt_deathtouch_damage && damage > 0`; `combat.rs:456` marks `dealt_deathtouch_damage = true` on the target
- Deathtouch + trample interaction: PASS — `combat.rs:239` treats 1 damage as lethal when attacker has deathtouch, allowing remaining power to trample through

### Test coverage
- Flash + Deathtouch keywords present: `innistrad_cards.rs:87` (ambush_viper_has_flash_and_deathtouch)
- Flash allows instant-speed casting: `keywords.rs:412` (flash_creature_castable_at_instant_speed, uses Ambush Viper directly)
- Deathtouch kills with one damage: `keywords.rs:246` (deathtouch_kills_with_one_damage, uses Typhoid Rats — engine-level)
- Deathtouch + trample assigns minimum: `keywords.rs:271` (deathtouch_trample_assigns_minimum — engine-level)
