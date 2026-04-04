## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Mana cost {3}{W}: Code uses `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])` — matches oracle.
- P/T 2/2: Code has `power: Some(2), toughness: Some(2)` — matches oracle.
- Subtype Griffin: Code has `subtypes: vec!["Griffin".into()]` — matches oracle.
- Flying keyword enforcement: `combat.rs` checks `has_keyword(attacker_id, Keyword::Flying, registry)` in `can_block_attacker`; a creature with Flying can only be blocked by Flying or Reach creatures — correct.
- Vigilance keyword enforcement: `combat.rs` checks `has_keyword(attacker_id, Keyword::Vigilance, registry)` before tapping attackers; attackers with Vigilance skip the tap — correct.
- No triggered/activated abilities or continuous effects: oracle text contains none; card implementation has `triggered_abilities: vec![], continuous_effects: vec![], additional_cost: None, flashback_cost: None` — all correct.

### Test coverage
- Keywords (Flying, Vigilance) registered correctly: `innistrad_cards.rs:69` (`abbey_griffin_has_flying_and_vigilance`) — TESTED
- Flying blocking restriction (functional combat test): NOT TESTED specifically for Abbey Griffin, but covered by general flying/combat engine tests
- Vigilance no-tap-when-attacking: NOT TESTED specifically for Abbey Griffin, but covered by general vigilance engine tests
- Mana cost / P/T / subtype data: NOT TESTED beyond the keyword test
