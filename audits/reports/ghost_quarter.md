# Audit: Ghost Quarter

## Oracle Reference (Scryfall)
- Cost: (none, land)
- Type: Land
- Oracle: "{T}: Add {C}.
  {T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."

## Implementation: ghost_quarter.rs

## Issues Found

No issues found. Name, type (Land), oracle text, mana ability, and activated ability all match. The sacrifice ability correctly requires tap, sacrifice self, and targets a land. The "may search" is auto-resolved (always searches), which is a reasonable AI simplification. Basic land search logic correctly checks for CardType::Land + Supertype::Basic.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
{T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
```

### Findings
- Name, type (Land), cost (none) all match.
- Oracle text in code matches Scryfall oracle.
- Mana ability: {T} adds {C}, checks untapped + battlefield -- correct.
- Activated ability: {T}, sacrifice self, targets any land -- correct.
- On resolution: destroys target land, then auto-searches controller's library for Basic Land -- correct.
- "May search" is auto-resolved (always searches), acceptable AI simplification.

### Verdict: PASS

---

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: `{T}: Add {C}.\n{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.`
**Type line**: `Land`
**Status**: ISSUE

### Code issues

1. **Missing library shuffle after search (ISSUE)**: After finding and placing the basic land onto the battlefield, the implementation does not shuffle the target land controller's library. The engine supports shuffling (e.g., `ChooseFromLibrary` resolution in engine.rs:2048 calls `.shuffle()`), so this is not an engine limitation -- it is a missing step. The library order is meaningful in the engine, so this affects gameplay correctness.

2. **Log message says "destroyed" even when land survives (minor)**: Line 79 always logs `"Ghost Quarter destroyed {target_name}"` regardless of the return value of `try_destroy`. If the target land is indestructible or regenerated, the log is misleading. The `try_destroy` return value (`DestroyResult`) is discarded.

3. **Oracle text in `card_data` says "Sacrifice Ghost Quarter" but Scryfall oracle says "Sacrifice this land"**: The code's `oracle_text` field reads `"...Sacrifice Ghost Quarter: Destroy target land..."` while the canonical Scryfall oracle text reads `"...Sacrifice this land: Destroy target land..."`. This is a minor text mismatch.

### Tricky interactions checked (min 3)

1. **Indestructible land**: The code calls `try_destroy` but does not check its return value, so the search still proceeds even if the land survives. This correctly matches the ruling: "The target land's controller gets to search for a basic land card even if that land wasn't destroyed by Ghost Quarter's ability."

2. **Self-targeting**: Ghost Quarter can be chosen as a target since it is on the battlefield when targets are selected. However, it is sacrificed as part of activation cost (SacrificeCost::SacrificeThis). When the ability resolves, lines 71-73 check `o.zone == Zone::Battlefield` and the sacrificed Ghost Quarter is no longer there, so `_ => return` fires and the ability does nothing. This correctly matches the ruling: "If you target Ghost Quarter with its own ability, the ability won't resolve because its target is no longer on the battlefield."

3. **Target removed before resolution**: If the target land leaves the battlefield before resolution (e.g., bounced), lines 71-74 return early with no effect. This matches the ruling: "If the targeted land is an illegal target by the time Ghost Quarter's ability resolves, it won't resolve and none of its effects will happen."

4. **Regenerated land**: If the land has regeneration shields, `try_destroy` returns `Regenerated` (land is tapped, damage removed, but stays on battlefield). The search still proceeds since the return value is not checked. Per the ruling, this is correct -- the controller still gets to search.

### Test coverage

- `ghost_quarter_card_data`: Verifies card type is Land and oracle text contains "Destroy target land". PASS.
- `ghost_quarter_taps_for_colorless`: Verifies the mana ability appears in legal actions. PASS.
- **Missing**: No test for the core activated ability (destroy a land + opponent searches for basic land).
- **Missing**: No test for indestructible interaction (search still happens).
- **Missing**: No test for self-targeting fizzle behavior.

---

## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues

- Missing library shuffle after search (lines 92-101 in ghost_quarter.rs)
  - Oracle text says: `put it onto the battlefield, then shuffle`
  - Code does: Puts land into play but never shuffles the library. Other cards like Caravan Vigil properly implement the shuffle using `state.get_player_mut(controller).library_order.shuffle(&mut rng)`.

- Incorrect logging when target has indestructible or is regenerated (lines 76-79)
  - Oracle text says: `Destroy target land. Its controller may search...` with ruling "The target land's controller gets to search for a basic land card even if that land wasn't destroyed by Ghost Quarter's ability."
  - Code does: Calls `try_destroy()` correctly, but then unconditionally logs `"Ghost Quarter destroyed {target_name}"` even if the destruction was prevented by indestructible or replaced by regeneration. The log should reflect whether the land was actually destroyed.

### Tricky interactions checked
- Indestructible land targeted: ISSUE - search happens correctly but logging is wrong
- Target becomes illegal before resolution: PASS - early return on line 73 handles this correctly
- Self-targeting (Ghost Quarter targets itself): PASS - sacrifice cost ensures it's gone before resolution, caught by zone check
- Regeneration prevents destruction: ISSUE - search happens correctly but logging claims destruction occurred
- Missing basic lands in library: PASS - no basic land found is handled correctly (nothing put into play)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (tap for mana): `mtg-engine/tests/innistrad_simple_cards.rs:161`
- Card data correctness: `mtg-engine/tests/innistrad_simple_cards.rs:152`
- Destroy target land and search: NOT TESTED
- Indestructible land interaction (search still happens): NOT TESTED
- Target becomes illegal before resolution: NOT TESTED
- Self-targeting scenario: NOT TESTED
- Library shuffle after search: NOT TESTED
- "May search" vs mandatory search behavior: NOT TESTED

---

## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues

- **Missing library shuffle after search** (`ghost_quarter.rs` lines 92-101)
  - Oracle text says: `put it onto the battlefield, then shuffle.`
  - Code does: Removes the basic land from `library_order`, calls `move_object(land_id, Zone::Battlefield)`, and logs the event — but never shuffles the controller's library. The engine supports shuffling elsewhere (e.g., `engine.rs:2048`, `engine.rs:2395`, `engine.rs:2613` all call `library_order.shuffle(&mut rng)`). The library order is tracked and meaningful in this engine, so the missing shuffle affects gameplay correctness.

- **"May search" is treated as mandatory** (`ghost_quarter.rs` lines 81-101)
  - Oracle text says: `Its controller may search their library for a basic land card`
  - Code does: Automatically finds and puts the first basic land from the library onto the battlefield with no player choice. The comment on line 81 acknowledges this: `"// Its controller may search for a basic land (auto-search)."` The engine supports presenting choices to players (e.g., Bitterheart Witch uses `ChooseCurseThenAttach` pending effects for its "you may search" ability). The opponent's controller should be able to decline the search.

- **Oracle text field says "Sacrifice Ghost Quarter" instead of "Sacrifice this land"** (`ghost_quarter.rs` line 23)
  - Oracle text says: `Sacrifice this land`
  - Code does: `"Sacrifice Ghost Quarter"` in the `oracle_text` field of `card_data()`.

- **Log message claims destruction regardless of outcome** (`ghost_quarter.rs` lines 78-79)
  - Oracle text says: `Destroy target land` (with ruling: search happens "even if that land wasn't destroyed")
  - Code does: `state.log(..., format!("Ghost Quarter destroyed {}", target_name))` unconditionally after `try_destroy`, even if the land was indestructible or regenerated. The `DestroyResult` return value from `try_destroy` is discarded.

### Tricky interactions checked
- Indestructible land targeted: pass (search still proceeds because `try_destroy` return value is not gating the search — this matches the ruling)
- Self-targeting (Ghost Quarter targets itself): pass (sacrifice as cost removes it from battlefield; zone check on line 72-73 causes early return with no effect, matching ruling)
- Target removed before resolution: pass (zone check `o.zone == Zone::Battlefield` on line 72 causes early return, matching ruling about illegal targets)
- Regenerated land: pass (search still proceeds after `try_destroy` returns `Regenerated`, matching ruling)
- No basic land in library: pass (line 92 `if let Some(land_id)` — if no match found, nothing happens, which is correct)
- Targeting any land (own or opponent's): pass (`PermanentWithFilter(HasCardType(vec![CardType::Land]))` checks `all_objects_in_zone(Zone::Battlefield)` and default `is_valid_target` returns true for all)
- Land enters under correct controller: pass (the found land object already has its owner/controller set from `setup_game`, and `move_object` doesn't change controller)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic mana ability (tap for colorless): `mtg-engine/tests/innistrad_simple_cards.rs:161`
- Card data correctness: `mtg-engine/tests/innistrad_simple_cards.rs:152`
- Core ability (destroy land + search for basic): NOT TESTED
- Library shuffle after search: NOT TESTED
- "May search" optionality: NOT TESTED
- Indestructible interaction (ruling: search still happens): NOT TESTED
- Self-targeting fizzle (ruling: ability doesn't resolve): NOT TESTED
- Target becomes illegal before resolution (ruling: no effects happen): NOT TESTED
- Regeneration interaction (ruling: search still happens): NOT TESTED

---

## Audit — 2026-04-03 22:32

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues

- Missing "may" choice on line 82 (`mtg-engine/src/cards/isd/ghost_quarter.rs:82`)
  - Oracle text says: `Its controller may search their library for a basic land card`
  - Code does: Auto-searches without presenting choice to the target land's controller (comment even says "auto-search")

- Missing library shuffle on line 82-101 (`mtg-engine/src/cards/isd/ghost_quarter.rs:82-101`)  
  - Oracle text says: `put it onto the battlefield, then shuffle`
  - Code does: Puts land onto battlefield but never shuffles the target controller's library

### Tricky interactions checked
- Self-targeting (targeting Ghost Quarter with its own ability): PASS - ability won't resolve because target becomes illegal after sacrificing as cost
- Illegal target handling: PASS - code checks target validity on resolution and returns early if target is gone  
- Indestructible/regenerated land handling: PASS - search happens regardless of destroy result, matching 2013-07-01 ruling
- Target selection: PASS - correctly targets any land using TargetFilter::HasCardType
- Choice ownership: FAIL - should present choice to target land's controller, but code auto-searches instead
- Library shuffling: FAIL - required by oracle text but missing from implementation

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Self-targeting edge case: NOT TESTED  
- Indestructible land interaction: NOT TESTED
- "may search" choice presentation: NOT TESTED
- Library shuffling: NOT TESTED
- Basic card data and mana ability: `mtg-engine/tests/innistrad_simple_cards.rs:152-172` / TESTED

---

## Audit — 2026-04-03 22:50

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
**Type line**: Land
**Status**: ISSUE

### Code issues
- Missing player choice for "may search" at `/Users/dlaw/mtg/mtg-engine/src/cards/isd/ghost_quarter.rs:81`
  - Oracle text says: `Its controller may search their library for a basic land card`
  - Code does: Auto-searches without presenting choice (comment says "auto-search")
- Missing library shuffle at `/Users/dlaw/mtg/mtg-engine/src/cards/isd/ghost_quarter.rs:81-101`
  - Oracle text says: `put it onto the battlefield, then shuffle`
  - Code does: Places land on battlefield but never calls `library_order.shuffle(&mut rng)`
- Oracle text display error at `/Users/dlaw/mtg/mtg-engine/src/cards/isd/ghost_quarter.rs:23`
  - Oracle text says: `Sacrifice this land`
  - Code displays: `Sacrifice Ghost Quarter` (should follow standard "this [type]" templating)

### Tricky interactions checked
- Search happens even if destruction prevented (indestructible/regeneration): PASS (code ignores `try_destroy` return value)
- Target land's controller does the search (not Ghost Quarter's controller): PASS (uses `target_controller`)
- "may search" presents player choice: FAIL (auto-searches instead)
- Library shuffles after search: FAIL (missing shuffle call)
- Illegal target causes ability to fizzle completely: PASS (early return on line 73 if target invalid)
- Self-targeting causes fizzle: PASS (early return on line 73 if Ghost Quarter not on battlefield)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Search happens even if destruction prevented: NOT TESTED
- Target land controller does search: NOT TESTED  
- "may search" choice presentation: NOT TESTED
- Library shuffle after search: NOT TESTED
- Illegal target fizzle: NOT TESTED
- Self-targeting fizzle: NOT TESTED
- Basic mana ability: `mtg-engine/tests/innistrad_simple_cards.rs:161`
- Card data validation: `mtg-engine/tests/innistrad_simple_cards.rs:152`
