## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
**Type line**: Creature — Zombie Drake
**Status**: ISSUE

### Code issues

- Engine auto-selects which creature to exile; player cannot choose
  - Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`
  - Code does: In `mtg-engine/src/engine.rs` around line 1574–1600, the engine picks creatures to exile by sorting descending by power (`exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first`) and taking the first N. No player choice is presented. The cast action generated for `ExileCreaturesFromGraveyard(n)` carries no exile target specification (eligible_sacrifices returns `vec![]` at ~line 554); the specific creature to exile is determined entirely by the engine heuristic at submit time. The player cannot choose which creature card from their graveyard to exile as the cost, even when multiple eligible creature cards are present.

### Tricky interactions checked

- Exactly one creature exiled (not zero, not more): PASS — `if creature_count < *n { continue; }` blocks casting when fewer than N creatures are available; `exile_candidates.into_iter().take(n).collect()` caps the exile at exactly N.
- Cannot cast without exiling a creature card: PASS — eligibility check at ~line 545–553 (action generation) and ~line 714–724 (flashback path) both enforce this.
- Spell itself not eligible for exile as its own cost: PASS — `o.id != obj.id` / `o.id != *object_id` exclusion in all three eligibility filters (action gen hand path, action gen graveyard/flashback path, submit_action path).
- Card data mana cost {1}{U}{U}: PASS — `ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue), ManaSymbol::Colored(Color::Blue)])`.
- Card types, subtypes, P/T: PASS — `card_types: vec![CardType::Creature]`, `subtypes: vec!["Zombie".into(), "Drake".into()]`, `power: Some(3)`, `toughness: Some(4)`.
- Flying keyword: PASS — `keywords: vec![Keyword::Flying]`.
- on_resolve moves to battlefield: PASS — `state.move_object(object_id, Zone::Battlefield)` is the correct pattern for permanent spells (cf. Makeshift Mauler, Skaab Ruinator which use the same pattern).
- Ruling: "You must exile exactly one creature card": PASS — enforced as described above.
- Ruling: "Players can only respond once cast and costs paid": PASS — cost is paid in submit_action before the object moves to the stack; no engine hook between cost payment and stack placement exists.
- Creature type check covers both registry and object.power: PASS — eligibility filter uses `o.power.is_some() || registry.card_data(o.card_id).map(|d| d.card_types.contains(&CardType::Creature)).unwrap_or(false)`, covering both registry-backed cards and any object-level creature markers.
- Tokens cannot be exiled as cost (tokens cease to exist in graveyard): PASS — not an issue; tokens don't persist in the graveyard.
- Player choice of which creature to exile: FAIL — see Code Issues above.

### Test coverage

- Cast requiring exactly one creature card in graveyard: NOT TESTED
- Cast blocked when graveyard has zero creature cards: NOT TESTED
- Player cannot exile more than one creature: NOT TESTED
- Which creature is auto-selected when multiple are present: NOT TESTED
- Drake enters battlefield with correct P/T and Flying after resolving: NOT TESTED
- Ruling "must exile exactly one": NOT TESTED
- Ruling "cannot cast without exiling a creature card": NOT TESTED
