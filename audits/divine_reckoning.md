# Audit: Divine Reckoning

## Reference (Scryfall)
- **Name:** Divine Reckoning
- **Cost:** {2}{W}{W}
- **Type:** Sorcery
- **Oracle:** Each player chooses a creature they control. Destroy the rest. Flashback {5}{W}{W}
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{W}{W})
- Type: CORRECT (Sorcery)
- Oracle text: CORRECT (says "destroys" matching Scryfall "Destroy the rest")
- Flashback cost: CORRECT ({5}{W}{W})
- Each player keeps one creature: CORRECT
- Destroys the rest (not sacrifice): CORRECT (uses try_destroy)
- P/T: CORRECT (N/A)

## Issues
None found.

---

## Audit (2026-04-02)

### Oracle Text (Scryfall)
```
Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

### Card Data

- **Mana cost**: {2}{W}{W} — CORRECT
- **Type**: Sorcery — CORRECT
- **Flashback cost**: {5}{W}{W} — CORRECT

### Issues Found

#### 1. Oracle text string mismatch (minor)
- **Oracle (Scryfall)**: `"Each player chooses a creature they control. Destroy the rest."`
- **Implementation oracle_text field (line 26)**: `"Each player chooses a creature they control, then destroys the rest."`
- The wording differs: oracle uses period-separated sentences; implementation uses comma with "then destroys". Should match Scryfall verbatim.

#### 2. Doc comment says "sacrifices" instead of "destroys" (minor)
- **Doc comment (line 8)**: `"Each player chooses a creature they control, then sacrifices the rest."`
- **Oracle**: The card destroys, it does not sacrifice. The doc comment is misleading.

#### 3. Player choice not presented — auto-selects highest toughness (major)
- **Oracle**: "Each player chooses a creature they control." — this is a player choice.
- **Implementation (lines 40-41)**: `// For each player, auto-keep the creature with the highest toughness. // (Simplification: in real MTG, each player would choose.)`
- The code automatically selects the creature with the highest toughness instead of allowing each player to choose. This is acknowledged in a comment as a simplification but is a gameplay-affecting deviation.

#### 4. Turn order for choices not respected
- **Ruling (2011-09-22)**: "Starting with the player whose turn it is, each player chooses a creature in turn order. Players will know the choice of each player who chose before them."
- **Implementation**: Iterates `state.players` in list order, which may or may not match turn order. Since the choice is auto-selected (issue 3), this is moot until issue 3 is fixed, but should be noted for completeness.

#### 5. Player with no creatures — correct
- The code correctly skips players with 0 or 1 creatures (lines 53-56). A player with no creatures simply has nothing to choose and nothing is destroyed.

#### 6. Destruction method — correct
- Uses `try_destroy` (line 74), which respects indestructible. This is correct — the oracle says "Destroy", not "sacrifice".

#### 7. Flashback — correct
- `flashback_cost` is set to `{5}{W}{W}` (mana value 7). Correct.

### Tests

- `divine_reckoning_keeps_one_per_player` — validates the auto-selection behavior but not true player choice.
- `divine_reckoning_with_one_creature_keeps_it` — correctly tests the edge case of a player with only one creature.
- `divine_reckoning_has_flashback` — validates flashback cost is present and equals mana value 7. Correct.

### Verdict

**Minor issues**: Oracle text string does not match Scryfall verbatim (line 26); doc comment incorrectly says "sacrifices" (line 8).
**Major issue**: Player choice is replaced by automatic highest-toughness selection. This is a known simplification (commented in code) but deviates from the rules.

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Each player chooses a creature they control. Destroy the rest.
Flashback {5}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

1. **Oracle text string mismatch (minor, unchanged from prior audit).**
   - Oracle: `"Each player chooses a creature they control. Destroy the rest."`
   - Code (line 26): `"Each player chooses a creature they control, then destroys the rest."`
   The wording differs: oracle uses two sentences; implementation uses comma-delimited phrasing.

2. **Doc comment says "sacrifices" (minor, unchanged from prior audit).**
   - Code (line 8): `"Each player chooses a creature they control, then sacrifices the rest."`
   - Oracle says "Destroy", not "sacrifice". These are mechanically different (sacrifice bypasses indestructible; destroy does not).

3. **Player choice auto-selected (major, unchanged from prior audit).**
   - Oracle: "Each player chooses a creature they control."
   - Code (lines 40-41): `// For each player, auto-keep the creature with the highest toughness. // (Simplification: in real MTG, each player would choose.)`
   - The code automatically selects the creature with the highest toughness instead of presenting a choice to each player.

4. **Destruction method is correct.** Uses `try_destroy` (line 74), which respects indestructible. Matches "Destroy".

5. **Flashback cost correct.** `{5}{W}{W}` (lines 28-31).

6. **Mana cost correct.** `{2}{W}{W}` (lines 16-20).

### Tricky interactions checked
- Players with 0 or 1 creatures: correctly skipped (lines 53-56).
- Indestructible creatures: `try_destroy` handles this correctly.
- Turn order for choices: not respected, but moot since choices are auto-selected.

### Test coverage
- `divine_reckoning_keeps_one_per_player` -- validates auto-selection behavior.
- `divine_reckoning_with_one_creature_keeps_it` -- edge case of single creature.
- `divine_reckoning_has_flashback` -- validates flashback cost.
