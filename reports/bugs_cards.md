# Card Implementation Bugs Report (2026-03-29)

## Critical

### 1. Falkenrath Noble -- Triggers on Wrong Creatures
**File:** `cards/falkenrath_noble.rs`
Triggers on ANY creature death. Should only trigger on creatures you control dying. Also targets opponent directly instead of "each opponent loses 1 life."

## High

### 2. Spectral Flight -- Missing +2/+2
**File:** `cards/spectral_flight.rs`
Only grants Flying keyword. Never applies the +2/+2 power/toughness boost from "Enchanted creature gets +2/+2 and has flying."

### 3. Furor of the Bitten -- Missing +2/+2 and Attack Requirement
**File:** `cards/furor_of_the_bitten.rs`
Neither the +2/+2 boost nor the "attacks each combat if able" mechanic is implemented.

### 4. Bonds of Faith -- +2/+2 Not Applied
**File:** `cards/bonds_of_faith.rs`
The +2/+2 bonus to Humans is never actually applied to the creature's power/toughness stats.

## Low

### 5. Claustrophobia -- Oracle Text Incomplete
**File:** `cards/claustrophobia.rs`
Oracle text field missing "When Claustrophobia enters the battlefield, tap enchanted creature." The code does tap, but the text doesn't say so.

### 6. Moan of the Unhallowed -- Inconsistent Cleanup Method
**File:** `cards/moan_of_the_unhallowed.rs`
Uses `move_spell_after_resolve()` while similar sorceries use `move_object()`. Minor inconsistency.
