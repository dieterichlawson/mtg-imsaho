# Audit: Bloodline Keeper // Lord of Lineage

## Oracle (Scryfall/API)
### Front: Bloodline Keeper
- **Name:** Bloodline Keeper
- **Cost:** {2}{B}{B}
- **Type:** Creature — Vampire
- **Oracle:** Flying. {T}: Create a 2/2 black Vampire creature token with flying. {B}: Transform Bloodline Keeper. Activate only if you control five or more Vampires.
- **P/T:** 3/3

### Back: Lord of Lineage
- **Type:** Creature — Vampire
- **Oracle:** Flying. Other Vampire creatures you control get +2/+2. {T}: Create a 2/2 black Vampire creature token with flying.
- **P/T:** 5/5

## Implementation: `mtg-engine/src/cards/bloodline_keeper.rs`
- **Name:** Bloodline Keeper -- CORRECT
- **Cost:** {2}{B}{B} -- CORRECT
- **Type:** Creature — Vampire -- CORRECT
- **P/T:** 3/3 front, 5/5 back -- CORRECT
- **Keywords:** Flying (both faces) -- CORRECT
- **Token creation:** 2/2 black Vampire with flying -- CORRECT
- **Transform condition:** 5+ Vampires, costs {B} -- CORRECT
- **Back face continuous effect:** ModifyPT +2/+2 for other Vampires you control -- CORRECT
- **Vampire counting:** Checks both object subtypes and registry subtypes -- CORRECT
- **DFC handling:** Uses `back_face_data`, `dynamic_pt`, `is_transformed` -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-01 14:00

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/90/bloodline-keeper-lord-of-lineage
**Oracle text (front)**:
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
**Oracle text (back)**:
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
**Type line (front)**: Creature — Vampire
**Type line (back)**: Creature — Vampire
**P/T (front)**: 3/3
**P/T (back)**: 5/5
**Mana cost**: {2}{B}{B}
**Status**: PASS

### Code issues
No issues found.

### Detailed verification

**Card data (front face)**:
- Name "Bloodline Keeper": correct
- Cost {2}{B}{B} (Generic(2), Black, Black): correct
- card_types [Creature]: correct
- supertypes []: correct (not legendary)
- subtypes ["Vampire"]: correct
- P/T 3/3: correct
- keywords [Flying]: correct
- oracle_text field: matches oracle

**Card data (back face)**:
- Name "Lord of Lineage": correct
- card_types [Creature]: correct
- subtypes ["Vampire"]: correct
- P/T 5/5 (via back_face_data and dynamic_pt): correct
- keywords [Flying]: correct
- continuous_effects ModifyPT +2/+2 with scope GlobalOther(You AND HasSubtype("Vampire")): correct — matches "Other Vampire creatures you control get +2/+2"
- Engine uses back_face_data() for continuous effects when is_transformed (state.rs lines 673-676): correct

**Activated abilities**:
- Ability 0: {T} create 2/2 Vampire token with flying — available on both faces: correct
- Ability 1: {B} transform — only on front face, requires 5+ Vampires: correct
- Token creation uses `create_token_with_subtypes` with colors [Black], types [Creature], keywords [Flying], subtypes ["Vampire"]: correct

**Vampire counting** (count_vampires method):
- Checks both `o.subtypes` (for tokens) and registry card_data subtypes: correct
- Includes self in count (Bloodline Keeper is a Vampire): correct per ruling

**Transform behavior**:
- Sets `is_transformed = true` and updates name: correct
- `should_transform` returns false (transform is activated, not automatic): correct

**Performance note** (not a correctness issue): `activated_abilities` calls `CardRegistry::with_all_cards()` on every invocation to count vampires, which is expensive. The registry should be passed as a parameter instead.

### Tricky interactions checked
- Self counts as Vampire for 5+ check: PASS
- Token Vampires count for 5+ check: PASS (checks o.subtypes)
- Transform is activated ability, not triggered: PASS
- Flying on both faces: PASS
- Back face +2/+2 doesn't buff self (GlobalOther): PASS
- Transform doesn't untap: PASS (code only sets is_transformed and name)

### Test coverage
- Token creation (2/2 Vampire): `tier15_cards.rs:857` — TESTED (but doesn't verify Flying keyword on token)
- Transform with 5+ Vampires: NOT TESTED
- Lord of Lineage +2/+2 buff to other Vampires: NOT TESTED
- Token creation on back face: NOT TESTED
- Vampire counting includes tokens: NOT TESTED
- Vampire counting includes self: NOT TESTED
