---
id: nevermore-02
status: new
card: Nevermore
card_file: mtg-engine/src/cards/isd/nevermore.rs
created: 2026-04-14T21:47:55Z
audit_run_id: 2026-04-14-nevermore-audit
audit_model: opus
audit_tokens: 8084
audit_duration: 1168
---

## Audit Finding

**Oracle text:**
> As this enchantment enters, choose a nonland card name.

**Code:**
> `nevermore.rs:49-57`: `let mut card_names: Vec<String> = registry.all_names().into_iter().filter(|name| { registry.get_id_by_name(name).and_then(|id| registry.card_data(id)).is_some_and(|d| !d.card_types.contains(&CardType::Land)) }).map(std::string::ToString::to_string).collect();`

**Description:**
Per MTG rules, when a card instructs you to "choose a card name," you may name any card that exists in the Oracle card database, not just cards in the current game or card pool. Nevermore's implementation restricts the choice to `registry.all_names()` — only cards implemented in the engine. A player cannot name a card that isn't in the registry, which means the restriction is artificially narrowed. In a real game, naming a card that isn't in the opponent's deck is a valid (if suboptimal) choice, and naming an unimplemented card that happens to be in the opponent's deck-building pool is impossible. This is an engine limitation rather than a card-specific bug, but it does cause Nevermore to behave differently from its Oracle text.

**Engine path:**
- nevermore.rs:49-57 (name list built from registry.all_names())
- engine.rs:527-530 (ChooseCardName presents options as indexed list)

**Required check:** Step 4

**Affected cards:**
- Nevermore
- Any future "choose a card name" card (e.g., Pithing Needle, Meddling Mage)

