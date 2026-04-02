# Audit: Trepanation Blade

## Scryfall Reference
- **Name:** Trepanation Blade
- **Cost:** {3}
- **Type:** Artifact — Equipment
- **Oracle:** Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard. Equip {2}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/trepanation_blade.rs`
- Name: "Trepanation Blade" -- MATCH
- Cost: {3} -- MATCH
- Types: Artifact -- MATCH
- Subtypes: ["Equipment"] -- MATCH
- Equip: {2} -- MATCH
- Trigger: Attacks (on equipped creature) -- MATCH
- is_equipment set on resolve -- MATCH
- Mill logic: reveals until land, counts all cards revealed (including land) -- MATCH
- Grants +1/+0 per card revealed until EOT -- MATCH

## Verdict
**PASS** — Equipment and mill trigger correctly implemented.

## Audit — 2026-04-01 15:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.
Equip {2}
**Type line**: Artifact — Equipment
**Mana cost**: {3}
**Rulings**:
- [2017-11-17] The land card is counted when calculating the bonus, and it will be put into the graveyard with the other revealed cards.
- [2017-11-17] If the equipped creature is attacking a planeswalker, the controller of the planeswalker is the defending player.
**Status**: PASS

### Code issues
No issues found.

### Detailed verification
1. **Name**: "Trepanation Blade" — correct.
2. **Mana cost {3}**: `Generic(3)` — correct.
3. **Card types [Artifact]**: correct.
4. **Subtypes ["Equipment"]**: correct.
5. **is_equipment set on resolve** (line 42): correct.
6. **Triggered ability declared**: `TriggerKind::Attacks` — correct. The engine fires this for equipment when the equipped creature attacks (verified in triggers.rs:688-694).
7. **on_attacks: self_id is the equipment** (line 59): correct. Looks up `attached_to` to find creature (line 63).
8. **Defending player from combat state** (lines 69-73): Uses `state.combat.as_ref().and_then(|c| c.attackers.get(&creature_id).copied())` — correctly identifies the defending player from actual combat data.
9. **Mill logic** (lines 78-101): Loops over library, removes from `library_order` and calls `move_object` to graveyard. Stops after revealing a land. Counts ALL cards including the land card. This matches the ruling: "The land card is counted when calculating the bonus."
10. **+1/+0 pump** (lines 109-118): Applies `cards_milled` as `power_mod` via `until_end_of_turn_effects`. Only applies if creature is still on the battlefield (line 109). Correct.
11. **Empty library handling** (line 81): If library is empty, loop breaks. No cards milled, no pump. Per oracle text "until they reveal a land card" — if no land is found, all cards are revealed and put into graveyard. Code handles this correctly: it continues until library is empty OR land is found.
12. **Equip ability** (lines 123-141): Sorcery speed, {2} cost, targets creature you control. Correct.
13. **Oracle text field** (line 27): Wording differs slightly from Scryfall oracle ("That player puts all cards revealed this way into their graveyard" vs "That player puts the revealed cards into their graveyard") and ordering differs (code has mill-then-pump in oracle_text, Scryfall has pump-then-mill), but the mechanical implementation matches the oracle correctly.

### Tricky interactions checked
- Land card counted in bonus: PASS (counter incremented before land check at line 96 vs 98)
- Empty library (no land found): PASS (all cards milled, loop exits when library empty)
- Creature removed before trigger resolves: PASS (line 109 checks creature still on battlefield)
- Equipment not attached: PASS (line 64 returns early if `attached_to` is None)

### Test coverage
- Card data (types, cost, subtypes): `tier9_cards.rs:290` (trepanation_blade_card_data)
- Mill and pump with multiple nonlands then land: `tier9_cards.rs:300` (trepanation_blade_attack_trigger_mills_and_pumps)
- Stops at first land: `tier9_cards.rs:348` (trepanation_blade_stops_at_first_land)
- Empty library (no lands): NOT TESTED
- Land card counted in bonus (ruling 1): PASS (tested implicitly in first test — 3 cards including land = +3/+0)
- Equipped creature removed mid-trigger: NOT TESTED

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard. / Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Card Data
- **Name:** Trepanation Blade -- CORRECT
- **Mana Cost:** {3} -- CORRECT
- **Type:** Artifact — Equipment -- CORRECT
- **P/T:** N/A -- CORRECT

### Code issues
1. **Oracle text wording mismatch**: The code oracle_text reorders the sentences and changes wording.
   - Oracle: "The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard."
   - Code: "That player puts all cards revealed this way into their graveyard. Equipped creature gets +1/+0 until end of turn for each card put into a graveyard this way."
2. **Behavior is functionally correct**: The land card IS counted in the +1/+0 bonus (matching the ruling), cards are milled to graveyard, equip cost is {2} at sorcery speed. No gameplay bug.
